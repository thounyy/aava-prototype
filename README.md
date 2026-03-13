# Aava Session Engine

Aava is a programmable video layer that leverages AI-powered compression and decentralized networks. This repository contains the session engine implementation using Nautilus (TEE) for secure, verifiable session management.

## Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       │ POST /api/viewers/{viewer}/streams/{stream}/sessions
       ▼
┌─────────────────┐
│   Backend API   │  (Port 8080)
│   (Proxy)       │
└──────┬──────────┘
       │
       │ HTTP Request
       │ ENCLAVE_URL
       ▼
┌─────────────────┐
│  Nautilus       │  (Port 3000)
│  Enclave (TEE)  │
└──────┬──────────┘
       │
       │ Authenticated Write
       │ (Password Protected)
       ▼
┌─────────────────┐
│     Redis       │
│   (Sessions)    │
│  localhost:6379 │
│  Auth Required  │
└─────────────────┘
```

## Run locally

### 1. Run Redis

```bash
# Start Redis
redis-server
# Test Redis connection
redis-cli ping
# Should return: PONG
```

### 2. Run the Enclave

```bash
cd nautilus/src/nautilus-server
# Run the enclave
RUST_LOG=info cargo run --bin nautilus-server
```

### 3. Run the Backend

```bash
cd backend
# Run the backend
RUST_LOG=info cargo run
```

## Testing

### 1. Test accounts 

```bash
# Create a creator account (tx is built, signed & executed server-side)
curl -X POST "http://127.0.0.1:8080/api/creators/<ACCOUNT_IDENTIFIER>"
```

### 2. Test streams

```bash
# Start a stream
curl -X POST "http://127.0.0.1:8080/api/creators/<ACCOUNT_IDENTIFIER>/streams"
  
# End a stream (registers blob, uploads to Walrus, certifies — all server-side)
curl -X POST "http://127.0.0.1:8080/api/creators/<ACCOUNT_IDENTIFIER>/streams/<STREAM_ID>/end"
```

### 3. Test sessions

```bash
# Test opening a session
curl -X POST "http://127.0.0.1:8080/api/viewers/<VIEWER_IDENTIFIER>/streams/<STREAM_ID>/sessions"

# Test closing a session
curl -X POST "http://127.0.0.1:8080/api/viewers/<VIEWER_IDENTIFIER>/sessions/<SESSION_ID_FROM_OPEN>/close"
```

### 4. Verify Redis Sessions

```bash
# Connect to Redis (with password)
redis-cli -a your-password

# Check all session keys
KEYS session:*

# Check stream session sets
KEYS stream:*:sessions

# Get session count for a stream
SMEMBERS stream:your-stream-id:sessions

# Get a specific session
GET session:your-session-id
```

## Security

### Data Integrity Guarantees

The architecture ensures data integrity through multiple layers:

1. **Redis Access Control**
   - Redis is bound to `localhost` only (no external access)
   - Password authentication required (`requirepass`)
   - Only the enclave has the password (via `REDIS_PASSWORD` env var)
   - Other processes on the machine cannot access Redis without the password

2. **Enclave Isolation**
   - The enclave (TEE) is the **only** process that writes to Redis
   - All session operations go through the enclave
   - The backend never directly accesses Redis

3. **Cryptographic Attestation**
   - When ending a stream, the enclave:
     - Reads all sessions from Redis
     - Computes the blob metadata from the dataset
     - Signs the data with the enclave's private key
   - The blob_id proves data integrity at the time of attestation
   - The signature proves the enclave saw the correct data
   - Any tampering with Redis data will result in a mismatched hash

4. **On-Chain Verification**
   - The signature is verified when registering the blob on Sui
   - Anyone can verify the data integrity by:
    - (Optional) Verify the Nautilus off-chain server code by building it locally and confirming that the generated PCRs match the on-chain records.
    - Send a request to the deployed enclave and receive a signed response.
    - Submit the signed response on-chain for verification before executing the corresponding application logic.
    - Downloading the dataset from Walrus using the blob id stored on Sui

### Security Best Practices

**Production Deployment:**
- ✅ Use strong Redis password (32+ characters, random)
- ✅ Store `REDIS_PASSWORD` in secure secret management (not in code)
- ✅ Run Redis in isolated network/VPC
- ✅ Enable Redis TLS for encrypted connections
- ✅ Use Redis ACLs to restrict commands
- ✅ Monitor Redis access logs
- ✅ Run enclave in TEE (AWS Nitro Enclaves, Intel SGX, etc.)

**Development:**
- ⚠️ Redis password is still required (prevents accidental access)
- ⚠️ Use `REDIS_PASSWORD` env var (more secure than URL)
- ⚠️ Don't commit passwords to git

## Configuration

### Environment Variables

**Backend:**
- `ENCLAVE_URL`: URL of the enclave (default: `http://localhost:3000`)
- `ENCLAVE_INTERNAL_TOKEN`: shared secret sent to enclave as `X-Internal-Token` (**required**)

**Enclave:**
- `REDIS_URL`: Redis connection URL (default: `redis://localhost:6379`)
- `REDIS_PASSWORD`: Redis password (recommended, more secure than URL)
- Alternative: Include password in `REDIS_URL` as `redis://:password@localhost:6379`
- `ENCLAVE_INTERNAL_TOKEN`: shared secret expected from backend in `X-Internal-Token` (**required**)

### Ports

- **Backend**: `8080` (configurable in `backend/src/main.rs`)
- **Enclave**: `3000` (configurable in `nautilus/src/nautilus-server/src/main.rs`)

## Next Steps

- [x] Complete session lifecycle with batch proof generation
- [x] Integrate Sui blockchain for verifying hash with Nautilus
- [x] Add Walrus integration for dataset publishing
- [ ] Deploy to AWS Nitro Enclaves and EC2 for testing
- [ ] Implement event listener and action API
- [ ] Add session event recording during stream

