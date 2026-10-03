# 规格文档：吸收 Oculos 电脑操作技术并应用到 CCP Computer Use

## 1. 背景

### 1.1 现状

**CCP Computer Use 当前实现**（位于 `crates/claude-codex-pro-core/src/claude_desktop_computer_use/`）：
- 基于 MCP 协议的单 exe Computer Use 服务器
- Windows 平台：使用 `SendInput` API 实现鼠标/键盘控制
- 截图：使用 GDI `BitBlt` + `GetDIBits` 捕获主屏幕
- 工具集：`screenshot`, `click`, `move_mouse`, `drag`, `scroll`, `type_text`, `press_keys`, `cursor_position`, `wait`
- 急停机制：检测鼠标左上角触发紧急停止
- 局限性：
  - 仅支持主屏幕截图，无多显示器支持
  - 无 UI 元素树访问能力（accessibility tree）
  - 无应用窗口/进程管理
  - 无元素查找、高亮、等待能力
  - 键盘输入仅支持 Unicode 文本输入和简单按键组合

**Oculos 实现**（位于 `H:\xunlei\oculos-main\oculos-main\src\`）：
- 基于 Windows UI Automation (UIA) 的完整 UI 自动化框架
- HTTP API + MCP 双协议支持
- 核心能力：
  - UI 元素树遍历（`get_ui_tree`, `find_elements`）
  - 元素精准操作（`click_element`, `set_text`, `send_keys`, `toggle_element`, `expand_element`, `select_element`, `set_range`, `scroll_element`）
  - 批量操作（`batch_actions` 支持原子化的多步骤操作）
  - 等待与轮询（`wait_for_element` 支持 `appears` / `disappears` 条件）
  - 窗口管理（`list_windows`, `focus_window`, `close_window`）
  - 元素高亮（`highlight_element` 临时绘制矩形）
  - 截图能力（`screenshot_window`, `screenshot_element`）
  - 复杂键盘输入（支持 `{CTRL+A}`, `{WIN+D}`, `{TAB 3}` 语法）

### 1.2 目标

将 Oculos 的先进技术吸收并应用到 CCP Computer Use 中，使其具备：
1. **UI 元素树访问**：让 AI 能看到应用的结构化 UI 元素，而非仅依赖截图
2. **精准元素操作**：基于元素 ID 的确定性操作，替代基于像素坐标的盲目点击
3. **窗口/进程管理**：列出、聚焦、关闭特定应用窗口
4. **智能等待与查找**：等待元素出现/消失，按名称/类型查找元素
5. **批量操作**：原子化执行多步骤 UI 流程
6. **更强的键盘控制**：支持 Oculos 的 `{KEY}` 语法和修饰键组合

## 2. 技术要点

### 2.1 Windows UI Automation (UIA) 集成

Oculos 的核心是 Windows UI Automation API（`windows::UI::UIAutomation`）：

```rust
// 初始化 UIA 客户端
CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)

// 获取根元素
automation.GetRootElement() -> IUIAutomationElement

// 从 HWND 获取元素
automation.ElementFromHandle(hwnd) -> IUIAutomationElement

// 遍历子元素
element.FindAll(scope, condition) -> IUIAutomationElementArray

// 元素操作模式（Patterns）
element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
element.GetCurrentPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId)
```

**关键依赖**：
- `windows = "0.58"` crate
- `windows::UI::UIAutomation::*` 模块
- COM 初始化（每线程 `CoInitializeEx(COINIT_MULTITHREADED)`）

### 2.2 UI 元素树表示

Oculos 的 `UiElement` 结构（`src/types.rs`）：

```rust
pub struct UiElement {
    pub oculos_id: String,          // 稳定 ID，格式 "pid.runtime_id"
    pub name: String,                // 元素名称（按钮文本、窗口标题等）
    pub element_type: ElementType,   // Button, Edit, ListItem, Window 等
    pub value: Option<String>,       // 输入框当前值
    pub bounding_rect: Rect,         // 屏幕坐标
    pub actions: Vec<String>,        // 可用操作列表
    pub children: Vec<UiElement>,    // 子元素（递归树）
    pub keyboard_focusable: bool,
    pub offscreen: bool,
    pub toggle_state: Option<String>,
    pub range_value: Option<RangeValue>,
    pub help_text: Option<String>,
}
```

**与 CCP 的集成策略**：
- CCP 新增 `list_windows`, `get_ui_tree`, `find_elements` 工具
- 保持现有的坐标操作工具作为后备方案
- AI 优先使用元素树，仅在必要时（Canvas、游戏界面）降级到截图+坐标

### 2.3 元素查找与等待

Oculos 的 `find_elements` 实现（`src/ops.rs:50-57`）：

```rust
pub fn find(
    backend: &dyn UiBackend,
    target: Target,  // Pid(u32) 或 Hwnd(usize)
    params: &FindParams
) -> Result<Vec<UiElement>>

