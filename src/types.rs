#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: u32,
    pub title: String,
    pub balance_cents: i64,
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Transaction {
    pub id: u32,
    // session_id is in the db entry, but I dont think it's ever necessary to return it
    pub created_at: i64,
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialTransaction {
    pub transaction_id: u32,
    pub account_id: u32,
    pub account_name: String,
    pub balance_diff_cents: i64,
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceUpdate {
    pub id: u32,
    pub balance_diff_cents: i64,
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransactionResult {
    UPDATED,
    BALANCEMISMATCH,
}

#[cfg(feature = "ssr")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackagedTransaction {
    pub parent: Transaction,
    pub children: Vec<PartialTransaction>,
}
