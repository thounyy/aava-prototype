module aava::enclave;

// === Structs ===

public struct Cap has key, store {
    id: UID,
}

public struct Enclave has key {
    id: UID,
    expected_image_digest: vector<u8>,
}

// === Package functions ===

fun init(ctx: &mut TxContext) {
    transfer::share_object(
        Enclave {
            id: object::new(ctx),
            expected_image_digest: vector::empty(),
        }
    );

    transfer::public_transfer(
        Cap { id: object::new(ctx) }, 
        tx_context::sender(ctx)
    );
}

public(package) fun verify(
    enclave: &Enclave,
    // doc: GcpAttestationDocument, // TODO: uncomment in production
): bool {
    // let image_digest = *gcp_attestation::image_digest(&doc);
    let image_digest = vector::empty();
        
    image_digest == enclave.expected_image_digest
}

// === Admin functions ===

public fun set_expected_image_digest(
    enclave: &mut Enclave,
    _: &Cap,
    expected_image_digest: vector<u8>,
) {
    enclave.expected_image_digest = expected_image_digest;
}