# 🔍 剩余待修复问题清单

**更新时间:** 2026-01-09  
**总进度:** 7/16 已修复 (43.75%)

---

## ✅ 已完成 (7/16)

### Batch 1: Critical Security Fixes (6/6) ✅

| # | 问题 | 严重性 | 状态 | 提交 |
|---|------|--------|------|------|
| 1 | PowerShell 命令注入 | Critical | ✅ Done | e7401fb |
| 2 | 路径遍历攻击 | High | ✅ Done | e7401fb |
| 3 | Windows 命令转义缺陷 | High | ✅ Done | e7401fb |
| 4 | Session ID 路径注入 | High | ✅ Done | 09ee4f8 |
| 5 | API 密钥日志泄漏 | High | ✅ Done | 09ee4f8 |
| 6 | 设置文件竞争条件 | Medium | ✅ Done | 09ee4f8 |

### Batch 2: Correctness Issues (1/4) ✅

| # | 问题 | 严重性 | 状态 | 提交 |
|---|------|--------|------|------|
| 7 | unwrap() Panic Risks | Medium | 🔄 In Progress (0.25%) | 7805b49 |

---

## 🔄 进行中 (1/16)

### Issue #7: unwrap() Panic Risks

**状态:** Clippy 防护已添加 ✅，清理进行中 🔄

**完成部分:**
- [x] 添加 Workspace Clippy Lints (防止新增)
- [x] 创建清理计划文档
- [x] 修复关键正则表达式 unwrap() (4个)

**待完成部分:**
- [ ] commands.rs JSON 序列化 unwrap() (~20个)
- [ ] settings.rs 关键路径 unwrap() (~151个)
- [ ] claude_zh_patch.rs (~153个)
- [ ] multica.rs (~136个)
- [ ] plugin_hub.rs (~131个)
- [ ] memory_assist.rs (~100个)

**总量:** 1594 个生产代码 unwrap() 待清理  
**进度:** 0.25% (4/1594 fixed)  
**预计完成:** 分阶段，3 个月持续清理

**相关文档:** [docs/UNWRAP_CLEANUP_PLAN.md](./UNWRAP_CLEANUP_PLAN.md)

---

## ⏳ 待开始 (9/16)

### Batch 2: Correctness Issues (3/4 待开始)

#### Issue #8: Error Context Loss ⏳

**严重性:** Medium  
**预计工作量:** 4 小时  
**优先级:** P1

**问题描述:**
- 错误转换为泛型消息，丢失上下文
- 用户看到"操作失败"但无详细信息
- 调试生产问题困难

**修复策略:**
1. 使用 `anyhow::Context` 在整个错误链中
2. 创建 `format_error_chain()` 辅助函数
3. 日志记录完整错误链，显示用户友好摘要

**文件范围:**
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `crates/claude-codex-pro-core/src/*.rs` (选择性)

---

#### Issue #9: Input Validation Gaps ⏳

**严重性:** Medium  
**预计工作量:** 4 小时  
**优先级:** P1

**问题描述:**
- 许多命令参数缺少验证
- 不受信任的字符串直接使用
- 可能导致意外行为

**修复策略:**
1. 创建验证库 crate `codex-validation`
2. 验证器：
   - `validate_port(u16) -> Result<u16>`
   - `validate_path(String) -> Result<PathBuf>`
   - `validate_url(String) -> Result<Url>`
3. 应用到所有 Tauri 命令

**当前状态:**
- ✅ Session ID 验证已完成 (DeleteClaudeSessionRequest)
- ✅ Session ID 验证已完成 (LoadClaudeSessionContextRequest)
- ⏳ 其他请求结构体待审查

**待审查的请求结构体:**
- `ClaudeDesktopDraftRequest` (text: String) - 已有空检查 ✅
- `LaunchRequest`
- `MulticaConnectionSaveRequest`
- `SystemPromptUrlRequest`
- `SaveRelayFileRequest`

---

#### Issue #10: Concurrent Access to Shared State ⏳

**严重性:** Medium  
**预计工作量:** 8 小时  
**优先级:** P1

**问题描述:**
- Settings 从多个线程访问但没有同步
- 竞争条件可能损坏配置
- 非原子的读-修改-写序列

