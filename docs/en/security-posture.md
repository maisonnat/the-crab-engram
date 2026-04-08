# The Crab Engram — Security Posture

## Encryption

### Database Encryption

The Crab Engram uses **ChaCha20-Poly1305** (AEAD cipher) for database-level encryption:

```rust
// Key derivation: SHA-256(passphrase + "engram-salt-v1")
let key = engram_core::derive_key("my-passphrase");

// Encrypt: generates random 12-byte nonce, prepends to ciphertext
let encrypted = engram_core::encrypt(&key, &db_bytes)?;

// Decrypt: extracts nonce from first 12 bytes
let decrypted = engram_core::decrypt(&key, &encrypted)?;
```

**Properties:**
- Authenticated encryption (detects tampering)
- Random nonce per encryption (same plaintext → different ciphertext)
- Nonce prepended to ciphertext (self-contained)
- SQLite header detection (auto-detects encrypted vs plaintext)

**Usage:**
```bash
the-crab-engram encrypt --passphrase "my-secret"
# → Creates engram.encrypted.db (or engram.decrypted.db if already encrypted)
```

### Key Derivation

Current: SHA-256 with static salt (`engram-salt-v1`)

> **Note:** Production deployments should use Argon2id for key derivation. The current SHA-256 approach is functional but not memory-hard.

---

## Multi-Agent Permissions

### Access Levels

| Level | Can Read | Can Write | Can Admin |
|---|---|---|---|
| `Read` | ✅ | ❌ | ❌ |
| `Write` | ✅ | ✅ | ❌ |
| `Admin` | ✅ | ✅ | ✅ |

### MCP Tool Profiles

| Profile | Tools Available | Use Case |
|---|---|---|
| `Agent` | 27 tools (no delete/stats/timeline/merge) | Default for AI agents |
| `Admin` | 4 tools only (delete/stats/timeline/merge) | Administrative operations |
| `All` | 31 tools | Full access (development) |

Delete operations require Admin profile:
```
mem_delete(profile=Agent) → "delete requires Admin profile"
```

### Permission Engine

Per-agent, per-project access control:

```rust
let mut engine = PermissionEngine::new();
engine.add_rule(PermissionRule::new("agent-a", "proj1", AccessLevel::Write));
engine.add_rule(PermissionRule::new("agent-b", "proj1", AccessLevel::Read));

engine.check("agent-a", "proj1", AccessLevel::Write)  // true
engine.check("agent-b", "proj1", AccessLevel::Write)  // false
```

---

## Code Safety

### No Unsafe Code

The codebase uses **zero `unsafe` blocks**. All memory safety is guaranteed by Rust's borrow checker.

### No Secrets in Codebase

- No API keys, tokens, or credentials in source code
- Database passphrase is passed via CLI argument (not hardcoded)
- Salt is a constant string (`engram-salt-v1`) — not a secret, used for key derivation domain separation

### Dependency Audit

Key security-relevant dependencies:

| Crate | Purpose | Notes |
|---|---|---|
| `chacha20poly1305` v0.10 | AEAD encryption | Well-audited, RustCrypto |
| `sha2` v0.10 | Hashing (dedup + key derivation) | Well-audited, RustCrypto |
| `rusqlite` v0.35 | SQLite (bundled) | No external SQLite dependency |
| `rmcp` v1.3 | MCP protocol | stdio transport only (no network) |
| `tokio` v1 | Async runtime | Industry standard |
| `serde` v1 | Serialization | Industry standard |

### SQLite Configuration

```sql
PRAGMA journal_mode = WAL;       -- Write-Ahead Logging
PRAGMA busy_timeout = 5000;      -- 5s lock wait
PRAGMA synchronous = NORMAL;     -- Balanced durability/performance
PRAGMA foreign_keys = ON;        -- Referential integrity
```

### Data Isolation

- **Project isolation:** Observations are scoped by `project` field
- **Personal scope:** `Scope::Personal` observations are isolated per agent
- **Session binding:** Every observation must belong to a session
- **Deduplication:** SHA-256 normalized_hash prevents duplicate storage

---

## Attack Surface

### MCP stdio Transport

- Communication over stdin/stdout (no network exposure)
- JSON-RPC protocol with strict schema validation
- Unknown tool names rejected with `invalid_params` error
- Profile-based tool filtering

### HTTP API

- Listens on `0.0.0.0` (all interfaces) — **bind to localhost in production**
- CORS fully permissive — **restrict origins in production**
- No authentication layer — **add auth middleware in production**
- No rate limiting — **add rate limiter in production**

### CLI

- Database path is user-controlled (`--db` flag)
- File operations use standard Rust `std::fs` (no shell execution)
- No subprocess spawning except for configured agent setup

---

## Recommendations for Production

1. **Use Argon2id** for key derivation instead of SHA-256
2. **Bind HTTP API to localhost** unless reverse-proxied
3. **Add authentication** to HTTP API (JWT, API keys)
4. **Restrict CORS** to specific origins
5. **Add rate limiting** to HTTP endpoints
6. **Use OS-level file permissions** on `~/.engram/engram.db`
7. **Rotate encryption passphrases** periodically
8. **Audit SQLite file** permissions (should be `0600`)
