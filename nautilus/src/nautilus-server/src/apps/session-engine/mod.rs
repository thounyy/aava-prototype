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
    close_session,
    flag_session,
    revoke_session,
    get_session,
    CloseSessionRequest,
    CloseSessionResponse,
    FlagSessionRequest,
    FlagSessionResponse,
    OpenSessionRequest,
    OpenSessionResponse,
    RevokeSessionRequest,
    RevokeSessionResponse,
    GetSessionRequest,
    GetSessionResponse,
    open_session,
};

// Re-export stream types and functions
pub use streams::{
    cleanup_stream, end_stream, CleanupStreamRequest, EndStreamRequest, EndStreamResponse,
    SessionData,
};
