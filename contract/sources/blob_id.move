module aava::blob_id;

// === Imports ===

use enclave::enclave::{Self, Enclave};

// === Structs ===

public struct BLOB_ID has drop {}

// === Public functions ===

fun init(otw: BLOB_ID, ctx: &mut TxContext) {
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

public(package) fun verify<BLOB_ID>(
    enclave: &Enclave<BLOB_ID>,
    blob_id: u256,
    timestamp_ms: u64,
    sig: &vector<u8>,
): bool {
    enclave::verify_signature<BLOB_ID, u256>(
        enclave,
        0, // HashSessions intent
        timestamp_ms,
        blob_id,
        sig,
    )
}