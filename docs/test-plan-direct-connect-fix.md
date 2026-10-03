# Claude Desktop 直连模式修复 - 测试计划

## 修复内容

**问题**：用户配置"直连"模式（关闭模型映射）后，Claude Desktop 配置文件仍写入本地代理地址而非供应商真实 URL。

**修复**：`plugin_hub.rs:3595-3608` 根据 `model_mapping_enabled` 字段决定使用的 URL：
- `false` → 供应商真实 URL（直连）
- `true` → 本地代理地址（代理模式，需要模型映射）

**修改文件**：`crates/claude-codex-pro-core/src/plugin_hub.rs`

## 测试前提条件

1. ✅ 代码已修改
2. 🔄 Release 版本重新构建（清理 core 包后）
3. ⏳ 等待构建完成

## 测试步骤

### 步骤 1：完全退出 Claude Desktop

```bash
taskkill /F /IM claude.exe
```

验证：`tasklist | grep claude.exe` 应该没有输出

### 步骤 2：启动新构建的 CCP

```bash
cd D:\Project\Claude-Codex-Pro-Tool\.claude\worktrees\project-file-size-29f861
.\target\release\claude-codex-pro.exe
```

### 步骤 3：配置供应商为直连模式

在 CCP 管理界面中：

1. 进入"供应商管理"页面
2. 选择一个供应商（推荐：Anthropic 官方或自定义供应商）
3. 点击"编辑"
4. **关键检查点**：确认"需要模型映射"开关为**关闭状态**
5. 如果是手动指定模型列表模式，确保模型列表中有至少一个安全的模型 ID（如 `claude-3-5-sonnet-20241022`）
6. 保存配置
7. 点击"应用到 Claude Desktop"

### 步骤 4：检查生成的配置文件

**配置文件路径**：
- Windows: `%APPDATA%\Claude\dev_overrides.json`
- 完整路径: `C:\Users\Damon\AppData\Roaming\Claude\dev_overrides.json`

```bash
cat "$APPDATA/Claude/dev_overrides.json" | grep -A 5 "inferenceGatewayBaseUrl"
```

**预期结果（直连模式）**：
```json
"inferenceGatewayBaseUrl": "https://api.anthropic.com"
```
或你配置的自定义供应商 URL，如：
```json
"inferenceGatewayBaseUrl": "https://api.toporeduce.cn"
```

**错误结果（修复前）**：
```json
"inferenceGatewayBaseUrl": "http://127.0.0.1:57331/claude-desktop"
```

### 步骤 5：重启 Claude Desktop

1. 完全退出 Claude Desktop（右键托盘图标 → 退出）
2. 重新启动 Claude Desktop
3. 等待启动完成（约 5-10 秒）

### 步骤 6：验证 Claude Desktop 中的配置

在 Claude Desktop 中：

1. 打开设置（右上角齿轮图标）
2. 进入"连接"或"开发者"设置
3. 查找"第三方 Gateway"或"Inference Gateway"配置区域

**预期显示**：
- Base URL: `https://api.anthropic.com`（或你的自定义 URL）
- Provider: Gateway
- Auth Scheme: Bearer

**不应显示**：
- Base URL: `http://127.0.0.1:57331/claude-desktop`

### 步骤 7：功能测试

1. 在 Claude Desktop 中新建对话
2. 发送测试消息："你好，请告诉我当前时间"
3. 验证能正常收到响应

**如果失败**：
- 检查 API Key 是否正确
- 检查供应商 URL 是否可访问
- 检查网络连接

## 对比测试：代理模式

为了确保代理模式没有回归，也需要测试：

### 代理模式配置

1. 在 CCP 中编辑同一个供应商
2. **开启"需要模型映射"**
3. 配置模型映射规则（如 `claude-3-5-sonnet-20241022` → `claude-sonnet-3.5`）
4. 保存并应用

### 代理模式验证

检查 `dev_overrides.json`：

```bash
cat "$APPDATA/Claude/dev_overrides.json" | grep "inferenceGatewayBaseUrl"
```

**预期结果（代理模式）**：
```json
"inferenceGatewayBaseUrl": "http://127.0.0.1:57331/claude-desktop"
```

这是**正确的**，因为代理模式需要通过本地代理转换模型 ID。

## 验收标准

### 必须通过

- [ ] 直连模式：`dev_overrides.json` 中 `inferenceGatewayBaseUrl` 为供应商真实 URL
- [ ] 直连模式：Claude Desktop 能正常对话
- [ ] 代理模式：`dev_overrides.json` 中 `inferenceGatewayBaseUrl` 为本地代理地址
- [ ] 代理模式：Claude Desktop 能正常对话
- [ ] 切换模式后重启 Claude Desktop，配置能正确生效

### 额外验证

- [ ] Anthropic 官方预设（已经是直连模式）没有回归
- [ ] 自定义供应商的直连模式正常工作
- [ ] 模型映射的转换逻辑没有受影响

## 回滚方案

如果测试失败，恢复到修复前的版本：

```bash
git checkout HEAD~1 crates/claude-codex-pro-core/src/plugin_hub.rs
cargo build --release
```

## 已知限制

1. 直连模式要求供应商 API 完全兼容 Anthropic API 格式
2. 直连模式下模型 ID 必须符合 Claude Desktop 的安全规则
3. 某些供应商可能需要代理模式来转换模型 ID 格式
