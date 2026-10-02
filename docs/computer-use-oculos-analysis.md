# Oculos Computer Use 技术分析与 CCP 借鉴方案

## 项目概况

**Oculos** 是一个跨平台桌面自动化服务器，通过 MCP (Model Context Protocol) 暴露操作系统的无障碍树（Accessibility Tree），允许 AI 代理控制桌面应用。

- GitHub: https://github.com/grp06/oculos
- 协议: MCP over stdin/stdout (JSON-RPC 2.0)
- 支持平台: Windows (UIA), macOS (Accessibility API), Linux (AT-SPI)
- 使用场景: Claude Code、Claude Desktop、Cursor、Windsurf 等 AI 工具

## 核心架构

### 三层结构

```
┌─────────────────────────────────────┐
│  MCP Server (mcp.rs)                │  ← JSON-RPC over stdio
│  - 工具定义与 schema                │
│  - 参数验证与序列化                 │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  Operations Layer (ops.rs)          │  ← 业务逻辑
│  - find / tree / wait_for           │
│  - Action 抽象与批量执行            │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│  Platform Backend (platform/*.rs)   │  ← 平台特定实现
│  - Windows: UI Automation           │
│  - macOS: Accessibility             │
│  - Linux: AT-SPI                    │
└─────────────────────────────────────┘
```

### Windows 实现核心（platform/windows.rs）

#### 1. COM 公寓模型
```rust
thread_local! {
    static COM_APARTMENT: ComApartment = ComApartment::enter();
}

fn ensure_com() {
    COM_APARTMENT.with(|_| {});
}
```
- 每个 tokio blocking 线程进入 MTA (Multithreaded Apartment)
- `CoIncrementMTAUsage` 保持 MTA 在进程生命周期内存活

#### 2. 元素注册表与稳定 ID
```rust
unsafe fn element_id(element: &IUIAutomationElement) -> String {
    match runtime_id(element) {
        Some(ints) => registry::stable_id(ints),
        None => registry::stable_id((
            "uia-no-runtime-id",
            FALLBACK_IDS.fetch_add(1, Ordering::Relaxed),
        )),
    }
}
```
- UIA RuntimeId → 稳定的 `oculos_id`
- 元素复用时 ID 保持不变
- 无 RuntimeId 时用全局计数器兜底

#### 3. 缓存请求优化
```rust
unsafe fn cache_request(&self, scope: TreeScope) -> Result<IUIAutomationCacheRequest> {
    let request = self.automation.CreateCacheRequest()?;
    for &property in CACHED_PROPERTIES { request.AddProperty(property)?; }
    for &pattern in CACHED_PATTERNS { request.AddPattern(pattern)?; }
    // ...
    root.BuildUpdatedCache(&request) // ← 一次跨进程调用
}
```
- **树遍历**: `TreeScope_Subtree` 批量获取整个子树
- **查找**: `TreeScope_Element` 只缓存匹配元素本身
- **避免 N+1 问题**: 减少跨进程 COM 调用

#### 4. 操作降级策略（以 click 为例）
```rust
fn click_element(&self, oculos_id: &str) -> Result<()> {
    // 1. 尝试 InvokePattern
    if let Some(p) = optional_pattern::<IUIAutomationInvokePattern>(...) {
        return p.Invoke();
    }
    // 2. 尝试 TogglePattern
    if let Some(p) = optional_pattern::<IUIAutomationTogglePattern>(...) {
        return p.Toggle();
    }
    // 3. 尝试 SelectionItemPattern
    // 4. 尝试 LegacyIAccessiblePattern
    // 5. 最后手段：合成鼠标点击
    let point = mouse_target(&element, id, "click")?;
    mouse_click(point)?;
}
```
- 优先使用高层 Pattern（语义清晰、兼容性好）
- 降级到低层输入（坐标依赖、窗口覆盖检测）

