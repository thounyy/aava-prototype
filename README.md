# Aava Session Engine

Aava is a programmable video layer that leverages AI-powered compression and decentralized networks. This repository contains the session engine implementation: a **backend** orchestrator and an **enclave** service for secure, verifiable session management (TEE-style deployment path).

**Full stack details (on-chain/off-chain, flows, env):** **[ARCHITECTURE.md](ARCHITECTURE.md)**

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
│  Enclave        │  (Port 3000)
│  session_enclave│
└──────┬──────────┘
       │
       │ Session writes (enclave only)
       ▼
┌─────────────────┐
│     Redis       │
│   (Sessions)    │
│  localhost:6379 │
└─────────────────┘
```

## Run locally

### Pre-flight checklist

Before starting backend and enclave:

1. **Redis** – Default dev setup is unauthenticated `redis://localhost:6379`. For production-style auth, put the password in **`REDIS_URL`** (e.g. `redis://:your-password@127.0.0.1:6379/`) or use `requirepass` and the same URL form.

2. **Set environment variables** – Backend and enclave both need **`ENCLAVE_INTERNAL_TOKEN`**.

3. **Start order**: Redis → Enclave → Backend

### 1. Run Redis

```bash
# Dev (no password)
redis-server

# Or with requirepass — then set REDIS_URL to redis://:your-password@127.0.0.1:6379/
# redis-server --requirepass your-secure-password
# redis-cli -a your-secure-password ping  → PONG
```

### 2. Run the Enclave

```bash
cd enclave
export ENCLAVE_INTERNAL_TOKEN=your-shared-secret
export REDIS_URL=redis://localhost:6379   # optional; include :password@ if Redis uses requirepass
RUST_LOG=info cargo run
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
  -d '{"creator_handle":"your_handle"}'
```

### 2. Test streams

```bash
# Start a stream
curl -X POST "http://127.0.0.1:8080/api/streams/start" \
  -H "Content-Type: application/json" \
  -d '{"creator_handle":"your_handle"}'

# End a stream (registers blob, uploads to Walrus, certifies — all server-side)
curl -X POST "http://127.0.0.1:8080/api/streams/end" \
  -H "Content-Type: application/json" \
  -d '{"creator_handle":"your_handle","stream_id":"<STREAM_OBJECT_ID>"}'
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
   - In production, bind Redis to private networks and require authentication (password in **`REDIS_URL`** or TLS)
   - Only the **enclave** should hold `REDIS_URL` with credentials; the backend does not connect to Redis

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
- ✅ Use strong Redis credentials (32+ characters, random) embedded in `REDIS_URL` or secret-backed config
- ✅ Store Redis URL / secrets in secret management (not in code)
- ✅ Run Redis in isolated network/VPC
- ✅ Enable Redis TLS for encrypted connections
- ✅ Use Redis ACLs to restrict commands
- ✅ Monitor Redis access logs
- ✅ Run enclave in TEE (AWS Nitro Enclaves, Intel SGX, etc.)

**Development:**
- ⚠️ Default local Redis may be unauthenticated; restrict before exposing beyond localhost
- ⚠️ Don't commit credentials to git

## Configuration

### Environment Variables

**Backend:**
- `ENCLAVE_URL`: URL of the enclave (default: `http://localhost:3000`)
- `ENCLAVE_INTERNAL_TOKEN`: shared secret sent to enclave as `X-Internal-Token` (**required**)

**Enclave:**
- `REDIS_URL`: Redis connection URL (default: `redis://localhost:6379`; embed password in URL if needed)
- `ENCLAVE_INTERNAL_TOKEN`: shared secret expected from backend in `X-Internal-Token` (**required**)

### Ports

- **Backend**: `8080` (configurable in `backend/src/main.rs`)
- **Enclave**: `3000` (see `enclave/src/main.rs`)

## Next Steps

- [x] Complete session lifecycle with batch proof generation
- [x] Integrate Sui blockchain for verifying hash with Nautilus
- [x] Add Walrus integration for dataset publishing
- [ ] Deploy to AWS Nitro Enclaves and EC2 for testing
- [ ] Implement event listener and action API
- [ ] Add session event recording during stream

