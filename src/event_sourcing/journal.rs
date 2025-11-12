use super::event::{AggregateType, DomainEvent, EventType};
use bitflags::bitflags;
use leptos::prelude::ServerFnError;
use serde::{Deserialize, Serialize};
use sqlx::{query_scalar, types::JsonValue, PgPool};
use std::collections::HashMap;
use uuid::Uuid;

bitflags! {
#[derive(Serialize, Deserialize, Hash, Default, Debug, Clone, Copy)]
    pub struct Permissions: u16 {
        const READ = 1 << 0;
        const ADDACCOUNT = 1 << 1;
        const APPENDTRANSACTION = 1 << 2;
        const INVITE = 1 << 3;
        const DELETE = 1 << 4;
    }
}

#[derive(Serialize, Deserialize)]
pub struct BalanceUpdate {
    pub account_name: String,
    pub changed_by: i64,
}

#[derive(Serialize, Deserialize)]
pub struct Transaction {
    pub author: Uuid,
    pub updates: Vec<BalanceUpdate>,
}

#[derive(Serialize, Deserialize)]
pub enum JournalEvent {
    Created { name: String, owner: Uuid },
    Renamed { name: String },
    CreatedAccount { account_name: String },
    DeletedAccount { account_name: String },
    AddedEntry { transaction: Transaction },
    Deleted,
}
impl JournalEvent {
    pub async fn push_db(&self, uuid: &Uuid, pool: &PgPool) -> Result<i64, ServerFnError> {
        let payload = serde_json::to_value(self)?;

        let id: i64 = sqlx::query_scalar(
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
        .bind(AggregateType::Journal)
        .bind(EventType::from_journal_event(self))
        .bind(payload)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct JournalState {
    pub id: Uuid,
    pub name: String,
    pub owner: Uuid,
    pub accounts: HashMap<String, i64>,
    pub transations: Vec<Transaction>,
    pub deleted: bool,
}

impl JournalState {
    pub async fn build(
        id: &Uuid,
        event_types: Vec<EventType>,
        pool: &PgPool,
    ) -> Result<Self, ServerFnError> {
        let journal_events: Vec<JsonValue> = query_scalar(
            r#"
                SELECT payload FROM events
                WHERE aggregate_id = $1 AND aggregate_type = $2 AND event_type = ANY($3)
                ORDER BY created_at ASC
                "#,
        )
        .bind(&id)
        .bind(AggregateType::Journal)
        .bind(&event_types)
        .fetch_all(pool)
        .await?;

        let mut aggregate = Self {
            id: *id,
            ..Default::default()
        };

        for raw_event in journal_events {
            let domain_event: DomainEvent = serde_json::from_value(raw_event)?;
            aggregate.apply(domain_event.to_journal_event()?);
        }
        Ok(aggregate)
    }

    pub fn from_events(id: Uuid, events: Vec<JsonValue>) -> Result<Self, ServerFnError> {
        let mut aggregate = Self {
            id,
            ..Default::default()
        };

        for raw_event in events {
            let domain_event: DomainEvent = serde_json::from_value(raw_event)?;
            aggregate.apply(domain_event.to_journal_event()?);
        }
        Ok(aggregate)
    }

    pub fn apply(&mut self, event: JournalEvent) {
        match event {
            JournalEvent::Created { name, owner } => {
                self.name = name;
                self.owner = owner;
            }

            JournalEvent::Renamed { name } => self.name = name,

            JournalEvent::CreatedAccount { account_name } => {
                _ = self.accounts.insert(account_name, 0)
            }
            JournalEvent::DeletedAccount { account_name } => {
                _ = self.accounts.remove(&account_name)
            }
            JournalEvent::AddedEntry { transaction } => {
                for balance_update in &transaction.updates {
                    self.accounts
                        .entry(balance_update.account_name.clone())
                        .and_modify(|balance| *balance += balance_update.changed_by);
                }
                _ = self.transations.push(transaction);
            }
            JournalEvent::Deleted => self.deleted = true,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct JournalTenantInfo {
    pub tenant_permissions: Permissions,
    pub inviting_user: Uuid,
    pub journal_owner: Uuid,
}

pub struct SharedJournal {
    pub id: Uuid,
    pub info: JournalTenantInfo,
}

pub struct SharedAndPendingJournals {
    pub shared: HashMap<Uuid, JournalTenantInfo>,
    pub pending: HashMap<Uuid, JournalTenantInfo>,
}

pub async fn get_id_from_name(name: String, pool: &PgPool) -> Result<Option<Uuid>, ServerFnError> {
    Ok(query_scalar(
        r#"
        SELECT aggregate_id FROM events
        WHERE (event_type = $1 OR event_type = $2) AND payload->'data'->>'username' = $3
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(EventType::JournalCreated)
    .bind(EventType::JournalRenamed)
    .bind(&name)
    .fetch_optional(pool)
    .await?)
}

pub async fn get_name_from_id(id: Uuid, pool: &PgPool) -> Result<Option<String>, ServerFnError> {
    let journal_state = JournalState::build(
        &id,
        vec![EventType::JournalCreated, EventType::JournalRenamed],
        pool,
    )
    .await?;
    if journal_state.name.is_empty() {
        return Ok(None);
    }
    Ok(Some(journal_state.name))
}