pub struct FindParams {
    pub query: Option<String>,           // 名称模糊匹配
    pub element_type: Option<ElementType>, // 类型过滤
    pub interactive_only: bool,          // 仅可交互元素
}
```

**等待机制**（`src/ops.rs:95-134`）：

```rust
pub fn wait_for(
    backend: &dyn UiBackend,
    target: Target,
    params: &FindParams,
    until: WaitUntil,  // Appears | Disappears
    timeout_ms: u64
) -> Result<Vec<UiElement>>
```

- 默认超时 5 秒，最大 30 秒
- 轮询间隔 250ms
- `Appears`：至少 1 个元素匹配即返回
- `Disappears`：所有匹配元素消失即返回（窗口关闭也算成功）

### 2.4 批量操作与原子性

Oculos 的 `batch_actions` 工具（`src/mcp.rs:634-659`）：

```json
{
  "actions": [
    { "element_id": "123.456", "action": "click" },
    { "element_id": "123.789", "action": "set-text", "text": "hello" },
    { "element_id": "123.101", "action": "send-keys", "keys": "{CTRL+S}" }
  ],
  "stop_on_error": true,  // 默认 true，首次失败即中止
  "delay_ms": 100         // 步骤间延迟
}
```

**验证前置**：
- 所有步骤先验证参数合法性（`prepare_batch`）
- 任一步骤非法则全部不执行
- 执行时返回每步的成功/失败状态

### 2.5 键盘输入增强

Oculos 支持的 `{KEY}` 语法（`src/keys.rs`）：

```rust
pub enum KeyStep {
    Text(String),        // 纯文本
    Chord(Chord),        // 组合键
}

pub struct Chord {
    pub modifiers: Vec<Modifier>,  // Ctrl, Shift, Alt, Meta
    pub key: Option<Key>,          // 字符键或功能键
}
```

**语法示例**：
- `{CTRL+A}` → Ctrl 按下 + A 按下 + A 释放 + Ctrl 释放
- `{WIN+D}` → Meta(Windows) + D 组合
- `{TAB 3}` → Tab 键重复 3 次
- `{ENTER}`, `{ESC}`, `{BACKSPACE}`, `{DELETE}`, `{F1}` 等功能键
- `{{ 和 }}` → 字面量大括号
- 混合文本：`Hello{CTRL+A}{DELETE}World` → 输入 Hello，全选，删除，输入 World

**CCP 集成**：
- 新增 `send_keys_advanced(element_id, keys: String)` 工具
- 保持现有 `type_text` 用于简单文本输入
- 使用 Oculos 的键盘解析器（`keys::parse`）

### 2.6 窗口截图优化

Oculos 的多显示器截图支持（`src/platform/windows.rs:1203-1239`）：

```rust
fn screenshot_element(&self, oculos_id: &str) -> Result<Vec<u8>> {
    let element = self.lookup(oculos_id)?;
    let bounds = element.CurrentBoundingRectangle()?; // 元素屏幕坐标
    
    // 检查元素是否在屏幕外
    if element.CurrentIsOffscreen()? {
        return Err("元素不在屏幕上（使用 scroll-into-view）");
    }
    
    // 裁剪到可见屏幕区域
    let visible = intersect(bounds, virtual_screen_rect())?;
    capture_screen_rect(visible)
}
```

**关键改进**：
- 支持多显示器的虚拟屏幕坐标系（`virtual_screen_rect`）
- 元素级截图（只捕获特定 UI 元素区域）
- 窗口级截图（`screenshot_window(pid)` 捕获整个窗口）

## 3. 技术架构设计

### 3.1 模块分层

```
crates/claude-codex-pro-core/src/claude_desktop_computer_use/
├── mod.rs                    # 模块导出
├── server.rs                 # MCP 服务器（现有）
├── tools.rs                  # 工具定义（现有）
├── register.rs               # 注册到 Claude Desktop（现有）
├── tests.rs                  # 测试（现有）
├── platform/
│   ├── mod.rs
│   ├── windows.rs            # 现有 Backend（鼠标/键盘/截图）
│   ├── macos.rs              # macOS 存根
│   └── unsupported.rs        # 不支持平台存根
└── uia/                      # 新增：UI Automation 层
    ├── mod.rs
    ├── backend.rs            # UiaBackend trait 实现
    ├── element.rs            # UiElement 转换与序列化
    ├── find.rs               # 元素查找与等待
    ├── actions.rs            # 元素操作（click, set_text, toggle 等）
    ├── windows.rs            # 窗口管理（list, focus, close）
    ├── screenshot.rs         # 多显示器截图支持
    ├── keys.rs               # 键盘输入解析（复制 Oculos keys.rs）
    └── registry.rs           # UIA 元素注册表（Runtime ID → Element 映射）
