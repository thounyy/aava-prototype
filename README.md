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

### Pre-flight checklist

Before starting backend and enclave:

1. **Start Redis with a password** – Redis must use `requirepass` so the enclave can authenticate:
   ```bash
   # One-off with password
   redis-server --requirepass your-secure-password

   # Or add to redis.conf: requirepass your-secure-password
   ```

2. **Set environment variables** – Both backend and enclave need `ENCLAVE_INTERNAL_TOKEN`. Enclave also needs `REDIS_PASSWORD` (must match Redis's `requirepass`).

3. **Start order**: Redis → Enclave → Backend

### 1. Run Redis

```bash
redis-server --requirepass your-secure-password
# Test: redis-cli -a your-secure-password ping  → PONG
```

### 2. Run the Enclave

```bash
cd nautilus/src/nautilus-server
export ENCLAVE_INTERNAL_TOKEN=your-shared-secret
export REDIS_PASSWORD=your-secure-password   # Same as Redis requirepass
RUST_LOG=info cargo run --bin nautilus-server
```

### 3. Run the Backend

```bash
cd backend
export ENCLAVE_INTERNAL_TOKEN=your-shared-secret   # Must match enclave
RUST_LOG=info cargo run
```

## Testing

### 1. Test accounts 

```bash
# Create a creator account (tx is built, signed & executed server-side)
# Use POST + JSON body; `/api/creators` or `/api/creators/create`
curl -X POST "http://127.0.0.1:8080/api/creators/create" \
  -H "Content-Type: application/json" \
  -d '{"account_handle":"your_handle"}'
```

### 2. Test streams

```bash
# Start a stream
curl -X POST "http://127.0.0.1:8080/api/streams/start" \
  -H "Content-Type: application/json" \
  -d '{"account_handle":"your_handle"}'

# End a stream (registers blob, uploads to Walrus, certifies — all server-side)
curl -X POST "http://127.0.0.1:8080/api/streams/end" \
  -H "Content-Type: application/json" \
  -d '{"account_handle":"your_handle","stream_id":"<STREAM_OBJECT_ID>"}'
```

### 3. Test sessions

```bash
# Open a session
curl -X POST "http://127.0.0.1:8080/api/sessions/open" \
  -H "Content-Type: application/json" \
  -d '{"viewer_handle":"viewer1","stream_id":"<STREAM_OBJECT_ID>"}'

# Close a session
curl -X POST "http://127.0.0.1:8080/api/sessions/close" \
  -H "Content-Type: application/json" \
  -d '{"session_id":"<SESSION_ID_FROM_OPEN>"}'
```

### 4. Verify Redis Sessions

```bash
# Connect to Redis (use same password as REDIS_PASSWORD)
redis-cli -a your-secure-password

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
- ⚠️ `REDIS_PASSWORD` is required (no unauthenticated Redis)
- ⚠️ Don't commit passwords to git

## Configuration

### Environment Variables

**Backend:**
- `ENCLAVE_URL`: URL of the enclave (default: `http://localhost:3000`)
- `ENCLAVE_INTERNAL_TOKEN`: shared secret sent to enclave as `X-Internal-Token` (**required**)

**Enclave:**
- `REDIS_URL`: Redis connection URL (default: `redis://localhost:6379`)
- `REDIS_PASSWORD`: Redis password (**required**)
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

