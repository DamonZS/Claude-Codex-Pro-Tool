# Claude Desktop 供应商直连能力对齐 - 验收标准

## 验证的规格文档

`spec/feature-claude-desktop-direct-connect-parity.md`

## 通过/失败标准

### 1. UI 显示正确（对齐 cc-switch）

**验证方式**：目视检查 + 截图

**通过标准**：

- [ ] 新建 Claude Desktop 供应商时，"高级选项"中显示"模型配置"标题
- [ ] 标题下方有说明文字，根据当前模式动态显示：
  - 直连模式：说明"仅当供应商直接接受 Claude Desktop 可识别的四档角色 ID..."
  - 模型映射模式：说明"Claude Desktop 只接受 claude-sonnet-* / claude-opus-*..."
- [ ] 右侧显示"接入方式" Select 下拉框，宽度约 156px
- [ ] Select 有两个选项：
  - 中文："直连"、"模型映射"
  - 英文："Direct"、"Model Mapping" 或 "Proxy"
- [ ] 默认选中"直连"
- [ ] 选中"直连"时，显示"模型列表"区域，提示"留空时 Claude Desktop 会自动读取 /v1/models；勾选 1M 会声明支持 1M 上下文"
- [ ] 选中"模型映射"时，显示"上游格式"选择 + 模型映射表（Sonnet/Opus/Fable/Haiku 四行）
- [ ] 编辑 Anthropic 官方预设时，"接入方式"应选中"直连"

### 2. 直连模式留空保存成功

**验证方式**：操作 + 日志检查

**通过标准**：

- [ ] 新建 Claude Desktop 供应商，填写 Base URL `https://api.anthropic.com`、API Key（任意测试值）
- [ ] 选择"直连"模式，**不添加任何模型行**（模型列表完全留空）
- [ ] 点击"保存"，成功保存（无报错弹窗）
- [ ] 刷新页面，重新编辑该供应商，"接入方式"仍显示为"直连"，模型列表为空
- [ ] 检查保存的配置数据：
  - `modelMappingEnabled` 为 `false`
  - `modelList` 为空字符串
  - `claudeDesktopMode` 为 `"direct"`

### 3. 手动填写模型时校验生效

**验证方式**：操作 + 错误提示检查

**通过标准**：

- [ ] 新建 Claude Desktop 供应商，选择"直连"模式
- [ ] 点击"添加模型"，添加一行，填写非法 ID `gpt-4`（不符合 Claude Desktop 规范）
- [ ] 点击"保存"，弹出错误提示："Claude Desktop 直连模型 ID 无效：gpt-4。请使用 claude-/anthropic/claude- 的 Sonnet、Opus、Haiku 或 Fable 模型。"
- [ ] 修改模型 ID 为 `claude-sonnet-4-6`，再次保存，成功

### 4. 模式切换不丢失数据

**验证方式**：操作 + 数据保留检查

**通过标准**：

- [ ] 新建 Claude Desktop 供应商，选择"模型映射"
- [ ] 填写模型映射表（例如 Sonnet 角色的"实际请求模型"填 `custom-sonnet-v1`）
- [ ] 切换到"直连"模式，再切换回"模型映射"
- [ ] 之前填写的 `custom-sonnet-v1` 仍在输入框中（数据未丢失）
- [ ] 同样验证"直连"模式的模型列表数据保留（添加一个模型，切换到"模型映射"，再切回"直连"，模型仍在）

### 5. 获取模型按钮在两种模式的行为

**验证方式**：操作 + UI 检查

**通过标准**：

- [ ] 直连模式和模型映射模式下都显示"获取模型"按钮
- [ ] 两种模式下点击"获取模型"都能成功获取（假设 API 有效）
- [ ] 获取后，模型可以通过下拉选择按钮填入对应字段

### 6. Anthropic 预设行为无回归

**验证方式**：操作 + 配置对比

**通过标准**：

