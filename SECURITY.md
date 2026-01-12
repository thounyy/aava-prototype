# Security Architecture

## Redis Security

### Access Control

**Only the enclave has access to Redis session data.**

1. **Network Isolation**
   - Redis binds to `127.0.0.1` only (localhost)
   - No external network access
   - Firewall rules can further restrict access

2. **Authentication**
   - Redis requires password authentication
   - Password set via `requirepass` in `redis.conf`
   - Enclave authenticates using `REDIS_PASSWORD` env var

3. **Data Integrity**
   - Enclave generates cryptographic hash of session data
   - Enclave signs the hash with its private key
   - Hash + signature prove data integrity at attestation time
   - Any tampering before attestation will be detected

### Security Flow

```
1. Session Created
   └─> Enclave writes to Redis (authenticated)
   
2. Stream Ends
   └─> Enclave reads from Redis (authenticated)
   └─> Enclave generates SHA-256 hash
   └─> Enclave signs hash with enclave keypair
   └─> Returns: {sessions, hash, signature}
   
3. Backend Receives Attestation
   └─> Verifies signature matches enclave public key
   └─> Hash proves data integrity
   └─> Uploads to Walrus (permanent storage)
   └─> Publishes hash to Sui (on-chain proof)
   
4. Cleanup
   └─> After successful Walrus upload
   └─> Enclave deletes sessions from Redis
```

### Threat Model

**Protected Against:**
- ✅ Unauthorized Redis access (password required)
- ✅ Network-based attacks (localhost only)
- ✅ Data tampering (detected via hash mismatch)
- ✅ Replay attacks (signature includes timestamp)

**Not Protected Against (if compromised):**
- ❌ Root access to machine (can access Redis directly)
- ❌ Enclave compromise (would break entire security model)
- ❌ Redis process compromise (requires system-level access)

### Production Recommendations

1. **Redis Configuration**
   ```conf
   bind 127.0.0.1
   protected-mode yes
   requirepass <strong-password>
   maxmemory-policy allkeys-lru
   ```

2. **Network Security**
   - Run Redis in isolated network namespace
   - Use firewall rules to block all external access
   - Consider Redis over TLS (requires Redis 6.0+)

3. **Access Control**
   - Use Redis ACLs for fine-grained permissions
   - Create dedicated user for enclave with minimal permissions
   - Rotate passwords regularly

4. **Monitoring**
   - Monitor Redis access logs
   - Alert on authentication failures
   - Track session creation/deletion patterns

5. **Enclave Security**
   - Enclave runs in TEE (Trusted Execution Environment)
   - Cryptographic attestation proves enclave integrity
   - Private key never leaves enclave

## Data Lifecycle

1. **Creation**: Session written to Redis by enclave
2. **Active**: Session data in Redis (ephemeral, TTL: 24h)
3. **Attestation**: Enclave reads, hashes, and signs data
4. **Archive**: Data uploaded to Walrus (permanent storage)
5. **On-Chain**: Hash published to Sui blockchain
6. **Cleanup**: Session deleted from Redis

**Key Point**: Redis is temporary storage. Permanent storage is Walrus + Sui blockchain.

