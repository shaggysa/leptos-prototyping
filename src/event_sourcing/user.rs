use bcrypt::verify;
use leptos::prelude::ServerFnError;
use serde::{Deserialize, Serialize};
use sqlx::{query_scalar, types::JsonValue, PgPool};
use std::collections::HashSet;
use uuid::Uuid;

use super::event::{AggregateType, DomainEvent, EventType};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum UserEvent {
    Created {
        username: String,
        hashed_password: String,
    },
    UsernameUpdated {
        username: String,
    },
    PasswordUpdated {
        hashed_password: String,
    },
    LoggedIn {
        session_id: String,
    },
    LoggedOut {
        session_id: String,
    },
    CreatedJournal {
        id: Uuid,
    },
    InvitedToJournal {
        id: Uuid,
    },
    AcceptedJournalInvite {
        id: Uuid,
    },
    DeclinedJournalInvite {
        id: Uuid,
    },
    RemovedFromJournal {
        id: Uuid,
    },
    Deleted,
}

impl UserEvent {
    pub async fn push_db(&self, uuid: Uuid, pool: &PgPool) -> Result<i64, ServerFnError> {
        let payload = match serde_json::to_value(self) {
            Ok(s) => s,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };
        match sqlx::query_scalar(
            r#"
            INSERT INTO events (
                aggregate_id,
                aggregate_type,
                event_type,
                payload
            )
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(uuid)
        .bind(AggregateType::User)
        .bind(EventType::from_user_event(self))
        .bind(payload)
        .fetch_one(pool)
        .await
        {
            Ok(s) => Ok(s),
            Err(e) => Err(ServerFnError::ServerError(e.to_string())),
        }
    }
}

#[derive(Default)]
pub struct UserState {
    pub id: Uuid,
    pub authenticated_sessions: std::collections::HashSet<String>,
    pub username: String,
    pub hashed_password: String,
    pub pending_journal_invites: HashSet<Uuid>,
    pub accepted_journal_invites: HashSet<Uuid>,
    pub owned_journals: HashSet<Uuid>,
    pub deleted: bool,
}

impl UserState {
    pub async fn from_events(id: Uuid, events: Vec<UserEvent>) -> Self {
        let mut aggregate = Self {
            id,
            ..Default::default()
        };

        for event in events {
            aggregate.apply(event).await;
        }
        aggregate
    }

    pub async fn apply(&mut self, event: UserEvent) {
        match event {
            UserEvent::Created {
                username,
                hashed_password: password,
            } => {
                self.username = username;
                self.hashed_password = password;
            }
            UserEvent::UsernameUpdated { username } => self.username = username,
            UserEvent::PasswordUpdated {
                hashed_password: password,
            } => self.hashed_password = password,
            UserEvent::LoggedIn { session_id } => {
                _ = self.authenticated_sessions.insert(session_id)
            }
            UserEvent::LoggedOut { session_id } => {
                _ = self.authenticated_sessions.remove(&session_id)
            }
            UserEvent::CreatedJournal { id } => _ = self.owned_journals.insert(id),
            UserEvent::InvitedToJournal { id } => _ = self.pending_journal_invites.insert(id),
            UserEvent::DeclinedJournalInvite { id } => _ = self.pending_journal_invites.remove(&id),
            UserEvent::AcceptedJournalInvite { id } => {
                if self.pending_journal_invites.remove(&id) {
                    _ = self.accepted_journal_invites.insert(id);
                }
            }
            UserEvent::RemovedFromJournal { id } => _ = self.accepted_journal_invites.remove(&id),
            UserEvent::Deleted => self.deleted = true,
        }
    }
}

pub async fn fetch_id(username: String, pool: &PgPool) -> Result<Uuid, ServerFnError> {
    let uuid: Result<Option<Uuid>, sqlx::Error> = query_scalar(
        r#"
        SELECT aggregate_id FROM events
        WHERE (event_type = $1 OR event_type = $2) AND payload->'data'->>'username' = $3
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(EventType::UserCreated)
    .bind(EventType::UsernameUpdated)
    .bind(&username)
    .fetch_optional(pool)
    .await;

    match uuid {
        Ok(Some(id)) => Ok(id),
        Ok(None) => Err(ServerFnError::ServerError(format!(
            "Unable to find a user with the username {}",
            username
        ))),
        Err(e) => Err(ServerFnError::ServerError(e.to_string())),
    }
}

pub async fn fetch_username(user_id: Uuid, pool: &PgPool) -> Result<String, ServerFnError> {
    let username_events: Vec<JsonValue> = match query_scalar(
        r#"SELECT payload FROM events
            WHERE aggregate_id = $1 AND (event_type = $2 OR event_type = $3)
            ORDER BY created_at ASC
            "#,
    )
    .bind(user_id)
    .bind(EventType::UserCreated)
    .bind(EventType::UsernameUpdated)
    .fetch_all(pool)
    .await
    {
        Ok(s) => s,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let mut user = UserState {
        ..Default::default()
    };

    for raw_event in username_events {
        let event: DomainEvent = match serde_json::from_value(raw_event) {
            Ok(s) => s,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };
        user.apply(event.to_user_event()?).await;
    }

    if !user.username.is_empty() {
        return Ok(user.username);
    }

    Err(ServerFnError::ServerError(
        "unable to fetch username from uuid".to_string(),
    ))
}

pub async fn get_hashed_pw(user_id: &Uuid, pool: &PgPool) -> Result<String, ServerFnError> {
    let password_events: Vec<JsonValue> = match query_scalar(
        r#"
        SELECT payload FROM events
        WHERE aggregate_id = $1 AND (event_type = $2 OR event_type = $3)
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .bind(EventType::UserCreated)
    .bind(EventType::UserPasswordUpdated)
    .fetch_all(pool)
    .await
    {
        Ok(s) => s,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let mut user = UserState {
        ..Default::default()
    };

    for raw_event in password_events {
        let event: DomainEvent = match serde_json::from_value(raw_event) {
            Ok(s) => s,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };
        user.apply(event.to_user_event()?).await;
    }
    Ok(user.hashed_password)
}

pub async fn is_authenticated(
    session_id: String,
    user_id: Uuid,
    pool: &PgPool,
) -> Result<bool, ServerFnError> {
    let auth_events: Vec<JsonValue> = match query_scalar(
        r#"
        SELECT payload FROM events
        WHERE aggregate_id = $1 AND (event_type = $2 OR event_type = $3)
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .bind(EventType::UserLoggedIn)
    .bind(EventType::UserLoggedOut)
    .fetch_all(pool)
    .await
    {
        Ok(s) => s,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let mut user = UserState {
        ..Default::default()
    };

    for raw_event in auth_events {
        let event: DomainEvent = match serde_json::from_value(raw_event) {
            Ok(s) => s,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };
        user.apply(event.to_user_event()?).await;
    }
    return Ok(user.authenticated_sessions.contains(&session_id));
}

pub async fn authenticate(
    session_id: String,
    user_id: Uuid,
    password: String,
    pool: &PgPool,
) -> Result<bool, ServerFnError> {
    let verified = match verify(password, get_hashed_pw(&user_id, pool).await?.as_str()) {
        Ok(s) => s,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };
    if verified {
        let _ = UserEvent::LoggedIn { session_id }
            .push_db(user_id, pool)
            .await?;
        return Ok(true);
    }
    Ok(false)
}
