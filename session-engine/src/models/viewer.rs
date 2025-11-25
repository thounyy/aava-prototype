// use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
// use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewerAccountRequest {
    pub user_handle: String, // to define (could be platform user id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateViewerAccountResponse {
    pub user_handle: String,
    pub account_id: String,
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct GetAccountRequest {
//     pub user_id: String,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct GetAccountResponse {
//     pub user_id: String,
//     pub account_object_id: String,
//     // TODO: Add account metadata
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AccountExistsRequest {
//     pub user_id: Option<String>,
//     pub account_object_id: Option<String>,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct AccountExistsResponse {
//     pub exists: bool,
// }



