# Claude Desktop 供应商直连能力对齐

## 背景

当前 CCP 供应商系统对 Claude Desktop 目标存在功能不一致：

- **Anthropic 官方预设**：配置为 `claudeDesktopMode: "direct"` 和 `modelMappingEnabled: false`，可以直接使用供应商返回的模型 ID（如 `claude-sonnet-4-6`），无需模型映射或手动指定模型列表。
- **用户手动创建的 Claude Desktop 供应商**：UI 层面强制要求"开启模型映射"或"手动指定模型列表"，否则保存时会因 `supplierDirectModelIsClaudeDesktopSafe` 校验失败而拒绝保存。

这导致用户无法像使用 Anthropic 预设那样，为其他兼容 Anthropic API 格式的供应商（如自建代理、第三方中转）配置"直连模式"。

## 目标

### 包含

1. 为 Claude Desktop 供应商编辑器添加"直连模式"选项，允许用户选择：
   - **直连模式**（对应 `claudeDesktopMode: "direct"`）：使用供应商 `/v1/models` 返回的模型 ID，要求模型 ID 符合 Claude Desktop 安全规则（`claude-` / `anthropic/claude-` 前缀 + `sonnet-` / `opus-` / `haiku-` / `fable-` 角色）。
   - **模型映射模式**（对应 `claudeDesktopMode: "proxy"` 且 `modelMappingEnabled: true`）：当前已有的模型映射表功能。
   - **手动指定模型列表**（对应 `claudeDesktopMode: "direct"` 且有手动 `modelList`）：当前已有的手动模型列表功能。

2. 修改保存时的校验逻辑：
   - 直连模式：只在"明确指定的手动模型列表"中校验 `supplierDirectModelIsClaudeDesktopSafe`。
   - 如果用户选择"直连模式"且未手动指定模型列表，则跳过该校验（信任供应商的 `/v1/models` 会返回合规模型 ID）。

3. 保持 Anthropic 预设的配置不变，确保现有行为无回归。

### 不包含

- 不修改 `supplierDirectModelIsClaudeDesktopSafe` 的校验规则本身（保持对模型 ID 格式的安全要求不变）。
- 不改变 Codex 供应商、聚合供应商、Claude（非 Desktop）供应商的任何行为。
- 不添加"自动探测供应商是否支持直连"的逻辑。

## 用户视角

### 场景：为自建 Claude 代理配置直连

用户有一个符合 Anthropic Messages API 格式的自建代理 `https://my-proxy.example.com`，其 `/v1/models` 返回标准 Claude 模型 ID（如 `claude-sonnet-4-6`、`claude-opus-4-8`）。

**当前体验**：
1. 用户添加 Claude Desktop 供应商，填写 Base URL 和 API Key。
2. UI 显示"需要模型映射"开关，默认关闭。
3. 用户尝试保存，弹出错误："Claude Desktop 直连模型 ID 无效：（无模型）。请使用 claude-/anthropic/claude- 的 Sonnet、Opus、Haiku 或 Fable 模型，或开启模型映射。"
4. 用户被迫开启模型映射或手动填写模型列表。

**期望体验**：
1. 用户添加 Claude Desktop 供应商，填写 Base URL 和 API Key。
2. UI 显示"模型来源"选项：
   - **直连（使用供应商模型列表）**：信任供应商 `/v1/models` 返回的模型 ID，要求模型 ID 符合 Claude Desktop 规范。
   - **模型映射**：手动配置 Claude Desktop 模型角色到上游模型的映射关系。
   - **手动指定模型列表**：手动填写模型 ID 列表（高级选项）。
3. 用户选择"直连"，点击保存，成功。
4. Claude Desktop 启动后，从供应商读取模型列表并正常使用。

## 功能要求

### 1. UI 结构调整（对齐 cc-switch）

在 `SupplierScreen.tsx` 的 Claude Desktop 供应商编辑器"高级选项"中：

**当前结构**：
```
- 需要模型映射（ToggleSwitch）
  - 关闭时：显示"手动指定 Claude Desktop 模型列表"折叠面板
  - 开启时：显示模型映射表（Sonnet/Opus/Haiku/Fable 角色映射）
```