```

### 3.2 Backend 分层设计

现有的 `Backend` trait（`tools.rs`）已实现基础鼠标/键盘/截图操作。新增一个更高级的 `UiaBackend` trait：

```rust
// tools.rs（现有）
pub trait Backend {
    fn screen_size(&self) -> Result<ScreenSize>;
    fn capture(&self) -> Result<Screen>;
    fn cursor_position(&self) -> Result<(i32, i32)>;
    fn move_mouse(&mut self, x: i32, y: i32) -> Result<()>;
    fn click(&mut self, x: i32, y: i32, button: MouseButton, count: u32) -> Result<()>;
    fn drag(&mut self, from: (i32, i32), to: (i32, i32)) -> Result<()>;
    fn scroll(&mut self, x: i32, y: i32, dx: i32, dy: i32) -> Result<()>;
    fn type_text(&mut self, text: &str) -> Result<()>;
    fn press_keys(&mut self, keys: &[String]) -> Result<()>;
}

// uia/backend.rs（新增）
pub trait UiaBackend: Backend {
    // 窗口管理
    fn list_windows(&self) -> Result<Vec<WindowInfo>>;
    fn focus_window(&self, pid: u32) -> Result<()>;
    fn close_window(&self, pid: u32) -> Result<()>;
    
    // UI 元素树
    fn get_ui_tree(&self, pid: u32) -> Result<UiElement>;
    fn get_ui_tree_hwnd(&self, hwnd: usize) -> Result<UiElement>;
    
    // 元素查找
    fn find_elements(
        &self,
        pid: u32,
        query: Option<&str>,
        element_type: Option<&ElementType>,
        interactive_only: bool
    ) -> Result<Vec<UiElement>>;
    
    // 元素操作
    fn click_element(&self, element_id: &str) -> Result<()>;
    fn set_text(&self, element_id: &str, text: &str) -> Result<()>;
    fn send_keys(&self, element_id: &str, keys: &str) -> Result<()>;
    fn focus_element(&self, element_id: &str) -> Result<()>;
    fn toggle_element(&self, element_id: &str) -> Result<()>;
    fn expand_element(&self, element_id: &str) -> Result<()>;
    fn collapse_element(&self, element_id: &str) -> Result<()>;
    fn select_element(&self, element_id: &str) -> Result<()>;
    fn set_range(&self, element_id: &str, value: f64) -> Result<()>;
    fn scroll_element(&self, element_id: &str, direction: &str) -> Result<()>;
    fn scroll_into_view(&self, element_id: &str) -> Result<()>;
    
