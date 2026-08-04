#[cfg(windows)]
use std::ffi::{OsStr, OsString};
#[cfg(windows)]
use std::iter::once;
#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
use std::path::{Path, PathBuf};

#[cfg(windows)]
use anyhow::Context;
#[cfg(windows)]
use windows::Win32::Foundation::{
    BOOL, CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, HANDLE, HWND, LPARAM, MAX_PATH,
    WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0, WPARAM,
};
#[cfg(windows)]
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize, IPersistFile,
};
#[cfg(windows)]
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
#[cfg(windows)]
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
#[cfg(windows)]
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
#[cfg(windows)]
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_SET_VALUE, REG_EXPAND_SZ, REG_ROUTINE_FLAGS,
    REG_SZ, REG_VALUE_TYPE, RRF_NOEXPAND, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ, RegCloseKey,
    RegCreateKeyW, RegDeleteKeyW, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    AttachThreadInput, CreateMutexW, GetCurrentThreadId, INFINITE, OpenProcess,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, QueryFullProcessImageNameW, ReleaseMutex,
    TerminateProcess, WaitForSingleObject,
};
#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY,
    VK_CONTROL, VK_F12, VK_I, VK_N, VK_RETURN, VK_SHIFT, VK_V,
};
#[cfg(windows)]
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, IShellLinkW, KF_FLAG_DEFAULT, SHGetKnownFolderPath, ShellExecuteW, ShellLink,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWMINNOACTIVE;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, HWND_BROADCAST, IsIconic, IsWindowVisible, SMTO_ABORTIFHUNG,
    SW_RESTORE, SendMessageTimeoutW, SetForegroundWindow, ShowWindow, WM_SETTINGCHANGE,
};
#[cfg(windows)]
use windows::core::{Interface, PCWSTR, PWSTR};

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const CF_UNICODETEXT_FORMAT: u32 = 13;

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryStringValue {
    pub value: String,
    pub value_type: REG_VALUE_TYPE,
}

#[cfg(windows)]
pub(crate) struct NamedMutexGuard {
    handle: HANDLE,
}

#[cfg(windows)]
impl Drop for NamedMutexGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.handle);
            let _ = CloseHandle(self.handle);
        }
    }
}

#[cfg(windows)]
pub(crate) fn acquire_named_mutex(name: &str) -> anyhow::Result<NamedMutexGuard> {
    let name = wide_null(name);
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }
        .context("创建 Codex 凭据环境互斥体失败")?;
    let wait_result = unsafe { WaitForSingleObject(handle, INFINITE) };
    if wait_result != WAIT_OBJECT_0 && wait_result != WAIT_ABANDONED {
        let _ = unsafe { CloseHandle(handle) };
        if wait_result == WAIT_FAILED {
            return Err(std::io::Error::last_os_error()).context("等待 Codex 凭据环境互斥体失败");
        }
        anyhow::bail!("等待 Codex 凭据环境互斥体返回意外状态")
    }
    Ok(NamedMutexGuard { handle })
}

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsProcessInfo {
    pub process_id: u32,
    pub parent_process_id: u32,
    pub exe_file: String,
    pub executable_path: Option<PathBuf>,
}

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundWindowInfo {
    pub process_id: u32,
    pub title: Option<String>,
}

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessWindowInfo {
    pub process_id: u32,
    pub title: Option<String>,
}

#[cfg(windows)]
pub struct ComApartment;

#[cfg(windows)]
impl ComApartment {
    pub fn init() -> windows::core::Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        }
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutSpec {
    pub path: PathBuf,
    pub target: PathBuf,
    pub arguments: String,
    pub working_directory: Option<PathBuf>,
    pub description: String,
    pub icon: Option<PathBuf>,
    pub show_minimized: bool,
}

