# Aava Session Engine

Aava is a programmable video layer that leverages AI-powered compression and decentralized networks. This repository contains the session engine implementation using Nautilus (TEE) for secure, verifiable session management.

## Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       │ POST /api/sessions/open
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

## Quick Start

### 1. Setup Redis

```bash
# Install Redis (Fedora/RHEL)
sudo dnf install redis

# Configure Redis security
sudo nano /etc/redis/redis.conf

# Set the following:
# 1. Bind to localhost only (for security)
bind 127.0.0.1

# 2. Enable protected mode
protected-mode yes

# 3. Set a strong password (REQUIRED for production)
# Generate a secure password:
openssl rand -base64 32

# Add to redis.conf:
requirepass your-generated-password-here

# Save and exit, then restart Redis
sudo systemctl restart redis
sudo systemctl enable redis

# Test Redis connection
redis-cli -a your-generated-password-here ping
# Should return: PONG
```

### 2. Run the Enclave (Terminal 1)

```bash
cd nautilus/src/nautilus-server

# Set Redis connection
# Option 1: Password in URL
export REDIS_URL="redis://:your-password@localhost:6379"

# Option 2: Password as separate env var (more secure, doesn't show in process list)
export REDIS_URL="redis://localhost:6379"
export REDIS_PASSWORD="your-password"

# Run the enclave
RUST_LOG=info cargo run --bin nautilus-server
```

**Security Note**: The enclave is the ONLY process that should have access to Redis. 
- Redis is bound to localhost only
- Password authentication is required
- Only the enclave writes/reads session data
- Cryptographic attestation ensures data integrity

The enclave will:
- Connect to Redis
- Listen on `http://localhost:3000`
- Handle `/open_session`, `/close_session`, `/end_stream`, and `/cleanup_sessions` endpoints

### 3. Run the Backend (Terminal 2)

```bash
cd backend

# Set enclave URL (points to enclave)
export ENCLAVE_URL="http://localhost:3000"

# Note: Backend no longer needs database connection
# Sessions are stored in Redis by the enclave

# Run the backend
cargo run
```

The backend will:
- Run database migrations
- Listen on `http://localhost:8080`
- Forward requests to the enclave

### 4. Test the Integration

```bash
# Test opening a session
curl -X POST http://localhost:8080/api/sessions/open \
  -H "Content-Type: application/json" \
  -d '{
    "viewer_id": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    "stream_id": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
  }'

# Response:
# {
#   "session_id": "550e8400-e29b-41d4-a716-446655440000",
#   "viewer_id": "0x1234...",
#   "stream_id": "0xabcd...",
#   "status": "Open",
#   "created_at": "2025-01-15T10:00:00Z"
# }

# Test closing a session (use session_id from above)
curl -X POST http://localhost:8080/api/sessions/close \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "550e8400-e29b-41d4-a716-446655440000"
  }'
```

## Testing

### Test Enclave Directly

```bash
# Test enclave health check
curl http://localhost:3000/health_check

# Test opening session directly
curl -X POST http://localhost:3000/open_session \
  -H "Content-Type: application/json" \
  -d '{
    "viewer_id": "0x1234",
    "stream_id": "0x5678"
  }'
```

### Test Backend → Enclave Flow

```bash
# Test via backend (which forwards to enclave)
curl -X POST http://localhost:8080/api/sessions/open \
  -H "Content-Type: application/json" \
  -d '{
    "viewer_id": "0x1234",
    "stream_id": "0x5678"
  }'
```

### Verify Redis Sessions

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
     - Calculates SHA-256 hash of the session data
     - Signs the data with the enclave's private key
   - The hash proves data integrity at the time of attestation
   - The signature proves the enclave saw the correct data
   - Any tampering with Redis data will result in a mismatched hash

4. **On-Chain Verification**
   - The hash and signature are published to Sui blockchain
   - Anyone can verify the data integrity by:
     - Downloading the dataset from Walrus
     - Calculating the hash
     - Comparing with the on-chain hash
     - Verifying the enclave signature

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

**Enclave:**
- `REDIS_URL`: Redis connection URL (default: `redis://localhost:6379`)
- `REDIS_PASSWORD`: Redis password (recommended, more secure than URL)
  - Alternative: Include password in `REDIS_URL` as `redis://:password@localhost:6379`

### Ports

- **Backend**: `8080` (configurable in `backend/src/main.rs`)
- **Enclave**: `3000` (configurable in `nautilus/src/nautilus-server/src/main.rs`)

## Next Steps

- [x] Complete session lifecycle with batch proof generation
- [ ] Integrate Sui blockchain for verifying hash with Nautilus
- [ ] Add Walrus integration for dataset publishing
- [ ] Deploy to AWS Nitro Enclaves and EC2 for testing
- [ ] Implement event listener and action API
- [ ] Add session event recording during stream