#### 5. 键盘输入：Unicode vs Virtual Key
```rust
fn chord_batch(chord: &Chord) -> Result<KeyBatch> {
    // 单字符无修饰符 → Unicode 输入（布局无关）
    if chord.modifiers.is_empty() {
        if let Some(Key::Char(c)) = chord.key {
            return Ok(KeyBatch {
                inputs: unicode_inputs(&c.to_string()),
                modifiers: Vec::new(),
            });
        }
    }
    // 修饰符 + 功能键 → Virtual Key（依赖布局）
    let modifiers: Vec<(u16, bool)> = chord.modifiers.iter().map(|&m| modifier_vk(m)).collect();
    // ...
}
```
- **KEYEVENTF_UNICODE**: 直接发送 UTF-16 码点，不受键盘布局影响
- **Virtual Key**: `{CTRL+A}` 等快捷键必须用 VK，需 `VkKeyScanW` 查询

#### 6. 焦点劫持应对
```rust
unsafe fn bring_to_foreground(hwnd: HWND, pid: u32) -> bool {
    // 1. 直接尝试
    SetForegroundWindow(hwnd);
    if wait_foreground(pid) { return true; }
    
    // 2. 发送空输入事件解除锁定
    let dummy = mouse_input(0, 0, 0, MOUSE_EVENT_FLAGS(0));
    SendInput(&[dummy], ...);
    SetForegroundWindow(hwnd);
    if wait_foreground(pid) { return true; }
    
    // 3. 附加到前台线程输入状态
    AttachThreadInput(current_thread, foreground_thread, true);
    BringWindowToTop(hwnd);
    SetForegroundWindow(hwnd);
    AttachThreadInput(current_thread, foreground_thread, false);
    wait_foreground(pid)
}
```
- Windows 的 **foreground lock** 防止应用偷焦点
- 三级降级策略提高成功率

### MCP 协议实现（mcp.rs）

#### 1. 工具定义与提示
```rust
const INSTRUCTIONS: &str = "OculOS exposes the desktop's accessibility tree. \
    Workflow: list_windows → find_elements (preferred) or get_ui_tree → act. \
    Only call actions listed in an element's `actions`. ...";

struct ToolDef {
    name: &'static str,
    title: &'static str,
    description: &'static str,
    hints: (bool, bool, bool), // (readOnly, destructive, idempotent)
    schema: fn() -> Value,
}
```
- **instructions** 字段告诉 AI 工作流程
- **hints** 标注操作特性，帮助 AI 决策

#### 2. 输出压缩
```rust
fn compact(v: Value) -> Value {
    match v {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter_map(|(k, v)| {
                    let drop = match (&v, k.as_str()) {
                        (Value::Null, _) => true,
                        (Value::String(s), _) => s.is_empty(),
                        (Value::Bool(true), "enabled") => true, // 默认启用
                        (Value::Bool(false), "focused") => true, // 默认未聚焦
                        _ => false,
                    };
                    (!drop).then_some((k, v))
                })
                .collect(),
        ),
        // ...
    }
}
```
- 删除 null、空字符串/数组、默认布尔值
- **减少 context 占用**，关键优化点

#### 3. 错误处理
```rust
let result = match self.call_tool(name, &args) {
    Ok(out) => json!({ "content": out.content, "isError": out.is_error }),
    // 工具失败 → 结果而非协议错误，AI 可以看到并修正
    Err(e) => json!({
        "content": [text(format!("Error ({}): {e:#}", ops::error_code(&e)))],
        "isError": true
    }),
};
```
- 工具级错误 → `isError: true` + 错误码
- AI 可根据 `not_found` / `timeout` / `permission_denied` 调整策略

## CCP 借鉴方案

### 当前 CCP Computer Use 现状

**文件**: `crates/claude-codex-pro-core/src/computer_use.rs` (假设存在)

根据 `ComputerUseTab.tsx` 可知 CCP 已有：
- 状态查询：`ClaudeDesktopComputerUseStatusResult`
- 日志记录：`ComputerUseLogEntry`（tool, ok, error, x, y, keys, textLength）
- 平台检测：`platform` 字段