#[cfg(windows)]
pub fn create_shortcut(spec: &ShortcutSpec) -> anyhow::Result<()> {
    if let Some(parent) = spec.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _com = ComApartment::init().context("初始化 COM 失败")?;
    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .context("创建 ShellLink COM 对象失败")?;
        shell_link
            .SetPath(PCWSTR(wide_null(spec.target.as_os_str()).as_ptr()))
            .context("设置快捷方式目标失败")?;
        shell_link
            .SetArguments(PCWSTR(wide_null(spec.arguments.as_str()).as_ptr()))
            .context("设置快捷方式参数失败")?;
        if let Some(working_directory) = &spec.working_directory {
            shell_link
                .SetWorkingDirectory(PCWSTR(wide_null(working_directory.as_os_str()).as_ptr()))
                .context("设置快捷方式工作目录失败")?;
        }
        shell_link
            .SetDescription(PCWSTR(wide_null(spec.description.as_str()).as_ptr()))
            .context("设置快捷方式描述失败")?;
        if let Some(icon) = &spec.icon {
            shell_link
                .SetIconLocation(PCWSTR(wide_null(icon.as_os_str()).as_ptr()), 0)
                .context("设置快捷方式图标失败")?;
        }
        if spec.show_minimized {
            shell_link
                .SetShowCmd(SW_SHOWMINNOACTIVE)
                .context("设置快捷方式窗口模式失败")?;
        }
        let persist_file: IPersistFile = shell_link.cast().context("获取 IPersistFile 失败")?;
        persist_file
            .Save(PCWSTR(wide_null(spec.path.as_os_str()).as_ptr()), true)
            .context("保存快捷方式失败")?;
    }
    Ok(())
}

#[cfg(windows)]
pub fn desktop_dir() -> Option<PathBuf> {
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None).ok()?;
        let value = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.as_ptr().cast()));
        value
    }
}

#[cfg(windows)]
pub fn open_url(url: &str) -> anyhow::Result<()> {
    shell_open(url)
}

#[cfg(windows)]
pub fn open_path(path: &Path) -> anyhow::Result<()> {
    shell_open(path.as_os_str())
}

#[cfg(windows)]
fn shell_open(target: impl AsRef<OsStr>) -> anyhow::Result<()> {
    let operation = wide_null("open");
    let file = wide_null(target);
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(file.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWMINNOACTIVE,
        )
    };
    let code = result.0 as isize;
    if code <= 32 {
        anyhow::bail!("ShellExecuteW returned {code}");
    }
    Ok(())
}

#[cfg(windows)]
pub fn set_current_user_string_value(subkey: &str, name: &str, value: &str) -> anyhow::Result<()> {
    set_current_user_registry_string_value(
        subkey,
        name,
        &RegistryStringValue {
            value: value.to_string(),
            value_type: REG_SZ,
        },
    )
}

#[cfg(windows)]
pub(crate) fn set_current_user_registry_string_value(
    subkey: &str,
    name: &str,
    value: &RegistryStringValue,
) -> anyhow::Result<()> {
    if value.value_type != REG_SZ && value.value_type != REG_EXPAND_SZ {
        anyhow::bail!("不支持的注册表字符串类型")
    }
    with_created_current_user_key(subkey, |key| {
        let encoded = wide_null(&value.value);
        let bytes = slice_as_u8(&encoded);
        unsafe {
            RegSetValueExW(
                key,
                PCWSTR(wide_null(name).as_ptr()),
                0,
                value.value_type,
                Some(bytes),
            )
        }
        .ok()
        .with_context(|| format!("写入注册表值 {subkey}\\{name} 失败"))
    })?;
    // Environment changes only reach future process environment blocks after
    // the user shell reloads them.
    if subkey.eq_ignore_ascii_case(WINDOWS_USER_ENVIRONMENT_KEY) {
        broadcast_environment_change();
    }
    Ok(())
}

#[cfg(windows)]
const WINDOWS_USER_ENVIRONMENT_KEY: &str = "Environment";

#[cfg(windows)]
fn broadcast_environment_change() {
    let param = wide_null("Environment");
    unsafe {
        // SMTO_ABORTIFHUNG + a short timeout so a hung top-level window can never
        // block the provider switch. The result is advisory, so ignore it.
        let mut result = 0usize;
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(param.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            Some(&mut result as *mut usize as *mut _),
        );
    }
}