**新结构（复制 cc-switch 做法）**：
```
- 模型配置（标题）
  - 说明文字：根据当前模式显示不同提示
  - 接入方式（Select 下拉框）：
    - 直连 [默认]
    - 模型映射
  
- 根据选择显示对应面板：
  - 直连模式：
    - 显示"模型列表"区域（可留空）
    - 提示："配置 Claude Desktop 可用的 Sonnet、Opus、Haiku 模型。留空时 Claude Desktop 会自动读取 /v1/models；勾选 1M 会声明支持 1M 上下文。"
    - 可选的手动模型行（添加模型按钮 + 模型输入框 + 1M 复选框 + 删除按钮）
  - 模型映射模式：
    - 显示"上游格式"下拉框（Anthropic Messages / OpenAI Chat / OpenAI Responses / Gemini）
    - 显示固定四档模型映射表（Sonnet/Opus/Fable/Haiku）
```

**关键差异**：
- **只有两个选项**："直连" 和 "模型映射"（不是三个）
- "手动指定模型列表" 不是独立模式，而是**直连模式下的可选功能**
- 用户可以在直连模式下**留空模型列表**（信任供应商 `/v1/models`）或**手动填写**（明确指定）
- 使用 Select 下拉框，而非 RadioGroup

### 2. 数据模型字段（对齐 cc-switch）

`RelayProfile` 中相关字段的语义：

- `claudeDesktopMode`: `"direct"` | `"proxy"` | `""`
  - `"direct"`: 直连模式（可留空或手动填写模型列表）
  - `"proxy"`: 代理模式（模型映射）
  
- `modelMappingEnabled`: `boolean`
  - `true`: 开启模型映射表
  - `false`: 未开启模型映射
  
- `modelList`: `string`
  - **空**：直连模式下留空，信任供应商的 `/v1/models`
  - **非空**：直连模式下手动指定的模型列表（换行或逗号分隔）

**两种模式的字段组合**：

| 模式 | claudeDesktopMode | modelMappingEnabled | modelList | 说明 |
|------|-------------------|---------------------|-----------|------|
| 直连（自动） | `"direct"` | `false` | `""` | 信任供应商 `/v1/models`，不校验 |
| 直连（手动） | `"direct"` | `false` | 非空 | 手动指定模型，校验格式 |
| 模型映射 | `"proxy"` | `true` | `""` | 使用 modelMappingJson，不使用 modelList |

### 3. 保存时校验逻辑（对齐 cc-switch）

修改 `saveDraft` 函数中的校验（约 643 行）：

**当前逻辑**：
```typescript
if (targetApp === "claude-desktop" && !normalized.modelMappingEnabled) {
  const invalidModel = supplierDirectModelRows(normalized.modelList)
    .find((row) => !supplierDirectModelIsClaudeDesktopSafe(row.model));
  if (invalidModel) {
    // 报错...
  }
}
```

**新逻辑（对齐 cc-switch）**：
```typescript
if (targetApp === "claude-desktop" && !normalized.modelMappingEnabled) {
  // 直连模式：只校验用户主动填写的模型 ID
  const routeEntries = supplierDirectModelRows(normalized.modelList)
    .filter((row) => row.model.trim()); // 只处理非空行
  
  if (routeEntries.length > 0) {
    // 有手动填写的模型，校验格式
    const invalidModel = routeEntries
      .find((row) => !supplierDirectModelIsClaudeDesktopSafe(row.model));
    if (invalidModel) {
      actions.showNotice({
        title: "供应商保存",
        message: `Claude Desktop 直连模型 ID 无效：${invalidModel.model}。请使用 claude-/anthropic/claude- 的 Sonnet、Opus、Haiku 或 Fable 模型。`,
        status: "failed",
      });
      return null;
    }
  }
  // routeEntries.length === 0 时（用户留空），跳过校验，信任供应商
}
```

**关键变化**：
- 增加 `filter((row) => row.model.trim())` 过滤，只校验非空行
- 当用户完全留空模型列表时（`routeEntries.length === 0`），不报错，允许保存
- 报错提示简化，不再建议"开启模型映射"（因为这已经是模式选择的结果）

### 4. UI 文案（对齐 cc-switch）

