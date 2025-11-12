use super::extensions;
use super::return_types::*;
use crate::event_sourcing;
use event_sourcing::event::{AggregateType, DomainEvent, EventType};
use event_sourcing::journal::{
    BalanceUpdate, JournalEvent, JournalState, Permissions, Transaction,
};
use event_sourcing::user;
use event_sourcing::user::{UserEvent, UserState};
use leptos::prelude::*;
use uuid::Uuid;

#[server]
pub async fn create_account(
    username: String,
    password: String,
    confirm_password: String,
) -> Result<(), ServerFnError> {
    let pool = extensions::get_pool().await?;
    let session_id = extensions::get_session_id().await?;

    if password != confirm_password {
        return Err(ServerFnError::ServerError(
            KnownErrors::SignupPasswordMismatch { username }.to_string(),
        ));
    }

    if user::get_id_from_username(&username, &pool).await? == None {
        let uuid = Uuid::new_v4();
        UserEvent::Created {
            username,
            hashed_password: bcrypt::hash(password, bcrypt::DEFAULT_COST)?,
        }
        .push_db(&uuid, &pool)
        .await?;

        UserEvent::LoggedIn { session_id }.push_db(&uuid, &pool);
    } else {
        return Err(ServerFnError::ServerError(
            KnownErrors::UserExists { username }.to_string(),
        ));
    }

    Ok(())
}

#[server]
pub async fn login(username: String, password: String) -> Result<(), ServerFnError> {
    let session_id = extensions::get_session_id().await?;
    let pool = extensions::get_pool().await?;

    let user_id = match user::get_id_from_username(&username, &pool).await? {
        Some(s) => s,
        None => {
            return Err(ServerFnError::ServerError(
                KnownErrors::UserDoesntExist.to_string(),
            ))
        }
    };

    let hashed_password = user::get_hashed_pw(&user_id, &pool).await?;

    if bcrypt::verify(&password, &hashed_password)? {
        UserEvent::LoggedIn { session_id }.push_db(&user_id, &pool);
    } else {
        return Err(ServerFnError::ServerError(
            KnownErrors::LoginFailed { username, password }.to_string(),
        ));
    }

    Ok(())
}

#[server]
pub async fn is_logged_in() -> Result<(), ServerFnError> {
    let session_id = extensions::get_session_id().await?;
    let pool = extensions::get_pool().await?;

    user::get_id_from_session(&session_id, &pool).await?; // this will return an KnownErrors::NotLoggedIn if the user isn't logged in

    Ok(())
}

#[server]
pub async fn log_out() -> Result<(), ServerFnError> {
    let session_id = extensions::get_session_id().await?;
    let pool = extensions::get_pool().await?;

    if let Ok(uuid) = user::get_id_from_session(&session_id, &pool).await {
        UserEvent::LoggedOut { session_id }
            .push_db(&uuid, &pool)
            .await?;
    }

    Ok(())
}

#[server]
pub async fn create_journal(journal_name: String) -> Result<(), ServerFnError> {
    let pool = extensions::get_pool().await?;
    let session_id = extensions::get_session_id().await?;
    let user_id = user::get_id_from_session(&session_id, &pool).await?;

    JournalEvent::Created {
        name: journal_name,
        owner: user_id,
    }
    .push_db(&Uuid::new_v4(), &pool)
    .await?;

    Ok(())
}

#[server]
pub async fn invite_to_journal(
    journal_id: Uuid,
    invitee_username: String,
    permissions: Permissions,
) -> Result<(), ServerFnError> {
    let pool = extensions::get_pool().await?;
    let session_id = extensions::get_session_id().await?;

    let own_id = user::get_id_from_session(&session_id, &pool).await?;
    if let Some(invitee_id) = user::get_id_from_username(&invitee_username, &pool).await? {
        let inviting_user_state = UserState::build(
            &own_id,
            vec![
                EventType::UserCreatedJournal,
                EventType::UserInvitedToJournal,
                EventType::UserAcceptedJournalInvite,
                EventType::UserDeclinedJournalInvite,
                EventType::UserRemovedFromJournal,
            ],
            &pool,
        )
        .await?;

        if inviting_user_state.owned_journals.contains(&journal_id) {
            UserEvent::InvitedToJournal {
                id: journal_id,
                permissions,
                inviting_user: own_id,
                owner: own_id,
            }
            .push_db(&invitee_id, &pool);
        } else if let Some(own_tenant_info) = inviting_user_state
            .accepted_journal_invites
            .get(&journal_id)
        {
            for permission in permissions {
                if !own_tenant_info.tenant_permissions.contains(permission) {
                    return Err(ServerFnError::ServerError(
                        KnownErrors::PermissionError {
                            required_permissions: permission,
                        }
                        .to_string(),
                    ));
                }
            }
            UserEvent::InvitedToJournal {
                id: journal_id,
                permissions,
                inviting_user: own_id,
                owner: own_tenant_info.journal_owner,
            }
            .push_db(&invitee_id, &pool)
            .await?;
        }
        Ok(())
    } else {
        Err(ServerFnError::ServerError(
            KnownErrors::UserDoesntExist.to_string(),
        ))
    }
}