#[cfg(windows)]
pub fn delete_current_user_value(subkey: &str, name: &str) -> anyhow::Result<()> {
    let broadcast_change = subkey.eq_ignore_ascii_case(WINDOWS_USER_ENVIRONMENT_KEY);
    let subkey_name = subkey.to_string();
    let subkey = wide_null(subkey);
    let name = wide_null(name);
    let mut key = HKEY::default();
    let open_result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
    };
    if open_result == ERROR_FILE_NOT_FOUND {
        return Ok(());
    }
    open_result
        .ok()
        .with_context(|| format!("打开注册表项 {subkey_name} 失败"))?;
    let _guard = RegistryKeyGuard(key);
    let delete_result = unsafe { RegDeleteValueW(key, PCWSTR(name.as_ptr())) };
    if delete_result != ERROR_FILE_NOT_FOUND {
        delete_result
            .ok()
            .with_context(|| format!("删除注册表值 {subkey_name} 失败"))?;
    }
    if broadcast_change {
        broadcast_environment_change();
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn current_user_registry_string_value_result(
    subkey: &str,
    name: &str,
) -> anyhow::Result<Option<RegistryStringValue>> {
    registry_string_value_result(HKEY_CURRENT_USER, subkey, name)
}

#[cfg(windows)]
pub fn local_machine_string_value(subkey: &str, name: &str) -> Option<String> {
    registry_string_value_result(HKEY_LOCAL_MACHINE, subkey, name)
        .ok()
        .flatten()
        .map(|value| value.value)
}

#[cfg(windows)]
fn registry_string_value_result(
    root: HKEY,
    subkey: &str,
    name: &str,
) -> anyhow::Result<Option<RegistryStringValue>> {
    let subkey_label = subkey.to_string();
    let name_label = name.to_string();
    let subkey = wide_null(subkey);
    let name = wide_null(name);
    let flags = REG_ROUTINE_FLAGS(RRF_RT_REG_SZ.0 | RRF_RT_REG_EXPAND_SZ.0 | RRF_NOEXPAND.0);
    let mut value_type = REG_VALUE_TYPE::default();
    let mut size = 0u32;
    let size_result = unsafe {
        RegGetValueW(
            root,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(name.as_ptr()),
            flags,
            Some(&mut value_type),
            None,
            Some(&mut size),
        )
    };
    if size_result == ERROR_FILE_NOT_FOUND || size_result == ERROR_PATH_NOT_FOUND {
        return Ok(None);
    }
    size_result
        .ok()
        .with_context(|| format!("读取注册表值 {subkey_label}\\{name_label} 大小失败"))?;
    if size == 0 {
        return Ok(None);
    }
    let mut value = vec![0u16; (size as usize).div_ceil(2)];
    let read_result = unsafe {
        RegGetValueW(
            root,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(name.as_ptr()),
            flags,
            Some(&mut value_type),
            Some(value.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if read_result == ERROR_FILE_NOT_FOUND || read_result == ERROR_PATH_NOT_FOUND {
        return Ok(None);
    }
    read_result
        .ok()
        .with_context(|| format!("读取注册表值 {subkey_label}\\{name_label} 失败"))?;
    let len = value.iter().position(|ch| *ch == 0).unwrap_or(value.len());
    let value = String::from_utf16_lossy(&value[..len]);
    if value.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(RegistryStringValue { value, value_type }))
}

#[cfg(windows)]
pub fn delete_current_user_key(subkey: &str) -> anyhow::Result<()> {
    let subkey = wide_null(subkey);
    unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(subkey.as_ptr())) }
        .ok()
        .or_else(|_| Ok(()))
}

#[cfg(windows)]
pub fn enumerate_processes() -> Vec<WindowsProcessInfo> {
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return Vec::new();
    };
    if snapshot.is_invalid() {
        return Vec::new();
    }
    let _guard = HandleGuard(snapshot);
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut processes = Vec::new();
    if unsafe { Process32FirstW(snapshot, &mut entry) }.is_err() {
        return Vec::new();
    }
    loop {
        let process_id = entry.th32ProcessID;
        processes.push(WindowsProcessInfo {
            process_id,
            parent_process_id: entry.th32ParentProcessID,
            exe_file: nul_terminated_wide_to_string(&entry.szExeFile),
            executable_path: query_process_image_path(process_id),
        });
        if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
            break;
        }
    }
    processes
}

#[cfg(windows)]
pub fn terminate_process(process_id: u32) -> bool {
    let Ok(handle) = (unsafe {
        OpenProcess(
            PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            process_id,
        )
    }) else {
        return false;
    };
    if handle.is_invalid() {
        return false;
    }
    let _guard = HandleGuard(handle);
    unsafe { TerminateProcess(handle, 0) }.is_ok()
}

