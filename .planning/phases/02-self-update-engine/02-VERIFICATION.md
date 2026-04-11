---
phase: 02-self-update-engine
verified: 2026-04-11T00:00:00Z
status: gaps_found
score: 6/7 must-haves verified
gaps:
  - truth: "Downloaded binary is verified against checksums-sha256.txt (SHA-256)"
    status: partial
    reason: "SHA-256 checksum verification is a placeholder. Code computes binary hash and downloads checksums-sha256.txt, but never compares them. TODO comment at line 717: 'parse and verify against appropriate entry'."
    artifacts:
      - path: "src/main.rs"
        issue: "Lines 692-730: hash computed, checksums file downloaded, but no parsing or comparison logic exists. The block logs 'Retrieved checksums file' and returns Ok(()) without verification."
    missing:
      - "Parse checksums-sha256.txt to find the entry matching current binary name"
      - "Compare computed_hash_hex against the expected hash from the file"
      - "Exit with error + actionable message if mismatch"
      - "Consider adding sha2 crate to Cargo.toml if not already present (it IS present — only comparison logic missing)"
human_verification: []
---

# Phase 02: Self-Update Engine Verification Report

**Phase Goal:** User can update the binary in-place with a single command, safely
**Verified:** 2026-04-11
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| #   | Truth                                                                     | Status      | Evidence                                                                                       |
| --- | ------------------------------------------------------------------------- | ----------- | ---------------------------------------------------------------------------------------------- |
| 1   | User can run `the-crab-engram self update` and get the latest release     | ✓ VERIFIED  | `SelfAction::Update` variant at line 200, `handle_self_update()` at line 624, `Update::configure()` builder at line 660 |
| 2   | User can run `--check-only` to see available version without downloading  | ✓ VERIFIED  | Lines 642-645: `if check_only` guard prints version comparison via `eprintln!` and returns early |
| 3   | User can run `--dry-run` to see what would happen without downloading     | ✓ VERIFIED  | Lines 648-651: `if dry_run` guard prints preview via `eprintln!` and returns early              |
| 4   | Downloaded binary is verified against checksums-sha256.txt (SHA-256)      | ✗ PARTIAL   | Hash computed (line 695-701), checksums file downloaded (line 708), but NO comparison. TODO at line 717: "parse and verify against appropriate entry" |
| 5   | Binary size is verified after update (Windows 0-byte bug)                 | ✓ VERIFIED  | Lines 683-690: `metadata(&exe)?.len() == 0` check with reinstall instructions and `exit(1)`   |
| 6   | Update uses self_update with rustls — no OpenSSL dependency               | ✓ VERIFIED  | Cargo.toml line 63: `self_update = { version = "0.44.0", features = ["reqwest", "rustls", ...], default-features = false }` |
| 7   | Self-update NEVER touches the SQLite database                             | ✓ VERIFIED  | `handle_self_update` (lines 624-733) has zero references to `open_store`, `SqliteStore`, or `.db` |

**Score:** 6/7 must-haves verified (1 partial)

### Required Artifacts

| Artifact          | Expected                                                                  | Status      | Details                                                                                   |
| ----------------- | ------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------------- |
| `Cargo.toml`      | self_update v0.44.0 with rustls + archive features, no OpenSSL            | ✓ VERIFIED  | Line 63: `version = "0.44.0"`, features include `reqwest`, `rustls`, `archive-tar`, `archive-zip`, `compression-flate2`, `compression-zip-deflate`, `default-features = false` |
| `Cargo.toml`      | reqwest with blocking + rustls (for checksum download)                     | ✓ VERIFIED  | Line 62: `reqwest = { version = "0.13", features = ["blocking", "rustls"], default-features = false }` |
| `Cargo.toml`      | sha2 dependency (for checksum computation)                                | ✓ VERIFIED  | Line 61: `sha2 = { workspace = true }` — workspace specifies `sha2 = "0.10"` at line 27   |
| `src/main.rs`     | `enum SelfAction` with Update and Version variants                        | ✓ VERIFIED  | Lines 197-210                                                                              |
| `src/main.rs`     | `Commands::Self_` with `#[command(name = "self")]`                        | ✓ VERIFIED  | Lines 166-171                                                                              |
| `src/main.rs`     | `fn handle_self_update` function                                          | ✓ VERIFIED  | Lines 624-733                                                                              |
| `src/main.rs`     | `UPDATE_REPO_OWNER`, `UPDATE_REPO_NAME`, `BIN_NAME` constants             | ✓ VERIFIED  | Lines 14-18: values `"maisonnat"`, `"the-crab-engram"`, `"the-crab-engram"`                |

### Key Link Verification

