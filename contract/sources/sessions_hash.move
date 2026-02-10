module aava::sessions_hash;

// === Imports ===

use enclave::enclave::{Self, Enclave};

// === Errors ===

const EInvalidSignature: u64 = 1;

// === Structs ===

public struct SESSIONS_HASH has drop {}

// === Public functions ===

fun init(otw: SESSIONS_HASH, ctx: &mut TxContext) {
    let cap = enclave::new_cap(otw, ctx);

    cap.create_enclave_config(
        std::string::utf8(b"Aava Session Engine"),
        x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", // pcr0
        x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", // pcr1
        x"000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", // pcr2
        ctx,
    );

    transfer::public_transfer(cap, tx_context::sender(ctx))
}

public(package) fun verify<SESSIONS_HASH>(
    enclave: &Enclave<SESSIONS_HASH>,
    blob_id_bytes: vector<u8>,
    timestamp_ms: u64,
    sig: &vector<u8>,
) {
    let res = enclave::verify_signature<SESSIONS_HASH, vector<u8>>(
        enclave,
        0, // HashSessions intent
        timestamp_ms,
        blob_id_bytes,
        sig,
    );
    assert!(res, EInvalidSignature);
}