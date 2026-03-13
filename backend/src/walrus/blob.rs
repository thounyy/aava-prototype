use base64::engine::{general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::walrus::error::WalrusError;

const DEFAULT_UPLOAD_RELAY_URL: &str = "https://upload-relay.testnet.walrus.space";

/// Represents a Blob object in the Walrus API.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlobObject {
    /// The unique ID of the Blob.
    pub id: String,
    /// The epoch at which the Blob was registered.
    pub registered_epoch: u64,
    /// The ID of the Blob.
    pub blob_id: String,
    /// The size of the Blob.
    pub size: u64,
    /// The encoding type of the Blob.
    pub encoding_type: String,
    /// The epoch at which the Blob was certified (if applicable).
    pub certified_epoch: Option<u64>,
    /// Storage information for the Blob.
    pub storage: StorageInfo,
    /// Indicates if the Blob is deletable.
    pub deletable: bool,
}

/// Represents storage information for a Blob.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    /// The storage ID.
    pub id: String,
    /// The starting epoch of the storage.
    pub start_epoch: u64,
    /// The ending epoch of the storage.
    pub end_epoch: u64,
    /// The size of the storage.
    pub storage_size: u64,
}

/// Represents a resource operation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceOperation {
    /// Details for a register from scratch operation (if applicable).
    pub register_from_scratch: Option<RegisterFromScratch>,
}

/// Represents details for a register from scratch operation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterFromScratch {
    /// The encoded length.
    pub encoded_length: u64,
    /// The number of epochs ahead.
    pub epochs_ahead: u64,
}

/// Represents an event in the Walrus API.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// The transaction digest.
    pub tx_digest: String,
    /// The event sequence.
    pub event_seq: String,
}

/// Either an event ID or an object ID, aligned with walrus-sdk.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventOrObjectId {
    Event(Event),
    Object(String),
}

/// Represents the result of a Blob storage operation (aligned with walrus-sdk).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum BlobStoreResult {
    AlreadyCertified {
        blob_id: String,
        #[serde(flatten)]
        event_or_object: EventOrObjectId,
        end_epoch: u64,
    },
    NewlyCreated {
        blob_object: BlobObject,
        resource_operation: ResourceOperation,
        cost: u64,
        shared_blob_object: Option<String>,
    },
    MarkedInvalid {
        blob_id: String,
        event: Event,
    },
    Error {
        blob_id: Option<String>,
        failure_phase: String,
        error_msg: String,
    },
}

#[derive(Debug, Deserialize)]
struct RawUploadRelayResponse {
    blob_id: Vec<u8>,
    confirmation_certificate: RawConfirmationCertificate,
}

#[derive(Debug, Deserialize)]
struct RawConfirmationCertificate {
    signers: Vec<u16>,
    serialized_message: Vec<u8>,
    signature: String,
}

#[derive(Debug)]
pub struct UploadRelayResponse {
    pub blob_id: Vec<u8>,
    pub certificate: CertificateData,
}

#[derive(Debug)]
pub struct CertificateData {
    pub signers: Vec<u16>,
    pub serialized_message: Vec<u8>,
    pub signature: Vec<u8>,
}

pub async fn upload_dataset(
    object_id: &str,
    blob_id: &str,
    dataset: Vec<u8>,
    tx_id: Option<&str>,
    nonce_b64: Option<&str>,
) -> Result<UploadRelayResponse, WalrusError> {
    let client = reqwest::Client::new();
    let mut url = format!(
        "{}/v1/blob-upload-relay?blob_id={}&deletable_blob_object={}",
        DEFAULT_UPLOAD_RELAY_URL, blob_id, object_id
    );
    if let Some(tx_id) = tx_id {
        url.push_str(&format!("&tx_id={}", tx_id));
    }
    if let Some(nonce) = nonce_b64 {
        url.push_str(&format!("&nonce={}", nonce));
    }

    let response = client
        .post(&url)
        .header("Content-Type", "application/octet-stream")
        .body(dataset)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(WalrusError::ApiError(status, body));
    }

    let body = response.text().await.map_err(|e| {
        WalrusError::ParseError(format!("Failed to read upload relay response body: {e}"))
    })?;
    info!("Upload relay response body: {}", body);

    let raw: RawUploadRelayResponse = serde_json::from_str(&body).map_err(|e| {
        WalrusError::ParseError(format!(
            "Failed to parse upload relay response: {e}\nBody: {body}"
        ))
    })?;

    let signature_bytes = STANDARD
        .decode(&raw.confirmation_certificate.signature)
        .map_err(|e| {
            WalrusError::ParseError(format!("Failed to decode certificate signature: {e}"))
        })?;

    Ok(UploadRelayResponse {
        blob_id: raw.blob_id,
        certificate: CertificateData {
            signers: raw.confirmation_certificate.signers,
            serialized_message: raw.confirmation_certificate.serialized_message,
            signature: signature_bytes,
        },
    })
}
