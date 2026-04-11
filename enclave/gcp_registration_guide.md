# GCP Confidential Spaces Registration on Sui

This guide explains how to register and verify GCP Confidential Spaces workloads on the Sui blockchain.

## Overview

GCP Confidential Spaces produce OIDC JWT attestation tokens that prove:
- The container image digest (what code is running)
- The hardware platform (AMD SEV, Intel TDX, etc.)
- The software environment (Confidential Space version)
- An ephemeral public key generated on boot

These tokens are verified on-chain to register trusted workload instances.

## Measurements

GCP Confidential Spaces use the following key measurement for registration:
- **image_digest**: SHA-256 hash of the container image (e.g., `sha256:ac91dd368193efa938776f35ff715e29f907c2b6671bae6589d44f60c8d82a54`) - **This is the critical security measurement**
- **eat_nonce**: Contains the Base64-encoded ephemeral public key

Optional metadata available in the JWT (can be validated but not required for registration):
- **hwmodel**: Hardware model (e.g., `GCP_AMD_SEV`, `GCP_INTEL_TDX`)
- **swname**: Software name (e.g., `CONFIDENTIAL_SPACE`)
- **swversion**: Software version (e.g., `["250301"]`)

## Move Module Structure

### EnclaveConfig Object (Shared)
Stores the approved workload configuration:

```move
struct EnclaveConfig<phantom T> has key {
    id: UID,
    // Container image measurement
    image_digest: String,  // e.g., "sha256:ac91dd3681..."
    
    // Software attestation
    swname: String,        // e.g., "CONFIDENTIAL_SPACE"
    swversion: vector<String>,  // e.g., ["250301"]
    
    // Hardware attestation
    hwmodel: String,       // e.g., "GCP_AMD_SEV" or "GCP_INTEL_TDX"
    
    // Version tracking
    config_version: u64,
    name: String,
}
```

### EnclaveCap Object (Owned)
Admin capability for managing the configuration:

```move
struct EnclaveCap<phantom T> has key, store {
    id: UID,
}
```

### Enclave Object (Shared)
Represents a registered workload instance:

```move
struct Enclave has key, store {
    id: UID,
    config_version: u64,
    public_key: vector<u8>,  // Ed25519 public key from ephemeral keypair
}
```

## Deployment and Registration Flow

### Step 1: Build and Deploy Container

```bash
# Build the container image
docker build -t us-central1-docker.pkg.dev/PROJECT/REPO/hello_world:latest .

# Push to Artifact Registry
docker push us-central1-docker.pkg.dev/PROJECT/REPO/hello_world:latest

# Get the image digest
IMAGE_DIGEST=$(gcloud artifacts docker images describe \
  us-central1-docker.pkg.dev/PROJECT/REPO/hello_world:latest \
  --format='value(image_summary.digest)')

echo $IMAGE_DIGEST
# sha256:ac91dd368193efa938776f35ff715e29f907c2b6671bae6589d44f60c8d82a54
```

### Step 2: Deploy Move Packages

```bash
# Deploy the enclave framework package
cd move/enclave
sui client publish

# Record the package ID and created objects
ENCLAVE_PACKAGE_ID=0x...

# Deploy your application package
cd move/YOUR_APP
sui client publish

# Record the package ID and created objects
APP_PACKAGE_ID=0x...
CAP_OBJECT_ID=0x...           # EnclaveCap object
ENCLAVE_CONFIG_OBJECT_ID=0x...  # EnclaveConfig object

# Set your application details
MODULE_NAME=your_module
OTW_NAME=YOUR_OTW
```

### Step 3: Register Image Digest

Update the EnclaveConfig with your workload's measurements:

```bash
sui client call \
  --function update_image_digest \
  --module enclave \
  --package $ENCLAVE_PACKAGE_ID \
  --type-args "$APP_PACKAGE_ID::$MODULE_NAME::$OTW_NAME" \
  --args $ENCLAVE_CONFIG_OBJECT_ID \
    $CAP_OBJECT_ID \
    "sha256:ac91dd368193efa938776f35ff715e29f907c2b6671bae6589d44f60c8d82a54" \
    "CONFIDENTIAL_SPACE" \
    "[\"250301\"]" \
    "GCP_AMD_SEV"

# Optionally set a descriptive name
sui client call \
  --function update_name \
  --module enclave \
  --package $ENCLAVE_PACKAGE_ID \
  --type-args "$APP_PACKAGE_ID::$MODULE_NAME::$OTW_NAME" \
  --args $ENCLAVE_CONFIG_OBJECT_ID \
    $CAP_OBJECT_ID \
    "My GCP Enclave - Updated 2025-11-20"
```

### Step 4: Deploy Confidential Space Instance

```bash
# Deploy to GCP Confidential Space
# (Use your terraform/deployment scripts)
ENCLAVE_URL=http://YOUR_GCP_INSTANCE:3000
```

### Step 5: Register Enclave Instance

Get the attestation from your deployed instance and register it on-chain:

