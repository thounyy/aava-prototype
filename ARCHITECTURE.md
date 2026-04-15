# Aava — Architecture

**Audience:** engineers and AI agents working in this repo. This document is the source of truth for **what runs where**, **trust boundaries**, and **main request flows**.

---

## 1. Protocol split (conceptual)

| Layer | What | Why |
|--------|------|-----|
| **Sui (L1)** | Streams, creator/viewer accounts, blob registration (`verify_and_store_blob`), certify/destroy blob, subscriptions/permissions as modeled in Move | Low-frequency, trustless state and settlement handles |
| **Walrus** | Large payloads: batched session JSON uploaded after attestation | Cheap blob storage; Sui holds object ids / commitments |
| **TEE / enclave service** | Ephemeral session rows, stream-end dataset assembly, signing over blob metadata | High throughput; only this tier should own session store + signing keys for dataset attestation |
| **Backend (“session engine”)** | Public HTTP API, Sui tx building/signing (operator wallet), Walrus relay upload | Stateless orchestrator; **does not** implement the session database |

**Session throughput** stays off-chain. **Proofs and handles** land on Sui; **bulk data** lands on Walrus.

---

## 2. Repository layout

| Path | Crate / role |
|------|----------------|
| `backend/` | **`session_engine`** — Axum API, `sui` + `walrus` modules, HTTP client to enclave (`enclave/` module) |
| `enclave/` | **`session_enclave`** — Axum service bound to **:3000**, Redis, session/stream handlers, Ed25519 signing for stream-end payloads |
| `contract/` | **`aava`** — Move package living on Sui, viewer and creator accounts, stream lifecycle and blob registration |

Ephemeral session state lives in **Redis**, accessed **only from the enclave process** (by design).

---

## 3. Runtime diagram

```
┌──────────────┐
│ Client / app │
└──────┬───────┘
       │ HTTPS (e.g. :8080)
       ▼
┌──────────────────────────────┐
│  backend (session_engine)    │  AppState: sui_rpc::Client only (stateless)
│  - /api/creators, viewers,   │
│    sessions, streams, actions│
│  - /openapi.json             │
└──────┬───────────────────────┘
       │ ENCLAVE_URL + X-Internal-Token
       ▼
┌──────────────────────────────┐
│  enclave (session_enclave)   │  :3000
│  /internal/sessions/*        │  Redis (REDIS_URL)
│  /internal/streams/end       │
│  /internal/streams/cleanup   │
└──────┬───────────────────────┘
       │
       ▼
┌──────────────┐
│    Redis     │  Keys: session:*, stream:*:sessions (conceptually)
└──────────────┘

       backend also ──► Sui RPC (testnet in code) ──► Move txs
                    ──► Walrus (blob upload / relay) after register tx
```

---

## 4. Trust boundaries

1. **Backend** must **not** connect to Redis for session data in production-shaped designs; **only the enclave** should read/write session keys so attested datasets match what was actually stored.
2. **Internal auth:** enclave routes under `/internal/*` require header **`X-Internal-Token`** matching env **`ENCLAVE_INTERNAL_TOKEN`** (set on both processes).
3. **Stream end:** enclave returns **signed** dataset metadata (blob id, hashes, sizes, session list); backend registers the blob on Sui, uploads bytes to Walrus, then certifies or destroys the blob on failure.

---

## 5. Main flows

### 5.1 Open / close session (simplified)

1. Client calls backend **`POST /api/sessions/open`** (see OpenAPI) with viewer + stream identifiers.
2. Backend validates business rules (and may read Sui for permissions — implementation evolves).
3. Backend calls enclave **`POST {ENCLAVE_URL}/internal/sessions/open`** with **`X-Internal-Token`**.
4. Enclave writes session record to Redis (and indexes by stream as needed).
5. Close / warn / revoke / get follow the same pattern under **`/internal/sessions/*`**.

### 5.2 Stream end (orchestration — `backend/src/api/streams.rs`)

Order of operations:

1. Read Walrus system params from Sui (pricing, shard count).
2. **`fetch_signed_dataset`** from enclave: enclave loads all sessions for `stream_id`, builds payload, returns **signature** + **timestamp** + structured **blob metadata** (blob id, root hash, encoding, sizes, session count).
3. If session count is 0, short-circuit (no Walrus/Sui blob path).
4. Build and execute **`verify_and_store_blob`** (init tx) with enclave attestation fields.
5. Upload JSON payload to **Walrus** via **`walrus::blob::upload_dataset`** (relay).
6. **`certify_blob`** on success, or **`destroy_blob`** if upload fails after init.
7. **`cleanup_dataset`** on enclave to remove Redis keys for that stream.

The operator wallet used for signing Sui transactions is defined by **`sui::executor`** (see logs at startup).

---

## 6. Enclave surface (reference)

Public / diagnostic:

- `GET /` — ping  
- `GET /attestation` — attestation handler  
- `GET /health_check`  

Internal (require **`X-Internal-Token`**):

- `POST /internal/sessions/open`, `close`, `warn`, `revoke`, `get`  
- `POST /internal/streams/end`, `cleanup`  

Implementation: `enclave/src/sessions.rs`, `enclave/src/streams.rs`, `enclave/src/handlers.rs`.

---

## 7. Backend modules (reference)

| Module | Responsibility |
|--------|------------------|
| `api::creator`, `api::viewer`, `api::sessions`, `api::streams`, `api::actions` | HTTP routes |
| `sui::*` | RPC reads, tx building, executor (sign + execute) |
| `walrus::*` | Blob upload, tips |
| `enclave::session`, `enclave::stream` | reqwest client to enclave |

---

## 8. Configuration (dev-oriented)

| Variable | Where | Purpose |
|----------|--------|---------|
| `ENCLAVE_URL` | backend | Default `http://localhost:3000` |
| `ENCLAVE_INTERNAL_TOKEN` | backend + enclave | Shared secret for `/internal/*` |
| `REDIS_URL` | enclave | e.g. `redis://localhost:6379` |
| `SESSION_ENGINE_HOST`, `SESSION_ENGINE_PORT` | backend | Bind address (default `127.0.0.1:8080`) |

**Ports:** backend **8080**, enclave **3000** (see `backend/src/main.rs`, `enclave/src/main.rs`).

---

## 9. Production notes (GCP-style, optional)

- Run **backend** on stateless compute (e.g. Cloud Run / GKE); run **enclave** on **Confidential VMs** if you want hardware-backed isolation; keep **Redis** (e.g. Memorystore) on **private IPs** reachable only from the enclave tier; use **Secret Manager** for tokens.
- **Indexer (optional):** a separate pipeline indexing Sui object changes and blob ids into a query store (BigQuery, Cloud SQL, Elasticsearch, etc.) makes “find the Walrus blob for stream X” and dashboards cheap; **Sui stays authoritative** for verification.
