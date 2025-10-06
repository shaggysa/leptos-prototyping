#[cfg(feature = "ssr")]
use axum::Extension;

use leptos::prelude::*;

#[cfg(feature = "ssr")]
use sqlx::SqlitePool;

#[cfg(feature = "ssr")]
use leptos_axum::extract;

#[cfg(feature = "ssr")]
use tower_sessions::Session;

#[cfg(feature = "ssr")]
use crate::types::Account;

#[cfg(feature = "ssr")]
#[server]
pub async fn get_accounts() -> Result<Vec<Account>, ServerFnError> {
    let Extension(session) = match extract::<Extension<Session>>().await {
        Ok(s) => s,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let Extension(pool) = match extract::<Extension<SqlitePool>>().await {
        Ok(s) => s,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    // force initialize the session
    if session
        .get::<bool>("initialized")
        .await
        .ok()
        .flatten()
        .is_none()
    {
        match session.insert("initialized", true).await {
            Ok(s) => s,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };
    }

    let session_id = match session.id() {
        Some(s) => s.to_string(),
        None => {
            return Err(ServerFnError::ServerError(
                "try refreshing the page".to_string(),
            ))
        }
    };

    let accounts: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT id, title, balance_cents FROM accounts WHERE session_id = ? ORDER BY id DESC",
    )
    .bind(&session_id)
    .fetch_all(&pool)
    .await?;

    Ok(accounts
        .into_iter()
        .map(|(id, title, balance_cents)| Account {
            id,
            title,
            balance_cents,
        })
        .collect())
}

#[cfg(feature = "ssr")]
#[server]
pub async fn add_account(title: String) -> Result<(), ServerFnError> {
    let Extension(session) = extract::<Extension<Session>>().await?;

    let Extension(pool) = extract::<Extension<SqlitePool>>().await?;

    // force initialize the session
    if session
        .get::<bool>("initialized")
        .await
        .ok()
        .flatten()
        .is_none()
    {
        match session.insert("initialized", true).await {
            Ok(s) => s,
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };
    }

    let session_id = match session.id() {
        Some(s) => s.to_string(),
        None => {
            return Err(ServerFnError::ServerError(
                "Session id not found".to_string(),
            ))
        }
    };

    sqlx::query("INSERT INTO accounts (session_id, title, balance_cents) VALUES (?, ?, 0)")
        .bind(&session_id)
        .bind(title.trim())
        .execute(&pool)
        .await?;

    Ok(())
}
