use serde_json::{Value, json};

/// Largest screenshot edge sizes sent to the model. Coordinates in tool
/// arguments are expressed in this (possibly downscaled) image space.
pub const MAX_SCREENSHOT_WIDTH: u32 = 1280;
pub const MAX_SCREENSHOT_HEIGHT: u32 = 800;
/// Moving the real mouse into this top-left square triggers the emergency stop.
pub const FAILSAFE_SIZE: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
}

/// A captured primary display. `size` is the input coordinate space
/// (physical pixels on Windows, points on macOS); `image` may be larger
/// (Retina) and is resized before encoding.
pub struct Screen {
    pub size: ScreenSize,
    pub image: image::RgbImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("left") {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "middle" => Ok(Self::Middle),
            other => Err(format!("不支持的鼠标按键：{other}")),
        }
    }
}

/// Platform input/capture primitives. Coordinates are in screen space.
pub trait Backend {
    fn screen_size(&self) -> anyhow::Result<ScreenSize>;
    fn capture(&self) -> anyhow::Result<Screen>;
    fn cursor_position(&self) -> anyhow::Result<(i32, i32)>;
    fn move_mouse(&mut self, x: i32, y: i32) -> anyhow::Result<()>;
    fn click(&mut self, x: i32, y: i32, button: MouseButton, count: u32) -> anyhow::Result<()>;
    fn drag(&mut self, from: (i32, i32), to: (i32, i32)) -> anyhow::Result<()>;
    /// Hold the left button and move through every point in order, then release
    /// once. Implementations must release the button even if a step fails.
    /// The default degrades to one `drag` per segment (button released between
    /// segments), so real backends override it.
    fn drag_path(&mut self, points: &[(i32, i32)]) -> anyhow::Result<()> {
        for pair in points.windows(2) {
            self.drag(pair[0], pair[1])?;
        }
        Ok(())
    }
    fn scroll(&mut self, x: i32, y: i32, dx: i32, dy: i32) -> anyhow::Result<()>;
    fn type_text(&mut self, text: &str) -> anyhow::Result<()>;
    /// `keys` are normalized names (see [`normalize_key`]); modifiers first.
    fn press_keys(&mut self, keys: &[String]) -> anyhow::Result<()>;
}

/// Fit `screen` into the max screenshot box, preserving aspect ratio and
/// never upscaling.
pub fn screenshot_size(screen: ScreenSize) -> ScreenSize {
    let width = screen.width.max(1);
    let height = screen.height.max(1);
    let scale = f64::min(
        1.0,
        f64::min(
            MAX_SCREENSHOT_WIDTH as f64 / width as f64,
            MAX_SCREENSHOT_HEIGHT as f64 / height as f64,
        ),
    );
    ScreenSize {
        width: ((width as f64 * scale).round() as u32).max(1),
        height: ((height as f64 * scale).round() as u32).max(1),
    }
}

/// Map a screenshot-space point to screen space. Rejects out-of-range input.
pub fn to_screen_point(screen: ScreenSize, x: f64, y: f64) -> Result<(i32, i32), String> {
    let shot = screenshot_size(screen);
    if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
        return Err(format!("坐标无效：({x}, {y})"));
    }
    if x >= shot.width as f64 || y >= shot.height as f64 {
        return Err(format!(
            "坐标 ({x}, {y}) 超出截图范围 {}x{}",
            shot.width, shot.height
        ));
    }
    let sx = (x * screen.width as f64 / shot.width as f64).floor() as i32;
    let sy = (y * screen.height as f64 / shot.height as f64).floor() as i32;
    Ok((
        sx.min(screen.width as i32 - 1),
        sy.min(screen.height as i32 - 1),
    ))
}

const NAMED_KEYS: &[&str] = &[
    "ctrl",
    "shift",
    "alt",
    "meta",
    "enter",
    "tab",
    "escape",
    "backspace",
    "delete",
    "space",
    "up",
    "down",
    "left",
    "right",
    "home",
    "end",
    "pageup",
    "pagedown",
    "insert",
    "f1",
    "f2",
    "f3",
    "f4",
    "f5",
    "f6",
    "f7",
    "f8",
    "f9",
    "f10",
    "f11",
    "f12",
];