```bash
# Get attestation from the deployed enclave
ATTESTATION_RESPONSE=$(curl -s $ENCLAVE_URL/attestation)

# Extract JWT and public key
JWT=$(echo $ATTESTATION_RESPONSE | jq -r '.jwt')
PUBLIC_KEY=$(echo $ATTESTATION_RESPONSE | jq -r '.public_key')

# Register the enclave instance on-chain
sui client call \
  --function register_enclave \
  --module enclave \
  --package $ENCLAVE_PACKAGE_ID \
  --type-args "$APP_PACKAGE_ID::$MODULE_NAME::$OTW_NAME" \
  --args $ENCLAVE_CONFIG_OBJECT_ID \
    "$JWT" \
    "$PUBLIC_KEY"

# Record the created Enclave object ID
ENCLAVE_OBJECT_ID=0x...
```

## Move Implementation

### update_image_digest Function

```move
public fun update_image_digest<T>(
    config: &mut EnclaveConfig<T>,
    cap: &EnclaveCap<T>,
    image_digest: String,
    swname: String,
    swversion: vector<String>,
    hwmodel: String,
) {
    config.image_digest = image_digest;
    config.swname = swname;
    config.swversion = swversion;
    config.hwmodel = hwmodel;
    config.config_version = config.config_version + 1;
}
```

### register_enclave Function

```move
public fun register_enclave<T>(
    config: &EnclaveConfig<T>,
    jwt: String,
    public_key_b64: String,
    ctx: &mut TxContext
): Enclave {
    // 1. Verify JWT signature using on-chain JWKS (managed by validators)
    let verified_claims = sui::gcp_attestation::verify_attestation(jwt);
    
    // 2. Verify the image digest matches the registered config
    assert!(
        verified_claims.image_digest == config.image_digest,
        EImageDigestMismatch
    );
    
    // 3. Verify software version matches
    assert!(verified_claims.swname == config.swname, ESwnameMismatch);
    
    // 4. Verify hardware model matches
    assert!(verified_claims.hwmodel == config.hwmodel, EHwmodelMismatch);
    
    // 5. Verify the public key in nonce matches provided public key
    assert!(
        verified_claims.eat_nonce == public_key_b64,
        EPublicKeyMismatch
    );
    
    // 6. Decode the public key
    let pk_bytes = base64::decode(public_key_b64);
    
    // 7. Create and share the Enclave object
    Enclave {
        id: object::new(ctx),
        config_version: config.config_version,
        public_key: pk_bytes,
    }
}
```

## Using Verified Computation

### Step 1: Request Computation from Enclave

```bash
# Request the enclave to process data
curl -H 'Content-Type: application/json' \
  -d '{"payload": {"data": "your_input"}}' \
  -X POST http://YOUR_GCP_INSTANCE:3000/process_data

# Response includes signature from ephemeral keypair
{
  "response": {
    "intent": 0,
    "timestamp_ms": 1732134000000,
    "data": {"result": "processed_output"}
  },
  "signature": "a1b2c3..."
}
```

### Step 2: Submit to Smart Contract

```bash
# Use the signature to call your application's Move function
sui client call \
  --function update_data \
  --module $MODULE_NAME \
  --package $APP_PACKAGE_ID \
  --type-args "$APP_PACKAGE_ID::$MODULE_NAME::$OTW_NAME" \
  --args $ENCLAVE_OBJECT_ID \
    "a1b2c3..." \
    1732134000000 \
    "processed_output"
```

### Step 3: Verify in Move Contract

Your application verifies the signature came from a registered enclave:

```move
public fun verify_and_update<T>(
    enclave: &Enclave,
    signature_hex: String,
    timestamp_ms: u64,
    data: YourDataType,
    ctx: &mut TxContext
) {
    // Create the intent message
    let intent_msg = create_intent_message(
        0, // intent scope
        timestamp_ms,
        data,
    );
    
    // Serialize to bytes
    let intent_msg_bytes = bcs::to_bytes(&intent_msg);
    
    // Decode signature from hex
    let signature_bytes = hex::decode(signature_hex);
    
    // Verify Ed25519 signature using enclave's registered public key
    let valid = sui::ed25519::ed25519_verify(
        &signature_bytes,
        &enclave.public_key,
        &intent_msg_bytes
    );
    
    assert!(valid, EInvalidSignature);
    
    // Process the verified data
    // ...
}
```

## Enclave Management

### Multiple Instances
You can register multiple Enclave objects for the same EnclaveConfig. Each instance has its own ephemeral public key but must match the same image_digest and configuration.

### Updating the Configuration
When you update your container image:

1. Build and deploy the new image
2. Get the new image digest
3. Call `update_image_digest` with the new measurements
4. Re-register all enclave instances with the new `config_version`

### Destroying Enclaves
The admin can destroy Enclave objects that are no longer needed:

```move
public fun destroy_enclave(
    enclave: Enclave,
    cap: &EnclaveCap<T>,
) {
    let Enclave { id, .. } = enclave;
    object::delete(id);
}
```

## Security Notes

1. **Image Digest**: Always verify the image digest matches your built container exactly
2. **Hardware Model**: Specify the expected hardware platform (AMD SEV or Intel TDX)
3. **Nonce Verification**: The `eat_nonce` containing the public key prevents replay attacks
4. **Validator Consensus**: JWT verification relies on the Sui validator set maintaining the JWKS
5. **Ephemeral Keys**: Keys are generated fresh on each boot and never leave the confidential space
