use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionRequest {
    pub viewer_id: String,
    pub stream_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenSessionResponse {
    pub session_id: String,
    pub viewer_id: String,
    pub stream_id: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Created,
    Active,
    Completed,
    Error(String),
}

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