#[server]
pub async fn accept_journal_invite(journal_id: Uuid) -> Result<(), ServerFnError> {
    use EventType::*;

    let pool = extensions::get_pool().await?;
    let session_id = extensions::get_session_id().await?;

    let user_id = user::get_id_from_session(&session_id, &pool).await?;

    let user_state = UserState::build(
        &user_id,
        vec![
            UserInvitedToJournal,
            UserAcceptedJournalInvite,
            UserDeclinedJournalInvite,
            UserRemovedFromJournal,
        ],
        &pool,
    )
    .await?;

    if user_state.pending_journal_invites.contains_key(&journal_id) {
        UserEvent::AcceptedJournalInvite { id: journal_id }.push_db(&user_id, &pool);
    } else {
        return Err(ServerFnError::ServerError(
            KnownErrors::NoInvitation.to_string(),
        ));
    }

    Ok(())
}

#[server]
pub async fn decline_journal_invite(journal_id: Uuid) -> Result<(), ServerFnError> {
    use EventType::*;

    let pool = extensions::get_pool().await?;
    let session_id = extensions::get_session_id().await?;

    let user_id = user::get_id_from_session(&session_id, &pool).await?;

    let user_state = UserState::build(
        &user_id,
        vec![
            UserInvitedToJournal,
            UserAcceptedJournalInvite,
            UserDeclinedJournalInvite,
            UserRemovedFromJournal,
        ],
        &pool,
    )
    .await?;

    if user_state.pending_journal_invites.contains_key(&journal_id) {
        UserEvent::DeclinedJournalInvite { id: journal_id }.push_db(&user_id, &pool);
    } else {
        return Err(ServerFnError::ServerError(
            KnownErrors::NoInvitation.to_string(),
        ));
    }
    Ok(())
}

#[server]
pub async fn get_associated_journals(
    user_id: Uuid,
) -> Result<Vec<AssociatedJournal>, ServerFnError> {
    use EventType::*;
    let mut journals = Vec::new();

    let pool = extensions::get_pool().await?;

    let user = UserState::build(
        &user_id,
        vec![
            UserCreatedJournal,
            UserInvitedToJournal,
            UserAcceptedJournalInvite,
            UserDeclinedJournalInvite,
            UserRemovedFromJournal,
        ],
        &pool,
    )
    .await?;

    for journal_id in user.owned_journals {
        let journal_state =
            JournalState::build(&journal_id, vec![JournalCreated, JournalDeleted], &pool).await?;
        if !journal_state.deleted {
            journals.push(AssociatedJournal::Owned {
                id: journal_id,
                name: journal_state.name,
            });
        }
    }

    for shared_journal in user.accepted_journal_invites {
        let journal_state = JournalState::build(
            &shared_journal.0,
            vec![JournalCreated, JournalDeleted],
            &pool,
        )
        .await?;
        if !journal_state.deleted {
            journals.push(AssociatedJournal::Shared {
                id: shared_journal.0,
                name: journal_state.name,
                tenant_info: shared_journal.1,
            });
        }
    }

    Ok(journals)
}

#[server]
pub async fn get_accounts(journal_id: Uuid) -> Result<Vec<Account>, ServerFnError> {
    use EventType::*;

    let mut accounts = Vec::new();

    let pool = extensions::get_pool().await?;
    let session_id = extensions::get_session_id().await?;
    let user_id = user::get_id_from_session(&session_id, &pool).await?;

    let user_state = UserState::build(
        &user_id,
        vec![
            UserCreatedJournal,
            UserInvitedToJournal,
            UserAcceptedJournalInvite,
            UserDeclinedJournalInvite,
            UserRemovedFromJournal,
        ],
        &pool,
    )
    .await?;

    if !user_state.owned_journals.contains(&journal_id) {
        let journal_perms = user_state.accepted_journal_invites.get(&journal_id);

        if journal_perms.is_none()
            || journal_perms
                .unwrap()
                .tenant_permissions
                .contains(Permissions::READ)
        {
            return Err(ServerFnError::ServerError(
                KnownErrors::PermissionError {
                    required_permissions: Permissions::READ,
                }
                .to_string(),
            ));
        }
    }

    let journal_state = JournalState::build(
        &journal_id,
        vec![
            JournalCreated,
            JournalAccountCreated,
            JournalAccountDeleted,
            JournalAddedEntry,
        ],
        &pool,
    )
    .await?;

    for account in journal_state.accounts {
        accounts.push(Account {
            name: account.0,
            balance: account.1,
        });
    }

    Ok(accounts)
}