**修复策略:**
1. 引入全局 settings 单例 `Arc<RwLock<BackendSettings>>`
2. 提供 `read_settings()` 和 `write_settings()` 包装器
3. 确保所有修改通过同步路径

**当前状态:**
- ✅ SettingsStore 已有文件锁 + Mutex 保护
- ✅ `save()` 和 `update()` 使用 SettingsFileLock
- ✅ Atomic file write (direct_write) 已实现
- ⏳ 需要审查多线程访问模式

**评估结果:**
- 当前实现已经有较好的并发保护
- 主要问题已在 Issue #6 中解决（原子文件写入）
- **可能降低优先级或标记为已完成** ✅

---

### Batch 3: Security Hardening (6/6 待开始)

#### Issue #11: Sensitive Data in Memory 🔒

**严重性:** Low  
**预计工作量:** 8 小时  
**优先级:** P2

**问题描述:**
- API 密钥在内存中以明文存储
- 进程转储可能泄漏密钥
- 密钥未被安全擦除

**修复策略:**
1. 使用 `zeroize` crate 安全擦除密钥
2. 在 `Drop` 时清零敏感字段
3. 考虑使用系统密钥环 (Windows Credential Manager)

**影响范围:**
- `BackendSettings` 中的 `api_key` 字段
- `RelayProfile` 结构体
- 临时密钥变量

---

#### Issue #12: Insufficient Logging for Security Events 📊

**严重性:** Low  
**预计工作量:** 3 小时  
**优先级:** P2

**问题描述:**
- 缺少安全相关操作的日志
- 没有权限提升的审计跟踪
- 难以调查安全事件

**修复策略:**
1. 定义安全事件分类
2. 添加结构化日志：`log_security_event(event, details)`
3. 记录事件：
   - 管理员权限请求
   - 文件权限更改
   - 配置修改

**接受标准:**
- 所有安全事件已记录
- 日志包含时间戳、用户、操作
- 配置日志轮转

---

#### Issue #13: Hardcoded Defaults 🔧

**严重性:** Low  
**预计工作量:** 2 小时  
**优先级:** P2

**问题描述:**
- 硬编码的超时、重试次数
- 配置不灵活
- 难以适应不同环境

**修复策略:**
1. 将常量移至配置文件
2. 提供环境变量覆盖
3. 文档化所有可配置选项

**范围:**
- `CLAUDE_ZH_PATCH_ELEVATED_TIMEOUT`
- `REPAIR_CODEX_FRONTEND_TIMEOUT`
- 其他超时常量

---

#### Issue #14: Clipboard Security 📋

**严重性:** Low  
**预计工作量:** 4 小时  
**优先级:** P2

**问题描述:**
- 敏感数据可能残留在剪贴板
- 其他应用可以读取剪贴板
- 没有清理机制

**修复策略:**
1. 在粘贴操作后清理剪贴板（可选）
2. 添加"清除剪贴板"功能
3. 警告用户剪贴板安全性

**范围:**
- `paste_claude_desktop_draft()`
- API 密钥复制操作

---

#### Issue #15: Network Timeouts ⏱️

**严重性:** Low  
**预计工作量:** 3 小时  
**优先级:** P2

**问题描述:**
- 网络请求可能无限期挂起
- 没有统一的超时策略
- 用户体验差

**修复策略:**
1. 为所有网络操作设置合理超时
2. 区分连接超时和读取超时
3. 提供重试机制

**范围:**
- TCP 连接
- HTTP 请求
- WebSocket 连接

---

#### Issue #16: Dependency Security Audit 📦

**严重性:** Low  
**预计工作量:** 5 小时  
**优先级:** P2

**问题描述:**
- 依赖版本未固定
- 供应链攻击风险
- 可重现性问题

**修复策略:**
1. 固定所有依赖到确切版本
2. 启用 `cargo deny` 进行许可证和 CVE 检查
3. 每周 `cargo audit` 在 CI/CD 中

**接受标准:**
- 所有依赖已固定
- `cargo deny` 通过
- 自动化安全公告

---

## 📊 优先级矩阵

### 立即行动 (本周)

