//! Windows UI Automation backend for CCP Computer Use.
//!
//! This module wraps Microsoft UI Automation API calls and provides a stable
//! interface for element discovery, interaction, and screenshot capture.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, anyhow};

use std::collections::hash_map::DefaultHasher;
use std::ffi::c_void;
use std::hash::{Hash, Hasher};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoIncrementMTAUsage,
    CoInitializeEx, CoUninitialize, SAFEARRAY,
};
use windows::Win32::System::Ole::{
    SafeArrayDestroy, SafeArrayGetDim, SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound,
};
use windows::Win32::System::Variant::VariantToInt32Array;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    AutomationElementMode_Full, CUIAutomation, CUIAutomation8, IUIAutomation,
    IUIAutomationCacheRequest, IUIAutomationElement, TreeScope, TreeScope_Subtree,
    UIA_AutomationIdPropertyId, UIA_BoundingRectanglePropertyId, UIA_ControlTypePropertyId,
    UIA_NamePropertyId, UIA_PROPERTY_ID, UIA_RuntimeIdPropertyId,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible,
};
#[cfg(target_os = "windows")]
use windows::core::BSTR;

use super::types::*;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum tree depth to prevent infinite recursion
const MAX_TREE_DEPTH: usize = 48;

/// Maximum text content length per element (reserved for future use)
#[allow(dead_code)]
const TEXT_CONTENT_LIMIT: i32 = 4096;

/// All properties we cache for fast tree/find operations
const CACHED_PROPERTIES: &[UIA_PROPERTY_ID] = &[
    UIA_NamePropertyId,
    UIA_ControlTypePropertyId,
    UIA_AutomationIdPropertyId,
    UIA_RuntimeIdPropertyId,
    UIA_BoundingRectanglePropertyId,
];

// ── COM initialization ────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
struct ComApartment {
    initialized: bool,
}

#[cfg(target_os = "windows")]
impl ComApartment {
    fn enter() -> Self {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        Self {
            initialized: hr.is_ok(),
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}

#[cfg(target_os = "windows")]
thread_local! {
    static COM_APARTMENT: ComApartment = ComApartment::enter();
}

#[cfg(target_os = "windows")]
fn ensure_com() {
    COM_APARTMENT.with(|_| {});
}

// ── Window helpers ────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn get_window_pid(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    pid
}

#[cfg(target_os = "windows")]
fn get_exe_name_from_pid(_pid: u32) -> String {
    // TODO: Implement using QueryFullProcessImageNameW
    String::new()
}

#[cfg(target_os = "windows")]
fn is_foreground_window(hwnd: HWND) -> bool {
    unsafe { GetForegroundWindow() == hwnd }
}

#[cfg(target_os = "windows")]
fn focus_window_impl(hwnd: HWND) -> Result<()> {
    use windows::Win32::UI::WindowsAndMessaging::{SW_RESTORE, SetForegroundWindow, ShowWindow};

    unsafe {
        // Restore if minimized
        let _ = ShowWindow(hwnd, SW_RESTORE);

        // Try to set foreground
        if !SetForegroundWindow(hwnd).as_bool() {
            return Err(anyhow!("Failed to set foreground window"));
        }
    }

    Ok(())
}

// ── Element registry ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct SafeElement(IUIAutomationElement);

#[cfg(target_os = "windows")]
unsafe impl Send for SafeElement {}
#[cfg(target_os = "windows")]
unsafe impl Sync for SafeElement {}

static FALLBACK_IDS: AtomicU64 = AtomicU64::new(0);

// ── Backend ───────────────────────────────────────────────────────────────────

pub struct WindowsUiaBackend {
    #[cfg(target_os = "windows")]
    automation: IUIAutomation,
    #[cfg(target_os = "windows")]
    registry: Mutex<HashMap<String, SafeElement>>,
}

#[cfg(target_os = "windows")]
unsafe impl Send for WindowsUiaBackend {}
#[cfg(target_os = "windows")]
unsafe impl Sync for WindowsUiaBackend {}

impl WindowsUiaBackend {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            unsafe {
                // Enable per-monitor DPI awareness
                let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

                // Keep MTA alive
                let _ = CoIncrementMTAUsage();
            }
            ensure_com();

            // Try CUIAutomation8 first (Windows 8+), fallback to CUIAutomation
            let automation: IUIAutomation = unsafe {
                match CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) {
                    Ok(a) => a,
                    Err(_) => CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                        .context("Failed to create UI Automation client")?,
                }
            };