    // 等待与高亮
    fn wait_for_element(
        &self,
        pid: u32,
        query: Option<&str>,
        element_type: Option<&ElementType>,
        until: WaitUntil,
        timeout_ms: u64
    ) -> Result<Vec<UiElement>>;
    fn highlight_element(&self, element_id: &str, duration_ms: u64) -> Result<Rect>;
    
    // 截图
    fn screenshot_window(&self, pid: u32) -> Result<Vec<u8>>;
    fn screenshot_element(&self, element_id: &str) -> Result<Vec<u8>>;
}
```

### 3.3 工具集扩展

在现有 8 个工具基础上新增 15 个工具：

**窗口管理**（3 个）：
1. `list_windows` - 列出所有可见窗口（PID、标题、程序名、主窗口 HWND）
2. `focus_window` - 聚焦指定窗口到前台
3. `close_window` - 关闭窗口（发送 WM_CLOSE）

**UI 元素访问**（2 个）：
4. `get_ui_tree` - 获取完整 UI 元素树（递归结构）
5. `find_elements` - 查找匹配条件的元素（名称、类型、仅可交互）

**元素操作**（7 个）：
6. `click_element` - 点击元素（基于 ID）
7. `set_text` - 设置输入框文本
8. `send_keys_advanced` - 发送复杂键盘输入（`{CTRL+A}` 语法）
9. `focus_element` - 聚焦元素
10. `toggle_element` - 切换复选框/开关
11. `expand_element` - 展开下拉框/树节点
12. `select_element` - 选择列表项/单选按钮

**辅助工具**（3 个）：
13. `wait_for_element` - 等待元素出现/消失
14. `highlight_element` - 高亮元素（临时红框）
15. `batch_actions` - 批量执行元素操作

**截图增强**（2 个）：
16. `screenshot_window` - 窗口截图
17. `screenshot_element` - 元素截图

**总计**：23 个工具（8 现有 + 15 新增）

### 3.4 数据结构定义

```rust
// uia/element.rs
pub struct UiElement {
    pub id: String,                  // "pid.runtime_id" 格式
    pub name: String,
    pub element_type: ElementType,
    pub value: Option<String>,
    pub bounding_rect: Rect,
    pub actions: Vec<String>,
    pub children: Vec<UiElement>,
    pub keyboard_focusable: bool,
    pub offscreen: bool,
    pub toggle_state: Option<ToggleState>,
    pub range_value: Option<RangeValue>,
    pub help_text: Option<String>,
}

pub enum ElementType {
    Button, CheckBox, ComboBox, Document, Edit,
    Group, Hyperlink, Image, List, ListItem,
    Menu, MenuBar, MenuItem, Pane, RadioButton,
    ScrollBar, Slider, Spinner, StatusBar, Tab,
    TabItem, Table, Text, TitleBar, ToolBar,
    ToolTip, Tree, TreeItem, Window, Custom,
}

pub enum ToggleState {
    Off, On, Indeterminate,
}

pub struct RangeValue {
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
}

pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

// uia/windows.rs
pub struct WindowInfo {
    pub pid: u32,
    pub hwnd: usize,
    pub title: String,
    pub exe_name: String,
    pub bounds: Rect,
    pub is_foreground: bool,
}

// uia/find.rs
pub struct FindParams {
    pub query: Option<String>,
    pub element_type: Option<ElementType>,
    pub interactive_only: bool,
}

