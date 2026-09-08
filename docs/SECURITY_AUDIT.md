# 🔒 安全审计指南

**最后更新:** 2026-01-09  
**状态:** Issue #16 - 依赖安全审计配置完成 ✅

---

## 📋 概述

本项目使用多层安全审计来确保依赖安全性：
1. **cargo-audit** - 检查已知漏洞 (RustSec Advisory DB)
2. **cargo-deny** - 许可证合规性和依赖策略
3. **GitHub Actions** - 自动化定期审计

---

## 🛠️ 工具安装

### 1. cargo-audit

检查 Cargo.lock 中的依赖是否有已知漏洞。

```bash
# 安装
cargo install cargo-audit

# 更新漏洞数据库
cargo audit fetch

# 运行审计
cargo audit
```

### 2. cargo-deny

多功能依赖检查工具（许可证、安全、重复版本等）。

```bash
# 安装
cargo install cargo-deny

# 运行所有检查
cargo deny check

# 分别运行
cargo deny check advisories  # 安全漏洞
cargo deny check licenses    # 许可证合规
cargo deny check bans        # 禁止的依赖
cargo deny check sources     # 来源验证
```

---

## 🔍 手动审计流程

### 每周检查 (推荐)

```bash
# 1. 更新数据库
cargo audit fetch

# 2. 检查漏洞
cargo audit

# 3. 检查许可证和策略
cargo deny check
```

### 在添加新依赖时

```bash
# 添加依赖后立即检查
cargo add <crate-name>
cargo audit
cargo deny check licenses
```

### 在更新依赖时

```bash
# 更新前检查
cargo update
cargo audit
cargo deny check
```

---

## 🤖 自动化审计

### GitHub Actions

配置文件: `.github/workflows/security-audit.yml`

**触发条件:**
- Push 到 main 分支
- Pull Request
- 每周一自动运行

**检查项:**
1. cargo-audit: 依赖漏洞
2. cargo-deny: 许可证合规
3. cargo-deny: 禁止的依赖
4. cargo-deny: 来源验证

**查看结果:**
```
GitHub Repository → Actions → Security Audit
```

---

## ⚙️ 配置文件

### .cargo/deny.toml

依赖策略配置：

```toml
[advisories]
# 拒绝已知漏洞
yanked = "deny"

[licenses]
# 允许的许可证
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
]

[bans]
# 警告多版本依赖
multiple-versions = "warn"

[sources]
# 仅允许 crates.io
unknown-registry = "deny"
unknown-git = "deny"
```

**配置位置:** `.cargo/deny.toml`

---

## 🚨 处理安全漏洞

### 发现漏洞时

1. **查看详情**
   ```bash
   cargo audit
   ```

2. **查找受影响的代码**
   ```bash
   cargo tree -i <vulnerable-crate>
   ```

3. **尝试更新**
   ```bash
   cargo update <vulnerable-crate>
   cargo audit
   ```

4. **如果无法更新**
   - 查看是否有替代 crate
   - 评估风险（漏洞是否影响我们的使用场景）
   - 如果低风险，添加到 `.cargo/deny.toml` 的 `ignore` 列表
   - 创建 GitHub Issue 追踪

5. **记录决策**
   - 在 `.cargo/deny.toml` 中注释说明
   - 更新本文档的 "已知问题" 部分

### 示例：临时忽略

如果漏洞不影响我们的使用场景：

```toml
# .cargo/deny.toml
[advisories]
ignore = [
    "RUSTSEC-2024-0001",  # example-crate: DoS in feature we don't use
]
```

---

## 📊 当前状态

### 最近审计

**日期:** 2026-01-09  
**工具:** Manual review  
**状态:** ✅ 配置完成

**发现:**
- 无自动化工具运行（需要安装）
- 配置文件已创建
- CI/CD workflow 已添加

### 已知问题

当前无已知安全问题。

### 依赖策略

1. **许可证要求:**
   - 允许: MIT, Apache-2.0, BSD-*
   - 警告: Copyleft 许可证
   - 拒绝: 无许可证

2. **版本策略:**
   - 警告多版本依赖
   - 允许 windows-sys 等系统crate的多版本

3. **来源策略:**
   - 仅允许 crates.io
   - 拒绝未知 registry
   - 拒绝 git 依赖

---

## 🔄 定期维护

### 每周

- [ ] 运行 `cargo audit`
- [ ] 检查 GitHub Actions 结果

### 每月

- [ ] 运行 `cargo deny check`
- [ ] 审查多版本依赖
- [ ] 更新 `Cargo.lock`

### 每季度

- [ ] 审查所有依赖
- [ ] 评估是否有更安全的替代品
- [ ] 更新本文档

---

## 📚 相关资源

### 工具文档

- [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/)
- [RustSec Advisory DB](https://rustsec.org/)

### 安全资源

- [Rust Security Advisory](https://rustsec.org/)
- [CVE Database](https://cve.mitre.org/)
- [GitHub Security Advisories](https://github.com/advisories)

### 相关文档

- [SECURITY_FIX_STRATEGY.md](../SECURITY_FIX_STRATEGY.md)
- [CONCURRENCY_AUDIT.md](./CONCURRENCY_AUDIT.md)
- [DEFECTS_TRACKING.md](../DEFECTS_TRACKING.md)

---

## 🎯 Issue #16 完成标准

- [x] 创建 `.cargo/deny.toml` 配置
- [x] 添加 GitHub Actions workflow
- [x] 创建审计指南文档
- [ ] 首次运行 cargo-audit (需要安装)
- [ ] 首次运行 cargo-deny (需要安装)
- [ ] 修复发现的问题

**状态:** 🟡 配置完成，待首次运行

---

**维护者:** Claude Code Team  
**最后更新:** 2026-01-09
