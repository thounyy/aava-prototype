use crate::walrus::error::WalrusError;

const DEFAULT_UPLOAD_RELAY_URL: &str = "https://upload-relay.testnet.walrus.space";

pub async fn publish_dataset_to_walrus(
    blob_id: &str,
    blob_object_id: &str,
    dataset: Vec<u8>,
) -> Result<serde_json::Value, WalrusError> {
    let client = reqwest::Client::new();
    let relay_base_url = std::env::var("WALRUS_UPLOAD_RELAY_URL")
        .unwrap_or_else(|_| DEFAULT_UPLOAD_RELAY_URL.to_string());
    let url = format!(
        "{}/v1/blob-upload-relay?blob_id={}&deletable_blob_object={}",
        relay_base_url, blob_id, blob_object_id
    );

    let response = client.post(url).body(dataset).send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(WalrusError::ApiError(status, body));
    }

    let confirmation_certificate: serde_json::Value = response.json().await.map_err(|e| {
        WalrusError::ParseError(format!("Failed to parse confirmation certificate: {e}"))
    })?;

    Ok(confirmation_certificate)
}
