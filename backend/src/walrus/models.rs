use serde::{Deserialize, Serialize};

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
