# Data Model: OpenCode Deep Integration

**Date**: 2026-04-15 | **Branch**: `009-opencode-deep-integration`

---

## Entities

### OpenCodePaths (Rust struct)

| Field | Type | Description |
|---|---|---|
| `config_dir` | `PathBuf` | Resolved OpenCode config directory |
| `config_file` | `PathBuf` | `opencode.json` or `opencode.jsonc` |
| `plugin_dir` | `PathBuf` | `<config_dir>/plugins/` |
| `agents_file` | `PathBuf` | `<config_dir>/AGENTS.md` |
| `is_jsonc` | `bool` | Whether config uses JSONC format |

**Resolution order**: `$OPENCODE_CONFIG_DIR` → `dirs::config_dir()/opencode/` → `.opencode/`

---

### SetupResult (Rust struct)

| Field | Type | Description |
|---|---|---|
| `actions` | `Vec<SetupAction>` | List of actions performed |

### SetupAction (Rust struct)

| Field | Type | Description |
|---|---|---|
| `action` | `ActionKind` | Created / Updated / Skipped / Removed |
| `target` | `String` | File path or config key affected |
| `detail` | `String` | Human-readable description |

### ActionKind (Rust enum)

| Variant | Description |
|---|---|
| `Created` | New file or config entry |
| `Updated` | Modified existing file or config |
| `Skipped` | No change needed (already correct) |
| `Removed` | Deleted file or config entry |

---

### DoctorCheck (Rust enum)

| Variant | Fields | Description |
|---|---|---|
| `BinaryInPath` | — | `the-crab-engram` is in PATH |
| `OpencodeInstalled` | — | `opencode` binary exists |
| `ConfigExists` | `path: PathBuf` | `opencode.json` exists |
| `McpEntryValid` | — | MCP entry for the-crab-engram present |
| `PluginExists` | `path: PathBuf` | Plugin TS file exists |
| `ServerRunning` | — | `GET /health` responds OK |
| `DatabaseOk` | — | SQLite integrity check passes |

### CheckResult (Rust struct)

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Check display name |
| `status` | `CheckStatus` | Pass / Fail / Warn |
| `message` | `String` | Detail or error message |
| `fix_command` | `Option<String>` | Command to auto-fix |

### CheckStatus (Rust enum)

| Variant | Symbol | Description |
|---|---|---|
| `Pass` | `+` | Check passed |
| `Fail` | `x` | Check failed |
| `Warn` | `!` | Passed with warning |

---

## Plugin Runtime State (TypeScript, in-memory)

### EngramPluginState (TS interface)

| Field | Type | Description |
|---|---|---|
| `baseUrl` | `string` | `http://localhost:7437` |
| `sessionId` | `string \| null` | Current engram session ID |
| `projectName` | `string` | Project name for engram |
| `lastInjection` | `number` | Timestamp of last push injection |
| `pendingContext` | `string \| null` | Cached injection from chat.message |
| `serverHealthy` | `boolean` | Whether HTTP API is responding |

---

## Relationships

```
OpenCodePaths ──used by──> setup_opencode()
                       ──used by──> doctor_opencode()

SetupResult ──returned by──> setup_opencode()
                          ──returned by──> uninstall_opencode()

DoctorCheck ──dispatches──> CheckResult[]
```

---

## Validation Rules

1. **Config merge**: MUST preserve all existing keys in `opencode.json`
2. **JSONC parse**: Strip `//` line comments, preserve `/*` block comments
3. **Plugin hash**: Skip write if existing file has identical SHA-256
4. **AGENTS.md merge**: Find `<!-- engram-protocol-start -->` marker, replace block; if missing, append
5. **Doctor `--fix`**: Only auto-fix Fail status, not Warn
6. **Setup idempotency**: Running setup twice produces same result (Skipped on second run)