| # | 问题 | 严重性 | 工作量 | ROI |
|---|------|--------|--------|-----|
| 7 | unwrap() 清理 | Medium | 持续 | ⭐⭐⭐⭐ |
| 8 | 错误上下文丢失 | Medium | 4h | ⭐⭐⭐⭐ |
| 9 | 输入验证差距 | Medium | 4h | ⭐⭐⭐⭐ |

### 下周 (Week 2)

| # | 问题 | 严重性 | 工作量 | ROI |
|---|------|--------|--------|-----|
| 10 | 并发访问 | Medium | 8h → 审查 | ⭐⭐⭐ |
| 11 | 内存敏感数据 | Low | 8h | ⭐⭐⭐ |
| 12 | 安全日志 | Low | 3h | ⭐⭐⭐⭐ |

### 两周后 (Week 3-4)

| # | 问题 | 严重性 | 工作量 | ROI |
|---|------|--------|--------|-----|
| 13 | 硬编码默认值 | Low | 2h | ⭐⭐ |
| 14 | 剪贴板安全 | Low | 4h | ⭐⭐ |
| 15 | 网络超时 | Low | 3h | ⭐⭐⭐ |
| 16 | 依赖审计 | Low | 5h | ⭐⭐⭐⭐⭐ |

---

## 🎯 完成标准

### Batch 2 完成标准

- [x] Issue #7: Clippy lints 添加 ✅
- [ ] Issue #7: 关键 unwrap() 清理（至少 50%）
- [ ] Issue #8: 错误上下文完整
- [ ] Issue #9: 所有用户输入已验证
- [ ] Issue #10: 并发访问模式已审查

### Batch 3 完成标准

- [ ] Issue #11: 敏感数据 zeroize
- [ ] Issue #12: 安全事件日志
- [ ] Issue #13: 配置外部化
- [ ] Issue #14: 剪贴板清理
- [ ] Issue #15: 网络超时设置
- [ ] Issue #16: 依赖固定 + cargo deny

---

## 📈 工作量估算

### 已完成

```
Batch 1: 6/6 issues
实际工作量: ~10 小时
原计划: 15 小时
效率: 150%
```

### 剩余工作量

```
Batch 2:
- Issue #7 (unwrap): 60 小时 (持续)
- Issue #8: 4 小时
- Issue #9: 4 小时
- Issue #10: 审查 (可能 0-2 小时)
小计: ~70 小时

Batch 3:
- Issue #11: 8 小时
- Issue #12: 3 小时
- Issue #13: 2 小时
- Issue #14: 4 小时
- Issue #15: 3 小时
- Issue #16: 5 小时
小计: 25 小时

总计: ~95 小时剩余
```

### 完成时间表

**乐观估计:** 3-4 周  
**现实估计:** 6-8 周（包含 unwrap() 持续清理）  
**保守估计:** 3 个月（完整 unwrap() 清理）

---

## 🔗 相关文档

- [SECURITY_FIX_STRATEGY.md](../SECURITY_FIX_STRATEGY.md) - 整体策略
- [SECURITY_FIXES_STATUS.md](./SECURITY_FIXES_STATUS.md) - 实时进度
- [UNWRAP_CLEANUP_PLAN.md](./UNWRAP_CLEANUP_PLAN.md) - unwrap() 清理计划
- [.github/ISSUE_TEMPLATE/](../.github/ISSUE_TEMPLATE/) - GitHub Issue 模板

---

## 💡 建议

### 立即行动

1. **测试 Batch 1 修复** ⭐⭐⭐⭐⭐
   - 单元测试
   - 安全测试
   - 回归测试

2. **代码审查** ⭐⭐⭐⭐⭐
   - 至少 2 位审查者
   - 重点审查安全逻辑

3. **完成 Issue #8 和 #9** ⭐⭐⭐⭐
   - 错误上下文改进
   - 输入验证完善

### 本周目标

- ✅ Batch 1 测试验证
- ✅ 完成 Issue #8, #9
- 🔄 持续 unwrap() 清理（目标 20-50 个/天）

### 长期规划

- 🔄 Issue #7 持续清理（3 个月）
- ⏳ Batch 3 逐步完成（2-4 周）
- ⏳ 自动化安全扫描 CI/CD

---

**维护者:** Claude Code Team  
**最后更新:** 2026-01-09  
**下次审查:** 2026-01-16