**模式选项标签**：
- 中文：
  - "直连"
  - "模型映射"
- 英文：
  - "Direct"
  - "Model Mapping" 或 "Proxy"

**说明文字（显示在模式选择上方）**：
- **模型映射模式开启时**（中文）："Claude Desktop 只接受 claude-sonnet-* / claude-opus-* / claude-haiku-* / claude-fable-* 四档角色 ID。选择模型映射后，CCP 会把这四档映射到供应商的实际模型，并在使用期间保持本地路由开启。"
- **模型映射模式开启时**（英文）："Claude Desktop only accepts claude-sonnet-* / claude-opus-* / claude-haiku-* / claude-fable-* role IDs. With model mapping enabled, CCP will map these four roles to the supplier's actual models and keep local routing active."
- **直连模式开启时**（中文）："仅当供应商直接接受 Claude Desktop 可识别的四档角色 ID（claude-sonnet-* / claude-opus-* / claude-haiku-* / claude-fable-*）时才适用直连；其他模型名（含 claude-3-5-sonnet-… 等旧式 ID）请选择模型映射。"
- **直连模式开启时**（英文）："Only use direct mode when the supplier directly accepts Claude Desktop's four role IDs (claude-sonnet-* / claude-opus-* / claude-haiku-* / claude-fable-*); for other model names (including legacy IDs like claude-3-5-sonnet-…), use model mapping."

**直连模式下的模型列表提示**（中文）：
"配置 Claude Desktop 可用的 Sonnet、Opus、Haiku、Fable 模型。留空时 Claude Desktop 会自动读取 /v1/models；勾选 1M 会声明支持 1M 上下文。"

**直连模式下的模型列表提示**（英文）：
"Configure available Sonnet, Opus, Haiku, Fable models for Claude Desktop. When empty, Claude Desktop will automatically read from /v1/models; checking 1M declares 1M context support."

## UI / 交互要求

### 1. 布局（对齐 cc-switch）

参考 cc-switch 的 `ClaudeDesktopProviderForm.tsx` 第 952-1011 行的布局：

```tsx
<div className="flex items-stretch justify-between gap-4">
  <div className="min-w-0 flex-1 space-y-1 pr-3">
    <Label>模型配置</Label>
    <p className="text-xs leading-relaxed text-muted-foreground">
      {needsModelMapping ? "模型映射模式说明..." : "直连模式说明..."}
    </p>
  </div>
  <div className="flex shrink-0 items-center gap-2 border-l border-border-default pl-4">
    <Label htmlFor="claude-desktop-model-mode" className="text-sm font-normal text-muted-foreground">
      接入方式
    </Label>
    <Select value={mode} onValueChange={handleModeChange}>
      <SelectTrigger id="claude-desktop-model-mode" className="w-[156px]">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="direct">直连</SelectItem>
        <SelectItem value="proxy">模型映射</SelectItem>
      </SelectContent>
    </Select>
  </div>
</div>
```

**关键点**：
- 使用 `<Select>` 组件（下拉框），而非 RadioGroup
- 左侧是标题 + 动态说明文字（根据当前模式显示）
- 右侧是"接入方式"标签 + Select 下拉框
- Select 宽度固定 `w-[156px]`

### 2. 默认值

- 新建 Claude Desktop 供应商：默认选中"直连"模式（`mode: "direct"`）
- 编辑现有供应商：
  - 如果 `modelMappingEnabled === true`，则 `mode = "proxy"`
  - 如果 `modelMappingEnabled === false`，则 `mode = "direct"`
- Anthropic 预设：`modelMappingEnabled: false`，应显示为"直连"模式

### 3. 状态联动

- 切换模式时，保留两个模式各自的数据（参考 cc-switch 第 294-306、501-523 行）：
  - `directRoutes` state：存储直连模式的模型列表
  - `proxyRoutes` state：存储模型映射模式的映射表
  - 从"模型映射"切换到"直连"：保留 `proxyRoutes`，恢复 `directRoutes`
  - 从"直连"切换到"模型映射"：保留 `directRoutes`，恢复 `proxyRoutes`
  
