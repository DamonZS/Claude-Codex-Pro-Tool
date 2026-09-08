---
name: "⚡ Security Fix - Batch 2"
about: Medium-priority correctness issues
title: "[SECURITY] Batch 2: Correctness & Error Handling Fixes"
labels: security, P1, medium
assignees: ''
---

## 📋 Issue Summary

**Batch:** 2 - Correctness Issues  
**Priority:** P1 - Medium  
**Target Completion:** 1 week  
**Estimated Effort:** 20 hours

---

## 🔧 Issues to Fix

### 7. unwrap() Panic Risks (Medium)

**Files:** Multiple locations in `crates/claude-codex-pro-core/src/launcher.rs:1137` and others  
**Severity:** Medium

**Problem:**
- Extensive use of `.unwrap()` can cause panics in production
- TCP connection failures, file I/O errors lead to process crashes
- No graceful degradation

**Fix Strategy:**
1. Scan all `.unwrap()` usage with `rg "\.unwrap\(\)" --type rust`
2. Replace with proper error handling:
   - Use `?` for propagating errors
   - Use `unwrap_or_default()` for optional operations
   - Use `expect("clear message")` for invariants
3. Add Clippy lints: `#![warn(clippy::unwrap_used)]`

**Acceptance:**
- [ ] All `.unwrap()` replaced or justified
- [ ] Clippy checks pass
- [ ] No panics in stress testing

---

### 8. TCP Stream Resource Leaks (Medium)

**Files:** `crates/claude-codex-pro-core/src/launcher.rs`  
**Severity:** Medium

**Problem:**
- TCP streams not explicitly closed on error paths
- Long-running processes accumulate file descriptors
- Can hit OS limits after days of uptime

**Fix Strategy:**
1. Create RAII wrapper `ManagedTcpStream`
2. Or use `scopeguard::defer!` for cleanup
3. Ensure `shutdown()` called in all exit paths

**Acceptance:**
- [ ] All TCP streams use RAII pattern
- [ ] Long-running test shows stable fd count
- [ ] No fd leaks under error injection

---

### 9. Concurrent Access to Shared State (Medium)

**Files:** `crates/claude-codex-pro-core/src/settings.rs`  
**Severity:** Medium

**Problem:**
- Settings accessed from multiple threads without synchronization
- Race conditions can corrupt configuration
- Non-atomic read-modify-write sequences

**Fix Strategy:**
1. Introduce global settings singleton with `Arc<RwLock<BackendSettings>>`
2. Provide `read_settings()` and `write_settings()` wrappers
3. Ensure all modifications go through synchronized paths

**Acceptance:**
- [ ] All settings access goes through singleton
- [ ] Concurrent test shows no data races (TSan)
- [ ] Write-after-write doesn't lose updates

---

### 10. Error Context Loss (Medium)

**Files:** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`  
**Severity:** Medium

**Problem:**
- Errors converted to generic messages, losing context
- User sees "操作失败" without details
- Debugging production issues difficult

**Fix Strategy:**
1. Use `anyhow::Context` throughout error chain
2. Create `format_error_chain()` helper
3. Log full error chain, show user-friendly summary

**Acceptance:**
- [ ] All error returns include context
- [ ] Error logs show full chain
- [ ] User messages remain friendly

---

## 📦 Implementation Checklist

- [ ] Scan and replace all `.unwrap()` (#7)
- [ ] Implement `ManagedTcpStream` wrapper (#8)
- [ ] Create settings singleton (#9)
- [ ] Add `anyhow::Context` to error paths (#10)
- [ ] Run tests with ThreadSanitizer
- [ ] Run 24-hour stress test
- [ ] Code review
- [ ] Merge to main

---

## ⏱️ Timeline

**Week 1, Day 1-2:**
- [ ] unwrap() cleanup (8h)

**Week 1, Day 3:**
- [ ] TCP resource management (4h)

**Week 1, Day 4:**
- [ ] Settings synchronization (4h)

**Week 1, Day 5:**
- [ ] Error context (4h)

**Total:** 20 hours over 1 week