pub enum WaitUntil {
    Appears,
    Disappears,
}
```

## 4. 实现范围

### 4.1 阶段 1：核心基础设施（必需）

**目标**：建立 UIA 集成基础，验证技术可行性

**交付内容**：
1. ✅ 创建 `uia/` 模块结构
2. ✅ 实现 `UiaBackend` trait
3. ✅ Windows UIA 初始化与 COM 管理
4. ✅ 元素树遍历与序列化（`get_ui_tree`）
5. ✅ 元素查找（`find_elements`）
6. ✅ 基础元素操作（`click_element`, `set_text`, `focus_element`）
7. ✅ 窗口管理（`list_windows`, `focus_window`）
8. ✅ 单元测试：在真实 Windows 应用上验证

**验收标准**：
- 能列出当前打开的窗口
- 能获取记事本的 UI 元素树
- 能查找记事本中的"保存"按钮
- 能点击按钮并设置文本框内容

### 4.2 阶段 2：高级操作与等待（重要）

**目标**：提供 AI 所需的完整 UI 自动化能力

**交付内容**：
1. ✅ 实现 `send_keys_advanced`（`{KEY}` 语法解析）
2. ✅ 实现 `toggle_element`, `expand_element`, `select_element`
3. ✅ 实现 `wait_for_element`（轮询与超时）
4. ✅ 实现 `batch_actions`（原子化批量操作）
5. ✅ 实现 `highlight_element`（临时绘制矩形）
6. ✅ 集成测试：完整 UI 流程自动化（打开程序 → 查找元素 → 批量操作 → 等待结果）

**验收标准**：
- 能使用 `{CTRL+A}{DELETE}` 清空文本框
- 能等待"另存为"对话框出现
- 能批量执行：点击文件菜单 → 点击另存为 → 输入文件名 → 点击保存
- 能高亮显示即将操作的按钮

### 4.3 阶段 3：截图优化与集成（可选）

**目标**：增强截图能力，替代现有简单实现

**交付内容**：
1. ⚠️ 多显示器支持（虚拟屏幕坐标系）
2. ⚠️ 窗口截图（`screenshot_window`）
3. ⚠️ 元素截图（`screenshot_element`）
4. ⚠️ 更新 MCP 服务器以暴露新工具

**验收标准**：
- 能在双显示器环境下截取副屏窗口
- 能截取特定按钮的区域
- Claude Desktop 能调用所有新增工具

## 5. 技术约束

### 5.1 平台支持

- **Windows**：完整实现（优先级 1）
- **macOS**：暂不支持（Oculos 有 macOS 实现，但需 `accessibility` 权限和 CGEvent API，复杂度高）
- **Linux**：暂不支持（Oculos 有 AT-SPI 实现，但需 `atspi` crate，依赖 D-Bus）

### 5.2 依赖约束

**新增依赖**：
```toml
[dependencies]
windows = { version = "0.58", features = [
    "Win32_UI_Accessibility",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Graphics_Gdi",
    "Win32_System_Com",
    "Win32_Foundation"
] }
image = "0.25"  # 已有
base64 = "0.22"  # 已有
```

**最小化原则**：
- 不引入 Oculos 的 HTTP 服务器（`axum`, `tower-http`）
- 不引入 Oculos 的日志系统（`tracing`, `tracing-subscriber`，CCP 有自己的）
- 仅复用核心逻辑：`types.rs`, `ops.rs`, `keys.rs`, `platform/windows.rs`

### 5.3 性能约束

- 元素树遍历深度：默认最大 10 层（防止卡死）
- 查找结果限制：默认最多 100 个元素，最大 500 个
- 等待超时：默认 5 秒，最大 30 秒
- 批量操作：最多 100 个步骤
- 截图压缩：JPEG 质量 80，与现有一致

### 5.4 安全约束

- UIA 操作仅限当前用户会话
- 不能操作以管理员权限运行的窗口（`SendInput` 限制）
- 不能操作 UAC 提示窗口
- 急停机制保持不变（鼠标左上角触发）

## 6. 风险与后续

### 6.1 已知风险

1. **COM 初始化复杂性**：
   - 每个线程需独立初始化 COM（`CoInitializeEx`）
   - MCP 服务器的 stdin/stdout 循环运行在同步线程（`tokio::task::spawn_blocking`）
   - 需确保 UIA 调用在同一线程或使用 MTA（Multi-Threaded Apartment）

2. **元素 ID 稳定性**：
   - Runtime ID 在元素重建后可能改变（例如动态列表项）
   - 需告知 AI：元素 ID 在操作失败时需重新查找

3. **性能开销**：
   - 完整 UI 树遍历可能较慢（大型应用如 VS Code、Chrome）
   - 建议 AI 优先使用 `find_elements` 而非 `get_ui_tree`

4. **macOS 未实现**：
   - Oculos 的 macOS 实现依赖 `accessibility` 框架和 CGEvent API
   - 需要用户授权辅助功能权限
   - 如需支持，需单独评估工作量（预计 2-3 周）

### 6.2 后续优化

1. **缓存机制**：
   - 缓存最近查找的元素（避免重复遍历）
   - Runtime ID 映射缓存（加速后续操作）

2. **增量更新**：
   - 监听 UIA 事件（`IUIAutomationElementChangedEventHandler`）
   - 仅更新变化的子树

3. **跨进程支持**：
   - 支持远程桌面场景（需 UIA RemoteOps）

4. **AI 指令优化**：
   - 提供结构化的 UI 元素树摘要（而非完整 JSON）
   - 根据任务类型动态调整查找策略

## 7. 验收标准

### 7.1 功能验收

| 工具 | 验收标准 |
|------|---------|
| `list_windows` | 列出至少 3 个可见窗口（包括记事本、资源管理器） |
| `get_ui_tree` | 返回记事本的完整元素树（至少包含菜单栏、文本框、状态栏） |
| `find_elements` | 能查找到记事本的"文件"菜单项 |
| `click_element` | 点击"文件"菜单项后菜单展开 |
| `set_text` | 在记事本文本框中输入"Hello World" |
| `send_keys_advanced` | 输入"Test{CTRL+A}{DELETE}"后文本框清空 |
| `focus_window` | 将后台记事本窗口带到前台 |
| `wait_for_element` | 点击"文件"后等待"另存为"菜单项出现（5 秒内） |
| `batch_actions` | 执行"点击文件 → 点击退出"组合操作 |
| `highlight_element` | 高亮记事本的"保存"按钮 2 秒 |

### 7.2 性能验收

- 元素树遍历：记事本（<50 个元素）< 500ms
- 查找元素：按名称查找（100 个元素内）< 200ms
- 点击元素：延迟 < 100ms
- 等待元素：250ms 轮询间隔，5 秒超时

### 7.3 稳定性验收

- 在 10 次连续操作中无崩溃
- 在操作不存在的元素时返回明确错误（而非崩溃）
- 在窗口关闭后操作元素时返回 `NotFound` 错误
- COM 初始化失败时降级到现有坐标操作

## 8. 不包含内容

以下内容**不在**本次技术吸收范围内：

1. ❌ Oculos 的 HTTP API 服务器（CCP 仅需 MCP 协议）
2. ❌ Oculos 的 Python/TypeScript SDK（CCP 仅暴露工具给 Claude）
3. ❌ Oculos 的 WebSocket 支持（CCP 使用 stdin/stdout MCP）
4. ❌ Oculos 的仪表板（Dashboard）UI（CCP 有自己的管理界面）
5. ❌ Oculos 的令牌认证机制（CCP 通过设置开关控制）
6. ❌ Oculos 的日志与 tracing（CCP 使用 `diagnostic_log`）
7. ❌ Linux AT-SPI 支持（优先级低）
8. ❌ macOS Accessibility 支持（评估后决定）

## 9. 交付检查清单

### 9.1 代码交付

- [ ] `crates/claude-codex-pro-core/src/claude_desktop_computer_use/uia/` 模块完整
- [ ] 所有新增工具有对应的 Rust 函数实现
- [ ] 单元测试覆盖核心逻辑（元素查找、操作、等待）
- [ ] 集成测试验证真实应用场景（记事本、计算器）
- [ ] 更新 `tools.rs` 的工具定义列表

### 9.2 文档交付

- [ ] 更新 `README.md`：说明新增的 UI Automation 能力
- [ ] 创建 `docs/computer-use-uia-guide.md`：AI 使用指南
- [ ] 更新 `docs/architecture.md`：UIA 模块架构说明
- [ ] 在设置页添加"UI Automation"功能开关说明

### 9.3 测试交付

- [ ] `cargo test -p claude-codex-pro-core uia` 全部通过
- [ ] 在真实 Windows 环境运行端到端测试
- [ ] 验证多显示器场景（如有条件）
- [ ] 验证急停机制不受影响

### 9.4 用户交付

- [ ] 在 CCP 管理界面显示"UI Automation 已启用"状态
- [ ] 提供测试用 prompt（例如"打开记事本并输入测试文本"）
- [ ] 记录已知限制（不能操作管理员窗口、UAC 提示）
