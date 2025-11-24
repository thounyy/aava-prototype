# Architecture Overview

## Data Storage Strategy

### On-Chain (Sui Blockchain)
- **Stream Objects**: Metadata, permission types, pricing
- **Subscriptions**: User subscription objects tied to streams
- **Payments**: Pay-per-view payment records

**Why on-chain?**
- Low update frequency
- Need for trustless verification
- Immutable record of ownership and permissions

### Off-Chain (PostgreSQL Database)
- **Sessions**: High-throughput session data

**Why off-chain?**
- Millions of sessions created per second
- Too expensive to store on-chain
- Can be verified later through batch proofs (Nautilus)

### TEE (Trusted Execution Environment)
- **Session State**: Encrypted session data during active streaming
- **Ad Rendering**: Secure ad processing and verification

**Why TEE?**
- Cryptographic proof of ad rendering
- Privacy-preserving session management
- Tamper-resistant verification

## Flow Diagram

```
┌─────────────┐
│   User      │
│  (client)   │
└──────┬──────┘
       │
       │ POST /api/sessions/open
       ▼
┌─────────────────┐
│   Backend API   │
└──────┬───────────┘
       │
       │ Check permissions
       ▼
┌─────────────────┐      ┌──────────────────┐
│  Sui Blockchain │◄─────┤  Query Stream    │
│  (On-Chain)     │      │  & Subscription  │
└─────────────────┘      └──────────────────┘
       │
       │ Permission granted
       ▼
┌─────────────────┐
│   TEE Cluster   │
│  Create Session │
└──────┬──────────┘
       │
       │ Session created
       ▼
┌─────────────────┐      ┌──────────────────┐
│   PostgreSQL    │◄─────┤  Store Session   │
│   (Off-Chain)   │      │ (High Throughput)│
└─────────────────┘      └──────────────────┘
       │
       │ Start streaming
       ▼
┌─────────────────┐
│  Streaming      │
│  Infrastructure │
└─────────────────┘
```

## Permission Check Flow

1. **User requests session** → Backend receives request
2. **Query Sui blockchain** → Get stream object and check permission type
3. **Check subscription/payment** → Query on-chain subscription or payment objects
4. **Return permission status** → Grant or deny access
5. **If granted** → Create session in TEE and database

## Session Lifecycle

1. **Created**: Session object created in TEE, stored in database
2. **Active**: Streaming has started
3. **Streaming**: Video is actively being delivered
4. **Completed**: Session ended, ready for batch verification
5. **Verified**: Batch proof generated and verified on-chain (future)

## Future: Batch Verification with Nautilus

```
Millions of Sessions (Off-Chain)
         │
         ▼
┌─────────────────┐
│  Nautilus       │
│  Batch Proof    │
└──────┬──────────┘
       │
       │ Single ZK Proof
       ▼
┌─────────────────┐
│  Sui Blockchain │
│  Verify Proof   │
└─────────────────┘
```

This allows millions of sessions to be verified with a single on-chain transaction.

