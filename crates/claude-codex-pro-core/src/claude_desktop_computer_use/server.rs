use super::tools::{
    Backend, FAILSAFE_SIZE, MouseButton, normalize_key, screenshot_size, to_screen_point,
    tool_definitions,
};
use base64::Engine;
use serde_json::{Value, json};

const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";
const ACTION_TOOLS: &[&str] = &[
    "click",
    "move_mouse",
    "drag",
    "scroll",
    "type_text",
    "press_keys",
];

/// Decides whether tools may run, and handles the emergency stop.
pub trait Gate {
    fn enabled(&self) -> bool;
    fn emergency_stop(&mut self);
}

/// Production gate backed by CCP settings. Re-read on every call so the CCP
/// toggle takes effect without restarting Claude Desktop.
pub struct SettingsGate {
    store: crate::settings::SettingsStore,
    stopped: bool,
}

impl SettingsGate {
    pub fn new(store: crate::settings::SettingsStore) -> Self {
        Self {
            store,
            stopped: false,
        }
    }
}

impl Gate for SettingsGate {
    fn enabled(&self) -> bool {
        !self.stopped
            && self
                .store
                .load()
                .map(|settings| settings.claude_desktop_computer_use_enabled)
                .unwrap_or(false)
    }

    fn emergency_stop(&mut self) {
        // Latch in-process even if persisting fails.
        self.stopped = true;
        let _ = self
            .store
            .update_boolean_preserving_profiles("claudeDesktopComputerUseEnabled", false);
    }
}

type AuditSink = Box<dyn FnMut(&Value)>;

pub struct ComputerUseServer<B: Backend, G: Gate> {
    backend: B,
    gate: G,
    audit: AuditSink,
}

impl<B: Backend, G: Gate> ComputerUseServer<B, G> {
    pub fn new(backend: B, gate: G) -> Self {
        Self::with_audit(
            backend,
            gate,
            Box::new(|detail| {
                let _ = crate::diagnostic_log::append_diagnostic_log("claude_computer_use", detail);
            }),
        )
    }

