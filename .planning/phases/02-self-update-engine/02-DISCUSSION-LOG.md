# Phase 2: Self-Update Engine - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-08
**Phase:** 02-self-update-engine
**Areas discussed:** CLI structure, repo config, backup, confirmation, error UX, database safety

---

## CLI Structure

| Option | Description | Selected |
|--------|-------------|----------|
| Top-level `update` | `the-crab-engram update` | |
| `self` namespace | `the-crab-engram self update` (uv pattern) | ✓ |

**User's choice:** Approved `self` namespace
**Notes:** uv sets the modern standard. Groups self-management commands logically.

---

## Repo Config

| Option | Description | Selected |
|--------|-------------|----------|
| Hardcoded only | Const in source code | |
| Hardcoded + env var override | Const + `CRAB_ENGRAM_UPDATE_REPO` env var | ✓ |
| Install receipt (uv pattern) | JSON file written by installer | |

**User's choice:** Hardcoded + env var override
**Notes:** No custom installer yet, so receipt system not needed. Env var for dev/testing.

---

## Binary Backup

| Option | Description | Selected |
|--------|-------------|----------|
| Persistent backup (original plan) | Save old binary before replacing | |
| No backup (atomic replace) | self_replace handles it safely | ✓ |

**User's choice:** No backup — atomic replace is sufficient
**Notes:** Research showed uv and self_update both use self_replace without backup. Atomic rename on Unix is filesystem-safe. UPDATE-04 requirement changed to --dry-run.

---

## Confirmation

| Option | Description | Selected |
|--------|-------------|----------|
| Interactive prompt [Y/n] | self_update default | |
| No prompt + --dry-run | uv pattern | ✓ |

**User's choice:** No prompt, --dry-run for preview
**Notes:** Modern CLI convention. User already expressed intent by running the command.

---

## Error UX

| Option | Description | Selected |
|--------|-------------|----------|
| Raw error strings | self_update default | |
| Colored + actionable messages | uv pattern | ✓ |

**User's choice:** Colored structured errors with recovery hints
**Notes:** Handle network, 403, 404, checksum, already-latest cases.

---

## Database Safety (CRITICAL)

| Option | Description | Selected |
|--------|-------------|----------|
| Discuss during backup discussion | Mixed binary + database backup | |
| Separate concern — never touch DB | Binary-only update, DB completely isolated | ✓ |

**User's choice:** Self-update must NEVER touch the SQLite database
**Notes:** User explicitly raised concern. The database IS the product. Self-update only replaces the binary executable. Pinned as critical decision in engram.

---

## OpenCode's Discretion

- Exact error message wording
- Color codes for terminal output
- Env var name (`CRAB_ENGRAM_UPDATE_REPO` vs `THE_CRAB_ENGRAM_REPO`)

## Deferred Ideas

- Install receipt system — add when custom installer exists
- `--token` flag for private repos — public only for now
- Rollback to previous version — add if users request
- Async background update check — Phase 8 territory
- Signature verification (zipsign) — after initial implementation works
