// Copyright (c), Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// ====
/// Session Engine Nautilus App
/// Handles session creation and termination with direct database writes
/// ====
pub mod sessions;
pub mod streams;

// Re-export session types and functions
pub use sessions::{
    close_session, open_session, CloseSessionRequest, CloseSessionResponse, OpenSessionRequest,
    OpenSessionResponse,
};

// Re-export stream types and functions
pub use streams::{
    cleanup_stream, end_stream, CleanupStreamRequest, EndStreamRequest, EndStreamResponse,
    SessionData,
};