    pub fn with_audit(backend: B, gate: G, audit: AuditSink) -> Self {
        Self {
            backend,
            gate,
            audit,
        }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Handle one JSON-RPC line. Returns `None` for notifications.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let message: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                return Some(error_response(
                    Value::Null,
                    -32700,
                    &format!("Parse error: {error}"),
                ));
            }
        };
        let id = message.get("id").cloned()?;
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let response = match method {
            "initialize" => json!({
                "protocolVersion": params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or(DEFAULT_PROTOCOL_VERSION),
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": super::COMPUTER_USE_SERVER_NAME,
                    "version": env!("CARGO_PKG_VERSION")
                },
                "instructions": "先调用 screenshot 观察屏幕，所有坐标使用截图像素坐标。用户可把鼠标甩到屏幕左上角急停。"
            }),
            "ping" => json!({}),
            "tools/list" => json!({ "tools": tool_definitions() }),
            "tools/call" => self.call_tool(&params),
            _ => {
                return Some(error_response(
                    id,
                    -32601,
                    &format!("Method not found: {method}"),
                ));
            }
        };
        Some(json!({ "jsonrpc": "2.0", "id": id, "result": response }).to_string())
    }

    fn call_tool(&mut self, params: &Value) -> Value {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let result = self.run_tool(name, &args);
        let mut audit = json!({ "tool": name, "ok": result.is_ok() });
        if let Some(object) = audit.as_object_mut() {
            for key in [
                "x", "y", "from_x", "from_y", "to_x", "to_y", "dx", "dy", "keys",
            ] {
                if let Some(value) = args.get(key) {
                    object.insert(key.to_string(), value.clone());
                }
            }
            if let Some(text) = args.get("text").and_then(Value::as_str) {
                object.insert("textChars".into(), json!(text.chars().count()));
            }
            if let Err(error) = &result {
                object.insert("error".into(), json!(error));
            }
        }
        (self.audit)(&audit);
        match result {
            Ok(content) => json!({ "content": content, "isError": false }),
            Err(error) => json!({
                "content": [{ "type": "text", "text": error }],
                "isError": true
            }),
        }
    }

    fn run_tool(&mut self, name: &str, args: &Value) -> Result<Value, String> {
        if !self.gate.enabled() {
            return Err(
                "Computer Use 未开启：请在 Claude Codex Pro「工具与插件」中开启 Claude Desktop Computer Use。"
                    .into(),
            );
        }
        if ACTION_TOOLS.contains(&name) && self.failsafe_hit() {
            self.gate.emergency_stop();
            return Err("已急停：检测到鼠标位于屏幕左上角，Computer Use 已关闭。请在 Claude Codex Pro 中重新开启。".into());
        }
        let screen = self.backend.screen_size().map_err(err)?;
        let point = |x_key: &str, y_key: &str| -> Result<(i32, i32), String> {
            let (x, y) = to_screen_point(screen, number(args, x_key)?, number(args, y_key)?)?;
            // The corner is reserved for the user's emergency stop; letting the
            // agent park the cursor there would trip it on the next call.
            if x < FAILSAFE_SIZE && y < FAILSAFE_SIZE {
                return Err("屏幕左上角为急停保留区域，不能作为操作坐标。".into());
            }
            Ok((x, y))
        };
        let done = |message: String| Ok(json!([{ "type": "text", "text": message }]));
        match name {
            "screenshot" => self.screenshot(),
            "click" => {
                let (x, y) = point("x", "y")?;
                let button = MouseButton::parse(args.get("button").and_then(Value::as_str))?;
                let count = args
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or(1)
                    .clamp(1, 3) as u32;
                self.backend.click(x, y, button, count).map_err(err)?;
                done(format!("已点击（{button:?} ×{count}）。"))
            }
            "move_mouse" => {
                let (x, y) = point("x", "y")?;
                self.backend.move_mouse(x, y).map_err(err)?;
                done("已移动鼠标。".into())
            }
            "drag" => {
                let from = point("from_x", "from_y")?;
                let to = point("to_x", "to_y")?;
                self.backend.drag(from, to).map_err(err)?;
                done("已拖拽。".into())
            }
            "scroll" => {
                let (x, y) = point("x", "y")?;
                let dx = args
                    .get("dx")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .clamp(-50, 50) as i32;
                let dy = args
                    .get("dy")
                    .and_then(Value::as_i64)
                    .unwrap_or(0)
                    .clamp(-50, 50) as i32;
                self.backend.scroll(x, y, dx, dy).map_err(err)?;
                done(format!("已滚动 dx={dx} dy={dy}。"))
            }
            "type_text" => {
                let text = args
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or("缺少 text")?;
                self.backend.type_text(text).map_err(err)?;
                done(format!("已输入 {} 个字符。", text.chars().count()))
            }
            "press_keys" => {
                let keys = parse_keys(args)?;
                self.backend.press_keys(&keys).map_err(err)?;
                done(format!("已按下 {}。", keys.join("+")))
            }
            "cursor_position" => {
                let (x, y) = self.backend.cursor_position().map_err(err)?;
                let shot = screenshot_size(screen);
                let sx = (x as f64 * shot.width as f64 / screen.width.max(1) as f64).round();
                let sy = (y as f64 * shot.height as f64 / screen.height.max(1) as f64).round();
                done(format!("鼠标位于截图坐标 ({sx}, {sy})。"))
            }
            "wait" => {
                let seconds = number(args, "seconds")?.clamp(0.0, 10.0);
                std::thread::sleep(std::time::Duration::from_secs_f64(seconds));
                done(format!("已等待 {seconds} 秒。"))
            }
            other => Err(format!("未知工具：{other}")),
        }
    }

    fn failsafe_hit(&self) -> bool {
        matches!(
            self.backend.cursor_position(),
            Ok((x, y)) if x < FAILSAFE_SIZE && y < FAILSAFE_SIZE
        )
    }

    fn screenshot(&self) -> Result<Value, String> {
        let screen = self.backend.capture().map_err(err)?;
        let shot = screenshot_size(screen.size);
        let resized = image::imageops::resize(
            &screen.image,
            shot.width,
            shot.height,
            image::imageops::FilterType::Triangle,
        );
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80)
            .encode_image(&resized)
            .map_err(err)?;
        Ok(json!([
            {
                "type": "image",
                "data": base64::engine::general_purpose::STANDARD.encode(&jpeg),
                "mimeType": "image/jpeg"
            },
            {
                "type": "text",
                "text": format!(
                    "截图尺寸 {}x{}（屏幕 {}x{}），坐标请使用截图像素。",
                    shot.width, shot.height, screen.size.width, screen.size.height
                )
            }
        ]))
    }
}

fn parse_keys(args: &Value) -> Result<Vec<String>, String> {
    let raw = args
        .get("keys")
        .and_then(Value::as_array)
        .filter(|keys| !keys.is_empty())
        .ok_or("keys 必须是非空数组")?;
    raw.iter()
        .map(|key| {
            let key = key.as_str().ok_or("keys 只能包含字符串")?;
            normalize_key(key).ok_or_else(|| format!("不支持的按键：{key}"))
        })
        .collect()
}

fn number(args: &Value, key: &str) -> Result<f64, String> {
    args.get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("缺少数字参数 {key}"))
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }).to_string()
}