**推测实现**：
- 类似 Claude Desktop 的 `--mcp-computer-use` 单 exe 模式
- 可能已基于 MCP 协议
- 但缺少 Oculos 的完整工具集（树遍历、批量操作、等待）

### 建议改进方向

#### 1. 引入完整的无障碍树操作
**当前可能只有**：screenshot, click, move_mouse, type_text, press_keys

**Oculos 提供**：
- `list_windows` - 窗口枚举
- `find_elements` - 元素搜索（比树遍历快）
- `get_ui_tree` - 完整树（带深度限制）
- `wait_for_element` - 轮询等待（appears/gone）
- `batch_actions` - 批量操作（减少往返）

**实现路径**：
```rust
// crates/claude-codex-pro-core/src/computer_use/mod.rs
pub mod accessibility; // 新增：无障碍树抽象
pub mod windows_uia;   // Windows UIA 实现
pub mod mcp_bridge;    // MCP 协议桥接（复用 Oculos 的 schema）

pub trait AccessibilityBackend: Send + Sync {
    fn list_windows(&self) -> Result<Vec<WindowInfo>>;
    fn find_elements(&self, pid: u32, query: Option<&str>, ...) -> Result<Vec<UiElement>>;
    fn click_element(&self, id: &str) -> Result<()>;
    // ...
}
```

#### 2. 优化日志格式
**当前**：分散的 x, y, keys, textLength 字段

**Oculos**：统一的 action 名称 + 参数
```rust
pub struct ComputerUseLogEntry {
    pub timestamp_ms: u64,
    pub action: String,        // "click", "set_text", "send_keys"
    pub target: Option<String>, // element_id 或 (x, y)
    pub args: Option<Value>,   // {"text": "..."} 或 {"keys": "..."}
    pub ok: bool,
    pub error: Option<String>,
}
```

#### 3. 添加安全检查
**Oculos 的防护**：
- 窗口覆盖检测（`WindowFromPoint` 验证 PID）
- 屏幕外元素拒绝操作
- 修饰键失败时强制释放（防止卡死）

**CCP 应增加**：
```rust
// 在 click 前检查
if is_covered_by_other_process(element) {
    return Err(error::unsupported(
        "Element is covered by another window; bring its window to foreground first"
    ));
}
```

#### 4. 提供配置选项
**类似 Oculos 的可调参数**：
- `MAX_TREE_DEPTH`: 48（防止深度爆炸）
- `TEXT_CONTENT_LIMIT`: 4096（TextPattern 读取上限）
- `CURSOR_RESTORE_DELAY`: 50ms（鼠标复位延迟）

**CCP 可在 `SettingsStore` 添加**：
```rust
pub struct ComputerUseSettings {
    pub max_tree_depth: usize,
    pub wait_timeout_ms: u64,
    pub highlight_duration_ms: u64,
}
```

### 实施步骤

#### Phase 1: 无侵入式分析（1-2 天）
1. 将 Oculos `platform/windows.rs` 移植为独立 crate
2. 在 CCP 中集成，仅用于诊断（不对外暴露）
3. 对比 CCP 当前实现与 Oculos 的差异

#### Phase 2: 扩展工具集（3-5 天）
1. 在 `computer_use.rs` 添加 `find_elements` / `get_ui_tree`
2. Tauri 命令层暴露新工具
3. 前端 ComputerUseTab 显示树/搜索结果

#### Phase 3: MCP 协议对齐（2-3 天）
1. 复用 Oculos 的工具 schema
2. 确保与 Claude Desktop 的 MCP Computer Use 兼容
3. 添加批量操作支持

#### Phase 4: 测试与优化（持续）
1. 跨应用测试（记事本、Chrome、VS Code）
2. 错误处理完善（element_not_found 重试逻辑）
3. 性能优化（缓存、ID 复用）

## 关键代码参考