#[cfg(windows)]
pub fn activate_process_window(process_id: u32) -> bool {
    let mut state = ActivateWindowState {
        process_id,
        hwnd: HWND::default(),
    };
    unsafe {
        let _ = EnumWindows(
            Some(find_process_window_proc),
            LPARAM((&mut state as *mut ActivateWindowState) as isize),
        );
    }
    if state.hwnd.is_invalid() {
        return false;
    }
    unsafe {
        if IsIconic(state.hwnd).as_bool() {
            let _ = ShowWindow(state.hwnd, SW_RESTORE);
        }
        if SetForegroundWindow(state.hwnd).as_bool() {
            return true;
        }
    }
    force_foreground_window(state.hwnd)
}

#[cfg(windows)]
fn force_foreground_window(hwnd: HWND) -> bool {
    unsafe {
        let current_thread = GetCurrentThreadId();
        let foreground_hwnd = GetForegroundWindow();
        let foreground_thread = if foreground_hwnd.is_invalid() {
            0
        } else {
            GetWindowThreadProcessId(foreground_hwnd, None)
        };
        let target_thread = GetWindowThreadProcessId(hwnd, None);
        if target_thread == 0 {
            return false;
        }

        let attached_current = target_thread != current_thread
            && AttachThreadInput(current_thread, target_thread, true).as_bool();
        let attached_foreground = foreground_thread != 0
            && foreground_thread != target_thread
            && AttachThreadInput(foreground_thread, target_thread, true).as_bool();

        let _ = BringWindowToTop(hwnd);
        let focused = SetForegroundWindow(hwnd).as_bool();

        if attached_foreground {
            let _ = AttachThreadInput(foreground_thread, target_thread, false);
        }
        if attached_current {
            let _ = AttachThreadInput(current_thread, target_thread, false);
        }

        focused
    }
}

#[cfg(windows)]
pub fn foreground_window_info() -> Option<ForegroundWindowInfo> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    if process_id == 0 {
        return None;
    }
    Some(ForegroundWindowInfo {
        process_id,
        title: window_title(hwnd),
    })
}

#[cfg(windows)]
pub fn visible_window_infos_for_process(process_id: u32) -> Vec<ProcessWindowInfo> {
    let mut state = CollectWindowState {
        process_id,
        windows: Vec::new(),
    };
    unsafe {
        let _ = EnumWindows(
            Some(collect_process_windows_proc),
            LPARAM((&mut state as *mut CollectWindowState) as isize),
        );
    }
    state.windows
}

#[cfg(windows)]
fn window_title(hwnd: HWND) -> Option<String> {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return None;
    }
    let mut buffer = vec![0u16; len as usize + 1];
    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if copied <= 0 {
        return None;
    }
    Some(
        OsString::from_wide(&buffer[..copied as usize])
            .to_string_lossy()
            .trim()
            .to_string(),
    )
    .filter(|title| !title.is_empty())
}

#[cfg(windows)]
pub fn set_clipboard_text(text: &str) -> anyhow::Result<()> {
    let clipboard = ClipboardGuard::open()?;
    unsafe {
        EmptyClipboard().ok().context("clear clipboard failed")?;
    }
    let wide = wide_null(text);
    let byte_len = wide.len() * std::mem::size_of::<u16>();
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, byte_len) }
        .context("allocate clipboard buffer failed")?;
    if handle.is_invalid() {
        anyhow::bail!("allocate clipboard buffer failed");
    }
    let lock = unsafe { GlobalLock(handle) };
    if lock.is_null() {
        anyhow::bail!("lock clipboard buffer failed");
    }
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr().cast::<u8>(), lock.cast::<u8>(), byte_len);
        let _ = GlobalUnlock(handle);
        SetClipboardData(CF_UNICODETEXT_FORMAT, HANDLE(handle.0))
            .ok()
            .context("set clipboard text failed")?;
    }
    drop(clipboard);
    Ok(())
}

#[cfg(windows)]
pub fn send_ctrl_v() -> bool {
    send_key_sequence(&[
        key_input(VK_CONTROL, false),
        key_input(VK_V, false),
        key_input(VK_V, true),
        key_input(VK_CONTROL, true),
    ])
}