#[server]
pub async fn add_account(account_name: String, journal_id: Uuid) -> Result<(), ServerFnError> {
    use EventType::*;

    let pool = extensions::get_pool().await?;

    let session_id = extensions::get_session_id().await?;

    let user_id = user::get_id_from_session(&session_id, &pool).await?;

    let user_state = user::UserState::build(
        &user_id,
        vec![
            UserCreatedJournal,
            UserInvitedToJournal,
            UserAcceptedJournalInvite,
            UserDeclinedJournalInvite,
            UserRemovedFromJournal,
        ],
        &pool,
    )
    .await?;

    if user_state.owned_journals.contains(&journal_id) {
        JournalEvent::CreatedAccount { account_name }
            .push_db(&journal_id, &pool)
            .await?;
    } else if let Some(tenant_info) = user_state.accepted_journal_invites.get(&journal_id) {
        if tenant_info
            .tenant_permissions
            .contains(Permissions::ADDACCOUNT)
        {
            return Err(ServerFnError::ServerError(
                KnownErrors::PermissionError {
                    required_permissions: Permissions::ADDACCOUNT,
                }
                .to_string(),
            ));
        }
    }

    Ok(())
}

#[server]
pub async fn transact(
    journal_id: Uuid,
    account_names: Vec<String>,
    balance_add_cents: Vec<String>,
    balance_remove_cents: Vec<String>,
) -> Result<(), ServerFnError> {
    let pool = extensions::get_pool().await?;
    let session_id = extensions::get_session_id().await?;

    let account_id = user::get_id_from_session(&session_id, &pool).await?;

    let user_state = UserState::build(
        &account_id,
        vec![
            EventType::UserCreatedJournal,
            EventType::UserInvitedToJournal,
            EventType::UserAcceptedJournalInvite,
            EventType::UserDeclinedJournalInvite,
            EventType::UserRemovedFromJournal,
        ],
        &pool,
    )
    .await?;

    if !user_state.owned_journals.contains(&journal_id) {
        if let Some(tenant_info) = user_state.accepted_journal_invites.get(&journal_id) {
            if !tenant_info
                .tenant_permissions
                .contains(Permissions::APPENDTRANSACTION)
            {
                return Err(ServerFnError::ServerError(
                    KnownErrors::PermissionError {
                        required_permissions: Permissions::APPENDTRANSACTION,
                    }
                    .to_string(),
                ));
            }
        } else {
            return Err(ServerFnError::ServerError(
                KnownErrors::PermissionError {
                    required_permissions: Permissions::APPENDTRANSACTION,
                }
                .to_string(),
            ));
        }
    }

    let mut updates: Vec<BalanceUpdate> = Vec::new();
    let mut total_balance_change: i64 = 0;

    for i in 0..balance_add_cents.len() {
        let add_amt = match balance_add_cents[i].parse::<i64>() {
            Ok(s) => s,
            Err(_) => {
                return Err(ServerFnError::ServerError(
                    KnownErrors::InvalidInput.to_string(),
                ))
            }
        };

        let remove_amt = match balance_remove_cents[i].parse::<i64>() {
            Ok(s) => s,
            Err(_) => {
                return Err(ServerFnError::ServerError(
                    KnownErrors::InvalidInput.to_string(),
                ))
            }
        };

        let account_sum = add_amt - remove_amt;

        if account_sum != 0 {
            total_balance_change += account_sum;
            updates.push(BalanceUpdate {
                account_name: account_names[i].clone(),
                changed_by: account_sum,
            });
        }
    }

    if total_balance_change != 0 {
        return Err(ServerFnError::ServerError(
            KnownErrors::BalanceMismatch {
                attempted_transaction: updates,
            }
            .to_string(),
        ));
    }

    JournalEvent::AddedEntry {
        transaction: Transaction {
            author: account_id,
            updates,
        },
    }
    .push_db(&journal_id, &pool);

    Ok(())
}

pub async fn get_transactions(
    journal_id: &Uuid,
) -> Result<Vec<TransactionWithTimeStamp>, ServerFnError> {
    let mut bundled_transactions = Vec::new();

    let pool = extensions::get_pool().await?;

    let raw_transactions: Vec<(sqlx::types::JsonValue, i64)> = sqlx::query_as(
        r#"
        SELECT payload, created_at FROM events
        WHERE aggregate_id = $1 AND aggregate_type = $2 AND event_type = $3
        SORT BY created_at ASC
        "#,
    )
    .bind(journal_id)
    .bind(AggregateType::Journal)
    .bind(EventType::JournalAddedEntry)
    .fetch_all(&pool)
    .await?;

    for raw_transaction in raw_transactions {
        let domain_event: DomainEvent = serde_json::from_value(raw_transaction.0)?;
        if let JournalEvent::AddedEntry { transaction } = domain_event.to_journal_event()? {
            bundled_transactions.push(TransactionWithTimeStamp {
                transaction,
                timestamp: raw_transaction.1,
            })
        }
    }
    Ok(bundled_transactions)
}