| From                             | To                                          | Via                                    | Status      | Details                                                              |
| -------------------------------- | ------------------------------------------- | -------------------------------------- | ----------- | -------------------------------------------------------------------- |
| `Commands::Self_` match arm      | `handle_self_update()`                      | direct function call                   | ✓ WIRED     | Line 611: `handle_self_update(check_only, dry_run)?`                 |
| `handle_self_update()`           | `self_update::backends::github::Update`     | builder pattern `.configure().build()?.update()` | ✓ WIRED     | Lines 660-669: `Update::configure()...build()...update()`            |
| `handle_self_update()`           | `std::fs::metadata` (size check)            | post-update binary size verification   | ✓ WIRED     | Lines 683-684: `std::env::current_exe()` + `std::fs::metadata(&exe)` |
| `handle_self_update()` output    | stderr                                      | `eprintln!`                            | ✓ WIRED     | All 10 eprintln! calls at lines 643-728; zero println! in this fn    |
| `handle_self_update()`           | SHA-256 computation                         | `sha2::Sha256` + `std::io::copy`       | ✓ WIRED     | Lines 694-701: file opened, hasher created, finalize called          |
| `handle_self_update()`           | Checksum file download                      | `reqwest_blocking::get`                | ✓ WIRED     | Line 708: `reqwest_blocking::get(&checksum_url)`                     |
| SHA-256 hash                     | checksums-sha256.txt comparison             | parsing + comparison                   | ✗ NOT WIRED | Line 717: `// TODO: parse and verify against appropriate entry`      |

### Data-Flow Trace (Level 4)

| Artifact               | Data Variable       | Source                                         | Produces Real Data | Status          |
| ---------------------- | ------------------- | ---------------------------------------------- | ------------------ | --------------- |
| `handle_self_update`   | `latest_version`    | `ReleaseList::configure()...fetch()` → GitHub API | Yes (live network) | ✓ FLOWING       |
| `handle_self_update`   | `computed_hash_hex` | `Sha256` of binary via `std::io::copy`          | Yes (actual file)  | ✓ FLOWING       |
| `handle_self_update`   | checksum comparison | TODO — never computed                           | N/A                | ✗ DISCONNECTED  |

### Requirements Coverage

| Requirement | Source Plan | Description                                              | Status      | Evidence                                                                                      |
| ----------- | ----------- | -------------------------------------------------------- | ----------- | --------------------------------------------------------------------------------------------- |
| UPDATE-01   | 02-01-PLAN  | User can update to latest release with `self update`     | ✓ SATISFIED | `handle_self_update()` calls `Update::configure()...update()`, lines 660-669                  |
| UPDATE-02   | 02-01-PLAN  | User can check for updates without downloading            | ✓ SATISFIED | `--check_only` flag at line 642, early return after `eprintln!` version comparison             |
| UPDATE-03   | 02-01-PLAN  | System verifies SHA-256 checksum                         | ✗ BLOCKED   | Hash computed + checksum file downloaded, but comparison is TODO (line 717)                   |
| UPDATE-04   | 02-01-PLAN  | User can preview update with `--dry-run`                 | ✓ SATISFIED | `--dry_run` flag at line 648, early return after `eprintln!` preview                          |
| UPDATE-05   | 02-01-PLAN  | System verifies binary size after update                 | ✓ SATISFIED | Lines 683-690: `metadata.len() == 0` → error message + `exit(1)`                              |
| UPDATE-06   | 02-01-PLAN  | Update uses `self_update` crate with `rustls`            | ✓ SATISFIED | Cargo.toml line 63: `default-features = false`, features include `"rustls"`                   |

### Anti-Patterns Found

| File          | Line | Pattern                           | Severity  | Impact                                                                |
| ------------- | ---- | --------------------------------- | --------- | --------------------------------------------------------------------- |
| `src/main.rs` | 717  | `// TODO: parse and verify...`    | 🛑 Blocker | SHA-256 checksum verification is non-functional — downloaded binary is NOT integrity-checked |

### Human Verification Required

None — all checks are programmatically verifiable.

### Gaps Summary

**1 gap found — UPDATE-03 SHA-256 checksum verification is incomplete.**

The implementation has the right *ingredients*:
- Binary SHA-256 is computed correctly (lines 694-701)
- `checksums-sha256.txt` is downloaded from the release (lines 705-708)
- The response is read into `body` (line 713)

But the critical *comparison* is missing. Line 717 has `// TODO: parse and verify against appropriate entry` followed by just logging the file size and returning `Ok(())`. This means:

1. An attacker who tampers with the downloaded binary would NOT be detected
2. A corrupted download would NOT trigger an error
3. The function silently succeeds even if the binary is compromised

**What's needed:**
- Parse `body` (the checksums file content) to find the line matching the current binary name (e.g., `the-crab-engram-{target}.tar.gz` or the extracted binary name)
- Extract the expected SHA-256 hash from that line
- Compare `computed_hash_hex` against the expected hash
- If mismatch: print actionable error ("Download failed integrity check. Try again or download manually from {url}") and `std::process::exit(1)`

This is a **security blocker** — UPDATE-03 (tampering mitigation per threat model T-02-01) is not achieved until the comparison logic is implemented.

---

_Verified: 2026-04-11_
_Verifier: OpenCode (gsd-verifier)_
