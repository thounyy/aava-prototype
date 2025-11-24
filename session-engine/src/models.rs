// use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
// use uuid::Uuid;

// === Account ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountRequest {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountResponse {
    pub user_id: String,
    pub account_object_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAccountRequest {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAccountResponse {
    pub user_id: String,
    pub account_object_id: String,
    // TODO: Add account metadata
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountExistsRequest {
    pub user_id: Option<String>,
    pub account_object_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountExistsResponse {
    pub exists: bool,
}

// === Session ===

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SessionRequest {
//     pub user_id: String,
//     pub stream_id: String,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct SessionResponse {
//     pub session_id: String,
//     pub user_id: String,
//     pub stream_id: String,
//     pub status: SessionStatus,
//     pub created_at: DateTime<Utc>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum SessionStatus {
//     Created,
//     Active,
//     Completed,
//     Error(String),
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PermissionCheck {
//     pub user_id: String,
//     pub stream_id: String,
//     pub has_permission: bool,
//     pub permission_type: Option<PermissionType>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum PermissionType {
//     Subscription,
//     PayPerView,
//     Free,
// }

// #[derive(Debug, Clone, sqlx::FromRow)]
// pub struct Session {
//     pub id: Uuid,
//     pub user_id: String,
//     pub stream_id: String,
//     pub status: String,
//     pub created_at: DateTime<Utc>,
// }

// // Note: Stream objects live on-chain (Sui blockchain)
// // See sui.rs for stream information retrieval





