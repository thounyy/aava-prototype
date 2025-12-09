# Aava Session Engine - Prototype

This is a prototype implementation of the session opening flow for the Aava protocol.

## Architecture

```
User (client.sh) → Backend API → Sui Blockchain (permissions) → TEE → Database (sessions) → Streaming
```

**On-Chain (Sui Blockchain):**
- IP owner account with licenses
- Broadcaster account with broadcast objects (metadata, permission types, pricing) 
- Viewer account with devices and subscriptions (id is derived from platform user id)

**Off-Chain (Database + TEE):**
- Sessions (high-throughput, stored in PostgreSQL)
- Session data in TEE (encrypted, for verification)

## Components

1. **Backend API** (`src/api.rs`): Handles HTTP requests for opening sessions
2. **Sui Integration** (`src/sui.rs`): Queries Sui blockchain for streams and permissions (on-chain)
3. **Database** (`src/database.rs`): PostgreSQL for storing sessions only (off-chain, high-throughput)
4. **TEE Module** (`src/tee.rs`): Placeholder for Trusted Execution Environment
5. **Streaming Module** (`src/streaming.rs`): Placeholder for video streaming
6. **Client Script** (`client.sh`): Simulates app button click

## Setup

### 1. Install PostgreSQL

Make sure PostgreSQL is running on your machine:

```bash
# On Fedora/RHEL
sudo dnf install postgresql postgresql-server
sudo postgresql-setup --initdb
sudo systemctl start postgresql
sudo systemctl enable postgresql

# Create database and user
sudo -u postgres psql
CREATE DATABASE aava;
CREATE USER postgres WITH PASSWORD 'postgres';
GRANT ALL PRIVILEGES ON DATABASE aava TO postgres;
\q
```

### 2. Install Rust Dependencies

```bash
cargo build
```

### 3. Run Migrations

The migrations will run automatically when you start the server, but you can also run them manually:

```bash
# Install sqlx-cli if needed
cargo install sqlx-cli --no-default-features --features postgres

# Run migrations
sqlx migrate run
```

### 4. Start the Server

```bash
cargo run
```

The server will start on `http://localhost:3000`

### 5. Run the Client Script

In another terminal:

```bash
chmod +x client.sh
./client.sh
```

## API Endpoints

### POST `/api/permissions/check`

Check if a user has permission to access a stream.

**Request:**
```json
{
  "user_id": "sarah",
  "stream_id": "tech-talk-live"
}
```

**Response:**
```json
{
  "user_id": "sarah",
  "stream_id": "tech-talk-live",
  "has_permission": true,
  "permission_type": "Subscription"
}
```

### POST `/api/sessions/open`

Open a new session for a user to watch a stream.

**Request:**
```json
{
  "user_id": "sarah",
  "stream_id": "tech-talk-live"
}
```

**Response:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "sarah",
  "stream_id": "tech-talk-live",
  "status": "Created",
  "created_at": "2024-01-01T12:00:00Z"
}
```

**Note:** The `session_id` in the response is used by the 3rd party app's SDK for forensic watermarking. Video streaming is handled entirely within the app.

## Next Steps

1. Implement real TEE integration
2. Integrate Sui blockchain for permission checking
3. Set up actual video streaming infrastructure
5. Implement proof generation and session verification