pub fn is_modifier(key: &str) -> bool {
    matches!(key, "ctrl" | "shift" | "alt" | "meta")
}

/// Normalize a user key name; returns `None` for unsupported keys.
pub fn normalize_key(raw: &str) -> Option<String> {
    let key = raw.trim().to_ascii_lowercase();
    let key = match key.as_str() {
        "control" => "ctrl",
        "option" => "alt",
        "cmd" | "command" | "win" | "windows" | "super" => "meta",
        "return" => "enter",
        "esc" => "escape",
        "del" => "delete",
        "arrowup" => "up",
        "arrowdown" => "down",
        "arrowleft" => "left",
        "arrowright" => "right",
        "pgup" => "pageup",
        "pgdn" => "pagedown",
        other => other,
    };
    let single = key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric());
    (single || NAMED_KEYS.contains(&key)).then(|| key.to_string())
}

fn point_schema(extra: Value) -> Value {
    let mut properties = json!({
        "x": { "type": "number", "description": "截图坐标系中的 X" },
        "y": { "type": "number", "description": "截图坐标系中的 Y" }
    });
    if let (Some(base), Some(more)) = (properties.as_object_mut(), extra.as_object()) {
        base.extend(more.clone());
    }
    json!({ "type": "object", "properties": properties, "required": ["x", "y"] })
}

pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "screenshot",
            "description": "截取主显示器。之后所有坐标都使用这张截图的像素坐标。每次操作后如需确认结果请重新截图。",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "click",
            "description": "在指定坐标点击鼠标。",
            "inputSchema": point_schema(json!({
                "button": { "type": "string", "enum": ["left", "right", "middle"] },
                "count": { "type": "integer", "minimum": 1, "maximum": 3, "description": "点击次数，2 为双击" }
            }))
        },
        {
            "name": "move_mouse",
            "description": "把鼠标移动到指定坐标（用于悬停）。",
            "inputSchema": point_schema(json!({}))
        },
        {
            "name": "drag",
            "description": "按住左键从起点拖到终点。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from_x": { "type": "number" }, "from_y": { "type": "number" },
                    "to_x": { "type": "number" }, "to_y": { "type": "number" }
                },
                "required": ["from_x", "from_y", "to_x", "to_y"]
            }
        },
        {
            "name": "drag_path",
            "description": "按住左键依次经过多个点后松开，用于一笔画出曲线、圆、签名等。points 为 2~200 个截图坐标点，点越密线条越平滑。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "points": {
                        "type": "array",
                        "minItems": 2,
                        "maxItems": 200,
                        "items": {
                            "type": "object",
                            "properties": { "x": { "type": "number" }, "y": { "type": "number" } },
                            "required": ["x", "y"]
                        }
                    }
                },
                "required": ["points"]
            }
        },
        {
            "name": "scroll",
            "description": "在指定坐标滚动。dy 正数向下，dx 正数向右，单位为滚轮格数。",
            "inputSchema": point_schema(json!({
                "dx": { "type": "integer" },
                "dy": { "type": "integer" }
            }))
        },
        {
            "name": "type_text",
            "description": "在当前焦点处输入文本（支持中文等 Unicode）。",
            "inputSchema": {
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            }
        },
        {
            "name": "press_keys",
            "description": "按下组合键，例如 [\"ctrl\",\"c\"]、[\"enter\"]、[\"meta\",\"v\"]。",
            "inputSchema": {
                "type": "object",
                "properties": { "keys": { "type": "array", "items": { "type": "string" }, "minItems": 1 } },
                "required": ["keys"]
            }
        },
        {
            "name": "cursor_position",
            "description": "返回鼠标当前位置（截图坐标系）。",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "wait",
            "description": "等待指定秒数（0~10），用于等待页面加载。",
            "inputSchema": {
                "type": "object",
                "properties": { "seconds": { "type": "number", "minimum": 0, "maximum": 10 } },
                "required": ["seconds"]
            }
        }
    ])
}