- "获取模型"按钮：
  - 两种模式都显示（cc-switch 在两种模式下都有"获取模型"按钮）
  - 直连模式：获取后可填充到下拉选项，但不强制用户必须选择
  - 模型映射模式：获取后填充到"实际请求模型"的下拉选项

### 4. 模型列表行为（直连模式）

- **可以完全留空**（用户不添加任何模型行），此时保存成功，信任供应商的 `/v1/models`
- **可以手动添加模型**（点击"添加模型"按钮），每行包含：
  - 模型 ID 输入框（必填，如 `claude-sonnet-4-6`）
  - 1M 复选框
  - 删除按钮
  - 可选的模型下拉选择按钮（当已获取模型时显示）

## 数据与接口要求

### 输入

- 用户在 UI 选择模型来源模式
- 用户填写的 Base URL、API Key、模型列表或映射表数据

### 输出

- `RelayProfile` 对象，包含正确的 `claudeDesktopMode`、`modelMappingEnabled`、`modelList`、`modelMappingJson` 字段组合
- 保存到 `relayProfiles` 数组

### 错误处理

- 手动模型列表模式：保存时校验模型 ID 格式，不符合规范则拒绝保存并提示
- 直连模式：保存时不校验（运行时由 Claude Desktop 决定是否接受模型 ID）
- 模型映射模式：保存时校验映射表完整性（所有角色都有 requestModel）

## 技术约束

- 使用 React hooks 管理模式状态
- 复用现有的 `supplierDirectModelIsClaudeDesktopSafe` 校验函数（仅在手动模型列表模式调用）
- 复用现有的 `supplierModelMappingRows`、`supplierDirectModelRows` 等辅助函数
- 不修改后端 Rust 代码（纯前端 UI 层面改动）
- 不修改 `SUPPLIER_PRESETS` 中 Anthropic 预设的配置

## 交付范围

### 代码文件

1. `apps/claude-codex-pro-manager/src/components/supplier/SupplierScreen.tsx`
   - 将"需要模型映射"开关替换为"接入方式" Select 下拉框
   - 调整布局：左侧标题+说明，右侧 Select（参考 cc-switch 布局）
   - 修改 `saveDraft` 中的校验逻辑：只校验用户填写的模型行
   - 更新说明文字：根据当前模式动态显示
   - 直连模式下的模型列表提示改为"留空时 Claude Desktop 会自动读取 /v1/models"

2. `apps/claude-codex-pro-manager/src/lib/locales/zh.ts` 和 `en.ts`
   - 更新模式选项文案："直连" / "模型映射"
   - 更新说明文字的翻译 key
   - 更新直连模式下的模型列表提示

3. CSS 调整（如需要）
   - 确保 Select 宽度为 `156px`
   - 左右布局的间距和边框样式对齐 cc-switch

### 测试

- 手动验证：创建直连模式 Claude Desktop 供应商，**完全不填模型列表**，保存成功
- 手动验证：创建直连模式 Claude Desktop 供应商，**手动填写一个合法模型 ID**（如 `claude-sonnet-4-6`），保存成功
- 手动验证：创建直连模式 Claude Desktop 供应商，**手动填写一个非法模型 ID**（如 `gpt-4`），保存失败并提示
- 手动验证：编辑 Anthropic 预设，确认显示为"直连"模式
- 手动验证：两种模式互相切换，各自的数据不丢失
- TypeScript 类型检查通过
- 前端构建成功

### 文档

- 无额外文档要求（功能自解释）

## 验收标准

详见 `acceptance/feature-claude-desktop-direct-connect-parity.md`。

## 参考实现

本功能完全对齐 cc-switch 的实现：
- 参考文件：`H:/xunlei/cc-switch-main(1)/cc-switch-main/src/components/providers/forms/ClaudeDesktopProviderForm.tsx`
- 关键代码行：
  - 第 255 行：`mode` state 定义（`"direct"` | `"proxy"`）
  - 第 395-396 行：`effectiveMode` 和 `needsModelMapping` 计算
  - 第 501-523 行：`handleModelMappingChange` 切换逻辑
  - 第 730-743 行：保存时校验逻辑（直连模式只校验非空行）
  - 第 952-1011 行：UI 布局（左右分栏 + Select）
  - 第 1204-1292 行：两种模式下的面板内容
