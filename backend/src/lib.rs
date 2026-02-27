pub mod api;
pub mod enclave;
pub mod sui;
pub mod walrus;

pub struct AppState {
    pub sui_client: sui_rpc::Client,
}