### Windows UIA 初始化
```rust
// 参考 oculos/platform/windows.rs:229-261
pub fn create_windows_backend() -> Result<WindowsUiBackend> {
    unsafe {
        // 物理像素模式
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
        
        // 保持 MTA 活跃
        CoIncrementMTAUsage()?;
        
        // 创建 UIA 客户端（优先 CUIAutomation8）
        let automation: IUIAutomation = 
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
            .or_else(|_| CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER))?;
        
        Ok(WindowsUiBackend { automation, registry: ElementRegistry::new() })
    }
}
```

### 元素查找（带条件下推）
```rust
// 参考 oculos/platform/windows.rs:383-475
pub fn find_elements(
    &self,
    pid: u32,
    query: Option<&str>,
    element_type: Option<ElementType>,
    interactive_only: bool,
) -> Result<Vec<UiElement>> {
    let root = self.element_from_hwnd(main_window(pid)?)?;
    
    // UIA 条件组合（服务器端过滤）
    let mut condition = self.automation.ControlViewCondition()?;
    if let Some(t) = element_type {
        let by_type = self.control_type_condition(&control_type_ids(t))?;
        condition = self.automation.CreateAndCondition(&condition, &by_type)?;
    }
    if let Some(q) = query {
        if let Some(by_query) = self.query_condition(q) {
            condition = self.automation.CreateAndCondition(&condition, &by_query)?;
        }
    }
    
    // 批量查找 + 客户端二次过滤
    let found = root.FindAllBuildCache(TreeScope_Subtree, &condition, &cache_request)?;
    let count = found.Length()?;
    let mut results = Vec::new();
    for i in 0..count {
        let element = found.GetElement(i)?;
        let node = cached_node(&element);
        if interactive_only && node.actions.is_empty() {
            continue;
        }
        self.register(&node.oculos_id, &element);
        results.push(node);
    }
    Ok(results)
}
```

### 批量操作
```rust
// 参考 oculos/ops.rs:298-352
pub fn run_batch(
    backend: &dyn UiBackend,
    steps: &[(String, Action)],
    stop_on_error: bool,
    delay_ms: u64,
) -> Vec<BatchResult> {
    let delay = Duration::from_millis(delay_ms.min(5_000));
    let mut results = Vec::with_capacity(steps.len());
    
    for (index, (id, action)) in steps.iter().enumerate() {
        if index > 0 && !delay.is_zero() {
            std::thread::sleep(delay);
        }
        
        let res = perform(backend, id, action);
        results.push(BatchResult {
            index,
            action: action.name().to_string(),
            element_id: id.clone(),
            success: res.is_ok(),
            error: res.err().map(|e| e.to_string()),
        });
        
        if res.is_err() && stop_on_error {
            break;
        }
    }
    results
}
```

## 许可与署名

**Oculos 许可**: MIT License
- ✅ 允许商业使用
- ✅ 允许修改和分发
- ⚠️ 必须保留原始版权声明和许可

**CCP 集成建议**：
```rust
// crates/claude-codex-pro-core/src/computer_use/windows_uia.rs
//! Windows UI Automation backend for CCP Computer Use.
//!
//! Portions of this implementation are adapted from OculOS (MIT License)
//! Copyright (c) 2024 grp06
//! https://github.com/grp06/oculos
//!
//! Key adaptations:
//! - Element caching strategy (BuildUpdatedCache)
//! - Click operation fallback chain (InvokePattern → TogglePattern → mouse)
//! - Foreground window focus bypass (AttachThreadInput)
```

## 参考资料

- Oculos 仓库: https://github.com/grp06/oculos
- Microsoft UI Automation: https://learn.microsoft.com/windows/win32/winauto/uiauto-uiautomationoverview
- Model Context Protocol: https://modelcontextprotocol.io/
- Claude Desktop Computer Use 文档: (内部)

---

**结论**：Oculos 提供了生产级的跨平台无障碍自动化参考实现，CCP 可借鉴其架构设计（三层抽象）、缓存优化（BuildUpdatedCache）和错误处理策略（分类错误码），显著提升 Computer Use 功能的可靠性和可用性。