#[cfg(windows)]
pub fn send_ctrl_n() -> bool {
    send_key_sequence(&[
        key_input(VK_CONTROL, false),
        key_input(VK_N, false),
        key_input(VK_N, true),
        key_input(VK_CONTROL, true),
    ])
}

#[cfg(windows)]
pub fn send_ctrl_shift_i() -> bool {
    send_key_sequence(&[
        key_input(VK_CONTROL, false),
        key_input(VK_SHIFT, false),
        key_input(VK_I, false),
        key_input(VK_I, true),
        key_input(VK_SHIFT, true),
        key_input(VK_CONTROL, true),
    ])
}

#[cfg(windows)]
pub fn send_f12() -> bool {
    send_key_sequence(&[key_input(VK_F12, false), key_input(VK_F12, true)])
}

#[cfg(windows)]
pub fn send_enter() -> bool {
    send_key_sequence(&[key_input(VK_RETURN, false), key_input(VK_RETURN, true)])
}

#[cfg(windows)]
fn query_process_image_path(process_id: u32) -> Option<PathBuf> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };
    if handle.is_invalid() {
        return None;
    }
    let _guard = HandleGuard(handle);
    let mut buffer = vec![0u16; MAX_PATH as usize * 4];
    let mut len = buffer.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            handle,
            Default::default(),
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        )
        .ok()?;
    }
    Some(PathBuf::from(OsString::from_wide(&buffer[..len as usize])))
}

#[cfg(windows)]
pub(crate) fn process_exists(process_id: u32) -> bool {
    query_process_image_path(process_id).is_some()
}

#[cfg(windows)]
fn send_key_sequence(inputs: &[INPUT]) -> bool {
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    sent == inputs.len() as u32
}

#[cfg(windows)]
fn key_input(key: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if key_up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(windows)]
struct ClipboardGuard;

#[cfg(windows)]
impl ClipboardGuard {
    fn open() -> anyhow::Result<Self> {
        unsafe {
            OpenClipboard(HWND::default())
                .ok()
                .context("open clipboard failed")?;
        }
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

#[cfg(windows)]
struct ActivateWindowState {
    process_id: u32,
    hwnd: HWND,
}

#[cfg(windows)]
struct CollectWindowState {
    process_id: u32,
    windows: Vec<ProcessWindowInfo>,
}

#[cfg(windows)]
unsafe extern "system" fn find_process_window_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = unsafe { &mut *(lparam.0 as *mut ActivateWindowState) };
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return BOOL(1);
    }
    let mut window_process_id = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut window_process_id));
    }
    if window_process_id == state.process_id {
        state.hwnd = hwnd;
        return BOOL(0);
    }
    BOOL(1)
}

#[cfg(windows)]
unsafe extern "system" fn collect_process_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = unsafe { &mut *(lparam.0 as *mut CollectWindowState) };
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return BOOL(1);
    }
    let mut process_id = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process_id)) };
    if process_id != state.process_id {
        return BOOL(1);
    }
    state.windows.push(ProcessWindowInfo {
        process_id,
        title: window_title(hwnd),
    });
    BOOL(1)
}

#[cfg(windows)]
fn with_created_current_user_key<T>(
    subkey: &str,
    f: impl FnOnce(HKEY) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyW(
            HKEY_CURRENT_USER,
            PCWSTR(wide_null(subkey).as_ptr()),
            &mut key,
        )
    }
    .ok()
    .with_context(|| format!("打开注册表键 HKCU\\{subkey} 失败"))?;
    let _guard = RegistryKeyGuard(key);
    f(key)
}

#[cfg(windows)]
fn slice_as_u8(value: &[u16]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), std::mem::size_of_val(value)) }
}

#[cfg(windows)]
fn wide_null(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(once(0)).collect()
}

#[cfg(windows)]
fn nul_terminated_wide_to_string(value: &[u16]) -> String {
    let len = value.iter().position(|ch| *ch == 0).unwrap_or(value.len());
    OsString::from_wide(&value[..len])
        .to_string_lossy()
        .to_string()
}

#[cfg(windows)]
struct HandleGuard(HANDLE);

#[cfg(windows)]
impl Drop for HandleGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(windows)]
struct RegistryKeyGuard(HKEY);

#[cfg(windows)]
impl Drop for RegistryKeyGuard {
    fn drop(&mut self) {
        let _ = unsafe { RegCloseKey(self.0) };
    }
}