- [ ] 在供应商列表中找到 Anthropic Official 供应商（或新建时选择该预设）
- [ ] 点击"应用"或"启用"，成功应用
- [ ] 启动 Claude Desktop（如环境允许），验证模型列表正常显示（claude-sonnet-4-6 等）
- [ ] 检查预设的配置（通过代码或日志）：
  - `claudeDesktopMode` 仍为 `"direct"`
  - `modelMappingEnabled` 仍为 `false`
  - `modelList` 为空或包含预设的模型列表

### 7. TypeScript 类型检查通过

**验证方式**：运行命令

**命令**：
```bash
npm --prefix apps/claude-codex-pro-manager run check
```

**通过标准**：
- [ ] 无 TypeScript 错误

### 8. 前端构建成功

**验证方式**：运行命令

**命令**：
```bash
npm --prefix apps/claude-codex-pro-manager run vite:build
```

**通过标准**：
- [ ] 构建成功，无错误

### 9. 开发服务器运行正常

**验证方式**：运行命令 + 浏览器访问

**命令**：
```bash
npm --prefix apps/claude-codex-pro-manager run vite:dev
```

**通过标准**：
- [ ] 服务启动成功（通常在 `http://localhost:5173`）
- [ ] 浏览器打开无控制台错误
- [ ] 能正常进入供应商页面，UI 交互流畅

## 证据要求

### 必需证据

1. **直连模式留空保存成功的截图**：
   - 显示"接入方式"选中"直连"
   - 显示模型列表区域为空（没有任何模型行）
   - 显示"保存成功"或类似提示
   
2. **手动填写模型校验失败的截图**：
   - 显示错误提示内容（包含非法模型 ID `gpt-4`）

3. **UI 布局截图**：
   - 显示"模型配置"区域的完整布局（左侧标题+说明，右侧 Select）
   - 直连模式下的完整界面
   - 模型映射模式下的完整界面

4. **Anthropic 预设编辑截图**：
   - 显示"接入方式"推断为"直连"

5. **TypeScript 检查输出**：
   - 命令输出文本或截图，显示无错误

6. **构建成功输出**：
   - 命令输出文本或截图，显示构建完成

### 可选证据

- 模式切换前后的界面对比（证明数据保留）
- 配置 JSON 对比（保存前后的 `modelList` / `modelMappingEnabled` 字段）

## 已知非目标

以下不在本次验收范围：

- **不验证**：直连模式下供应商 `/v1/models` 接口的实际响应内容（供应商侧行为）
- **不验证**：Claude Desktop 实际启动后的模型显示（需要完整的后端 + Claude Desktop 环境）
- **不验证**：Codex、聚合供应商、Claude（非 Desktop）的行为（本次改动未涉及）
- **不验证**：路由开关、API 格式选择等其他供应商配置项（无改动）
- **不验证**：OAuth 供应商（GitHub Copilot、Codex OAuth、xAI OAuth）的行为（它们强制 `proxy` 模式）

## 回归风险检查

### 必须确认无回归的场景

- [ ] 编辑现有的"模型映射已开启"的 Claude Desktop 供应商，保存后 `modelMappingEnabled` 仍为 `true`
- [ ] 编辑现有的"直连模式 + 手动指定模型列表"的 Claude Desktop 供应商，保存后 `modelList` 内容不变
- [ ] Codex 供应商的编辑流程无变化（不显示"接入方式"选项，显示原有的"上游格式"和"模型映射"）
- [ ] 聚合供应商的编辑流程无变化
- [ ] Anthropic 预设的 `claudeDesktopMode: "direct"` 和 `modelMappingEnabled: false` 配置保持不变

## 完成条件

**所有"通过标准"复选框均被勾选，且提供了必需证据。**

如果任一项失败：
1. 记录失败原因
2. 修复代码
3. 重新验证失败项及相关项
4. 更新证据

## 参考对比

本次实现应与 cc-switch 的行为一致：
- cc-switch 直连模式留空模型列表可以保存成功
- cc-switch 只有两个模式选项："直连" 和 "模型映射"
- cc-switch 的 UI 布局为：左侧标题+说明，右侧 Select
- cc-switch 的保存校验：只检查用户填写的模型行（`routeEntries.filter(...)`）