            Ok(Self {
                automation,
                registry: Mutex::new(HashMap::new()),
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    fn register(&self, id: &str, element: &IUIAutomationElement) {
        self.registry
            .lock()
            .unwrap()
            .insert(id.to_string(), SafeElement(element.clone()));
    }

    #[cfg(target_os = "windows")]
    pub(super) fn lookup(&self, id: &str) -> Result<IUIAutomationElement> {
        self.registry
            .lock()
            .unwrap()
            .get(id)
            .map(|e| e.0.clone())
            .ok_or_else(|| anyhow!("Element not found: {}", id))
    }

    #[cfg(target_os = "windows")]
    unsafe fn cache_request(&self, scope: TreeScope) -> Result<IUIAutomationCacheRequest> {
        let build = || -> windows::core::Result<IUIAutomationCacheRequest> {
            unsafe {
                let request = self.automation.CreateCacheRequest()?;
                request.SetAutomationElementMode(AutomationElementMode_Full)?;
                request.SetTreeFilter(&self.automation.ControlViewCondition()?)?;
                request.SetTreeScope(scope)?;
                for &property in CACHED_PROPERTIES {
                    request.AddProperty(property)?;
                }
                Ok(request)
            }
        };
        build().context("Failed to create cache request")
    }

    #[cfg(target_os = "windows")]
    unsafe fn element_from_hwnd(&self, hwnd: HWND) -> Result<IUIAutomationElement> {
        unsafe {
            self.automation
                .ElementFromHandle(hwnd)
                .context("Failed to get element from window handle")
        }
    }

    pub fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        #[cfg(target_os = "windows")]
        {
            ensure_com();
            let mut result = Vec::new();

            unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
                unsafe {
                    let list = &mut *(lparam.0 as *mut Vec<HWND>);
                    list.push(hwnd);
                    BOOL(1)
                }
            }

            let mut windows: Vec<HWND> = Vec::new();
            unsafe {
                let _ = EnumWindows(
                    Some(enum_cb),
                    LPARAM(&mut windows as *mut Vec<HWND> as isize),
                );
            }

            for hwnd in windows {
                unsafe {
                    if !IsWindowVisible(hwnd).as_bool() {
                        continue;
                    }

                    let mut buf = [0u16; 512];
                    let copied = GetWindowTextW(hwnd, &mut buf);
                    if copied == 0 {
                        continue;
                    }
                    let title = String::from_utf16_lossy(&buf[..copied as usize]);

                    let pid = get_window_pid(hwnd);
                    let exe_name = get_exe_name_from_pid(pid);
                    let is_foreground = is_foreground_window(hwnd);

                    result.push(WindowInfo {
                        pid,
                        hwnd: hwnd.0 as usize,
                        title,
                        exe_name,
                        rect: Rect::default(),
                        // 已最小化的窗口其元素矩形约为 (-32000, -32000)，不能用于点击/绘制。
                        visible: !IsIconic(hwnd).as_bool(),
                        is_foreground,
                    });
                }
            }

            Ok(result)
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    pub fn get_tree(&self, hwnd: usize) -> Result<UiElement> {
        #[cfg(target_os = "windows")]
        {
            ensure_com();
            let hwnd = HWND(hwnd as *mut _);
            unsafe {
                let root = self.element_from_hwnd(hwnd)?;
                let request = self.cache_request(TreeScope_Subtree)?;
                let cached = root
                    .BuildUpdatedCache(&request)
                    .context("Failed to build cached tree")?;
                Ok(self.cached_subtree(&cached, 0))
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn cached_subtree(&self, element: &IUIAutomationElement, depth: usize) -> UiElement {
        unsafe {
            if depth > MAX_TREE_DEPTH {
                let id = self.element_id(element);
                return UiElement::depth_limit_placeholder(id);
            }

            let mut node = self.cached_node(element);

            // Register element to registry
            self.registry
                .lock()
                .unwrap()
                .insert(node.id.clone(), SafeElement(element.clone()));

            // Get children
            if let Ok(children) = element.GetCachedChildren() {
                if let Ok(count) = children.Length() {
                    for i in 0..count {
                        if let Ok(child) = children.GetElement(i) {
                            node.children.push(self.cached_subtree(&child, depth + 1));
                        }
                    }
                }
            }

            node
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn cached_node(&self, element: &IUIAutomationElement) -> UiElement {
        unsafe {
            let ctrl = element.CachedControlType().map(|c| c.0).unwrap_or(0);
            let element_type = self.control_type(ctrl);
            let id = self.element_id(element);

            let mut node = UiElement::new(id, element_type);
            node.label = self.bstr_opt(element.CachedName()).unwrap_or_default();
            node.automation_id = self.bstr_opt(element.CachedAutomationId());
            node.enabled = element
                .CachedIsEnabled()
                .map(|b| b.as_bool())
                .unwrap_or(true);

            // 只读缓存：BoundingRectangle 已在 CACHED_PROPERTIES 中随树一次取回，
            // 不再逐节点跨进程调用 Current*。最小化窗口的矩形约为 (-32000, -32000)，
            // 由调用方在取树前排除最小化窗口。
            if let Ok(rect_struct) = element.CachedBoundingRectangle() {
                node.rect = Rect {
                    x: rect_struct.left,
                    y: rect_struct.top,
                    width: rect_struct.right - rect_struct.left,
                    height: rect_struct.bottom - rect_struct.top,
                };
            }

            node
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn element_id(&self, element: &IUIAutomationElement) -> String {
        unsafe {
            match self.runtime_id(element) {
                Some(ints) => self.stable_id(&ints),
                None => self.stable_id(&format!(
                    "uia-no-runtime-id-{}",
                    FALLBACK_IDS.fetch_add(1, Ordering::Relaxed)
                )),
            }
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn runtime_id(&self, element: &IUIAutomationElement) -> Option<Vec<i32>> {
        unsafe {
            // Try cached value first (no cross-process call)
            if let Ok(value) = element.GetCachedPropertyValue(UIA_RuntimeIdPropertyId) {
                let mut buf = [0i32; 32];
                let mut count = 0u32;
                if VariantToInt32Array(&value, &mut buf, &mut count).is_ok() {
                    let count = (count as usize).min(buf.len());
                    if count > 0 {
                        return Some(buf[..count].to_vec());
                    }
                }
            }
            // Fall back to live getter
            let array = element.GetRuntimeId().ok()?;
            self.safearray_to_i32s(array)
        }
    }

    #[cfg(target_os = "windows")]
    unsafe fn safearray_to_i32s(&self, array: *mut SAFEARRAY) -> Option<Vec<i32>> {
        unsafe {
            if array.is_null() {
                return None;
            }
            // RAII guard to destroy SAFEARRAY on scope exit
            struct SafeArrayGuard(*mut SAFEARRAY);
            impl Drop for SafeArrayGuard {
                fn drop(&mut self) {
                    if !self.0.is_null() {
                        unsafe {
                            let _ = SafeArrayDestroy(self.0);
                        }
                    }
                }
            }
            let _guard = SafeArrayGuard(array);

            // Check it's a 1-D array of 4-byte integers
            if SafeArrayGetDim(array) != 1 || (*array).cbElements != 4 {
                return None;
            }

            let lower = SafeArrayGetLBound(array, 1).ok()?;
            let upper = SafeArrayGetUBound(array, 1).ok()?;
            if upper < lower || upper - lower >= 64 {
                return None;
            }

            let mut out = Vec::with_capacity((upper - lower + 1) as usize);
            for index in lower..=upper {
                let mut value = 0i32;
                SafeArrayGetElement(array, &index, &mut value as *mut i32 as *mut c_void).ok()?;
                out.push(value);
            }
            Some(out)
        }
    }

    #[cfg(target_os = "windows")]
    fn stable_id<T: Hash>(&self, key: &T) -> String {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    #[cfg(target_os = "windows")]
    fn bstr_opt(&self, value: windows::core::Result<BSTR>) -> Option<String> {
        value.ok().and_then(|b| {
            let s = b.to_string();
            if s.is_empty() { None } else { Some(s) }
        })
    }

    #[cfg(target_os = "windows")]
    fn control_type(&self, id: i32) -> ElementType {
        match id {
            50000 => ElementType::Button,
            50001 => ElementType::Calendar,
            50002 => ElementType::CheckBox,
            50003 => ElementType::ComboBox,
            50004 => ElementType::Edit,
            50005 => ElementType::Link,
            50006 => ElementType::Image,
            50007 => ElementType::ListItem,
            50008 => ElementType::ListBox,
            50009 => ElementType::Menu,
            50010 => ElementType::MenuBar,
            50011 => ElementType::MenuItem,
            50012 => ElementType::ProgressBar,
            50013 => ElementType::RadioButton,
            50014 => ElementType::ScrollBar,
            50015 => ElementType::Slider,
            50016 => ElementType::Spinner,
            50017 => ElementType::StatusBar,
            50018 => ElementType::TabControl,
            50019 => ElementType::TabItem,
            50020 => ElementType::Text,
            50021 => ElementType::ToolBar,
            50022 => ElementType::ToolTip,
            50023 => ElementType::TreeView,
            50024 => ElementType::TreeItem,
            50025 => ElementType::Custom,
            50026 => ElementType::Group,
            50027 => ElementType::Thumb,
            50028 => ElementType::DataGrid,
            50029 => ElementType::DataItem,
            50030 => ElementType::Document,
            50031 => ElementType::SplitButton,
            50032 => ElementType::Window,
            50033 => ElementType::Pane,
            50034 => ElementType::Header,
            50035 => ElementType::HeaderItem,
            50036 => ElementType::Table,
            50037 => ElementType::TitleBar,
            50038 => ElementType::Separator,
            _ => ElementType::Unknown,
        }
    }

    pub fn find_elements(&self, hwnd: usize, params: &FindParams) -> Result<Vec<UiElement>> {
        #[cfg(target_os = "windows")]
        {
            ensure_com();

            // Get the tree and search in-memory
            let tree = self.get_tree(hwnd)?;
            Ok(crate::claude_desktop_computer_use::uia::find::find_in_tree(
                &tree, params, 100,
            ))
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (hwnd, params);
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }

    pub fn focus_window(&self, hwnd: usize) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(hwnd as *mut _);
            focus_window_impl(hwnd)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = hwnd;
            Err(anyhow!("Windows UIA backend is only available on Windows"))
        }
    }
}
