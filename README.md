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
       │ Direct Write
       ▼
┌─────────────────┐
│   PostgreSQL    │
│   Database      │
└─────────────────┘
```

## Quick Start

### 1. Setup PostgreSQL

```bash
# Install PostgreSQL (Fedora/RHEL)
sudo dnf install postgresql postgresql-server

# Initialize database (first time only)
sudo postgresql-setup --initdb

# Start PostgreSQL
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Configure authentication for TCP/IP connections
sudo nano /var/lib/pgsql/data/pg_hba.conf

# Change these lines:
# host    all    all    127.0.0.1/32    ident
# host    all    all    ::1/128         ident
# To:
# host    all    all    127.0.0.1/32    md5
# host    all    all    ::1/128         md5

# Set password for postgres user
sudo -u postgres psql -c "ALTER USER postgres WITH PASSWORD 'postgres';"

# Reload PostgreSQL
sudo systemctl reload postgresql

# Create database
sudo -u postgres psql -c "CREATE DATABASE aava;"
```

### 2. Run the Enclave (Terminal 1)

```bash
cd nautilus/src/nautilus-server

# Set database connection
export DATABASE_URL="postgresql://postgres:postgres@localhost/aava"

# Run the enclave
RUST_LOG=info cargo run --bin nautilus-server
```

The enclave will:
- Connect to PostgreSQL
- Listen on `http://localhost:3000`
- Handle `/open_session` and `/close_session` endpoints

### 3. Run the Backend (Terminal 2)

```bash
cd backend

# Set enclave URL (points to enclave)
export ENCLAVE_URL="http://localhost:3000"

# Set database URL (for migrations only)
export DATABASE_URL="postgresql://postgres:postgres@localhost/aava"

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

### Verify Database

```bash
# Check sessions in database
psql -h localhost -U postgres -d aava -c "SELECT * FROM sessions;"

# Count sessions
psql -h localhost -U postgres -d aava -c "SELECT COUNT(*) FROM sessions;"
```

## Configuration

### Environment Variables

**Backend:**
- `ENCLAVE_URL`: URL of the enclave (default: `http://localhost:3000`)
- `DATABASE_URL`: Database connection string (for migrations only)

**Enclave:**
- `DATABASE_URL`: Database connection string (default: `postgresql://postgres:postgres@localhost/aava`)

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

