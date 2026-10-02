use super::tools::{Backend, MouseButton, Screen, ScreenSize, to_screen_point};
use super::*;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct FakeBackend {
    cursor: (i32, i32),
    actions: Vec<String>,
}

impl Backend for FakeBackend {
    fn screen_size(&self) -> anyhow::Result<ScreenSize> {
        Ok(ScreenSize {
            width: 2560,
            height: 1600,
        })
    }
    fn capture(&self) -> anyhow::Result<Screen> {
        Ok(Screen {
            size: self.screen_size()?,
            image: image::RgbImage::from_pixel(2560, 1600, image::Rgb([20, 40, 60])),
        })
    }
    fn cursor_position(&self) -> anyhow::Result<(i32, i32)> {
        Ok(self.cursor)
    }
    fn move_mouse(&mut self, x: i32, y: i32) -> anyhow::Result<()> {
        self.actions.push(format!("move {x} {y}"));
        Ok(())
    }
    fn click(&mut self, x: i32, y: i32, button: MouseButton, count: u32) -> anyhow::Result<()> {
        self.actions
            .push(format!("click {x} {y} {button:?} {count}"));
        Ok(())
    }
    fn drag(&mut self, from: (i32, i32), to: (i32, i32)) -> anyhow::Result<()> {
        self.actions.push(format!("drag {from:?} {to:?}"));
        Ok(())
    }
    fn scroll(&mut self, x: i32, y: i32, dx: i32, dy: i32) -> anyhow::Result<()> {
        self.actions.push(format!("scroll {x} {y} {dx} {dy}"));
        Ok(())
    }
    fn type_text(&mut self, text: &str) -> anyhow::Result<()> {
        self.actions.push(format!("type {text}"));
        Ok(())
    }
    fn press_keys(&mut self, keys: &[String]) -> anyhow::Result<()> {
        self.actions.push(format!("keys {}", keys.join("+")));
        Ok(())
    }
}

#[derive(Clone)]
struct FakeGate(Rc<RefCell<(bool, u32)>>);

impl Gate for FakeGate {
    fn enabled(&self) -> bool {
        self.0.borrow().0
    }
    fn emergency_stop(&mut self) {
        let mut state = self.0.borrow_mut();
        state.0 = false;
        state.1 += 1;
    }
}

struct Harness {
    server: ComputerUseServer<FakeBackend, FakeGate>,
    gate: FakeGate,
    audit: Rc<RefCell<Vec<Value>>>,
}

fn harness(enabled: bool, cursor: (i32, i32)) -> Harness {
    let gate = FakeGate(Rc::new(RefCell::new((enabled, 0))));
    let audit = Rc::new(RefCell::new(Vec::new()));
    let sink = audit.clone();
    let backend = FakeBackend {
        cursor,
        ..Default::default()
    };
    let server = ComputerUseServer::with_audit(
        backend,
        gate.clone(),
        Box::new(move |detail| sink.borrow_mut().push(detail.clone())),
    );
    Harness {
        server,
        gate,
        audit,
    }
}

fn rpc(h: &mut Harness, method: &str, params: Value) -> Value {
    let line = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
    serde_json::from_str(&h.server.handle_line(&line.to_string()).expect("response")).unwrap()
}

fn call(h: &mut Harness, name: &str, arguments: Value) -> Value {
    rpc(
        h,
        "tools/call",
        json!({ "name": name, "arguments": arguments }),
    )["result"]
        .clone()
}

#[test]
fn initialize_echoes_protocol_and_advertises_tools() {
    let mut h = harness(true, (500, 500));
    let result = &rpc(
        &mut h,
        "initialize",
        json!({ "protocolVersion": "2024-11-05" }),
    )["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], COMPUTER_USE_SERVER_NAME);
    assert!(result["capabilities"]["tools"].is_object());
}

#[test]
fn tools_list_has_all_tools_with_schemas() {
    let mut h = harness(true, (500, 500));
    let tools = rpc(&mut h, "tools/list", json!({}))["result"]["tools"].clone();
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| {
            assert!(tool["inputSchema"].is_object());
            tool["name"].as_str().unwrap()
        })
        .collect();
    for expected in [
        "screenshot",
        "click",
        "move_mouse",
        "drag",
        "scroll",
        "type_text",
        "press_keys",
        "cursor_position",
        "wait",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
}

#[test]
fn protocol_errors_and_notifications() {
    let mut h = harness(true, (500, 500));
    assert_eq!(rpc(&mut h, "nope", json!({}))["error"]["code"], -32601);
    let parse: Value = serde_json::from_str(&h.server.handle_line("{bad").unwrap()).unwrap();
    assert_eq!(parse["error"]["code"], -32700);
    assert!(
        h.server
            .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );
}

#[test]
fn disabled_gate_blocks_every_tool() {
    let mut h = harness(false, (500, 500));
    let result = call(&mut h, "click", json!({ "x": 10, "y": 10 }));
    assert_eq!(result["isError"], true);
    assert!(h.server.backend().actions.is_empty());
}

#[test]
fn failsafe_corner_stops_and_disables() {
    let mut h = harness(true, (1, 2));
    let result = call(&mut h, "click", json!({ "x": 10, "y": 10 }));
    assert_eq!(result["isError"], true);
    assert_eq!(h.gate.0.borrow().1, 1);
    assert!(!h.gate.enabled());
    assert!(h.server.backend().actions.is_empty());
}

#[test]
#[ignore = "captures the real screen; run explicitly with --ignored"]
fn native_capture_matches_screen_size() {
    super::platform::prepare_process();
    let backend = super::platform::NativeBackend::default();
    let size = backend.screen_size().unwrap();
    let screen = backend.capture().unwrap();
    assert_eq!(screen.size, size);
    assert!(screen.image.width() >= size.width && screen.image.height() >= size.height);
    assert!(screen.image.pixels().any(|pixel| pixel.0 != [0, 0, 0]));
    backend.cursor_position().unwrap();
}

#[test]
fn agent_cannot_target_failsafe_corner() {
    let mut h = harness(true, (500, 500));
    let result = call(&mut h, "move_mouse", json!({ "x": 0, "y": 0 }));
    assert_eq!(result["isError"], true);
    assert!(h.server.backend().actions.is_empty());
    assert_eq!(h.gate.0.borrow().1, 0);
}

#[test]
fn coordinates_scale_from_screenshot_space() {
    let screen = ScreenSize {
        width: 2560,
        height: 1600,
    };
    assert_eq!(
        screenshot_size(screen),
        ScreenSize {
            width: 1280,
            height: 800
        }
    );
    assert_eq!(to_screen_point(screen, 640.0, 400.0), Ok((1280, 800)));
    assert!(to_screen_point(screen, 1280.0, 10.0).is_err());
    assert!(to_screen_point(screen, -1.0, 10.0).is_err());
    let small = ScreenSize {
        width: 1024,
        height: 768,
    };
    assert_eq!(screenshot_size(small), small);

    let mut h = harness(true, (500, 500));
    call(&mut h, "click", json!({ "x": 100, "y": 50, "count": 2 }));
    assert_eq!(h.server.backend().actions, vec!["click 200 100 Left 2"]);
    let out = call(&mut h, "click", json!({ "x": 5000, "y": 50 }));
    assert_eq!(out["isError"], true);
}

#[test]
fn screenshot_returns_bounded_jpeg() {
    let mut h = harness(true, (500, 500));
    let result = call(&mut h, "screenshot", json!({}));
    assert_eq!(result["isError"], false);
    let image = &result["content"][0];
    assert_eq!(image["type"], "image");
    assert_eq!(image["mimeType"], "image/jpeg");
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image["data"].as_str().unwrap())
        .unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap();
    assert!(decoded.width() <= 1280 && decoded.height() <= 800);
}

#[test]
fn keys_are_normalized_and_text_not_audited() {
    let mut h = harness(true, (500, 500));
    call(&mut h, "press_keys", json!({ "keys": ["Control", "C"] }));
    assert_eq!(
        call(&mut h, "press_keys", json!({ "keys": ["bogus"] }))["isError"],
        true
    );
    call(&mut h, "type_text", json!({ "text": "秘密密码123" }));
    assert_eq!(
        h.server.backend().actions,
        vec!["keys ctrl+c", "type 秘密密码123"]
    );
    let audit = h.audit.borrow();
    let last = audit.last().unwrap();
    assert_eq!(last["textChars"], 7);
    assert!(!last.to_string().contains("秘密"));
}

#[test]
fn register_and_unregister_preserve_other_entries() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("claude_desktop_config.json");
    std::fs::write(
        &config,
        r#"{"theme":"dark","mcpServers":{"other":{"command":"x"}}}"#,
    )
    .unwrap();
    let paths = vec![config.clone()];
    let exe = dir.path().join("claude-codex-pro.exe");

    register_computer_use_at(&paths, &exe, false).unwrap();
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(value["theme"], "dark");
    assert_eq!(value["mcpServers"]["other"]["command"], "x");
    assert_eq!(
        value["mcpServers"][COMPUTER_USE_SERVER_NAME]["args"],
        json!([COMPUTER_USE_MCP_ARG])
    );
    let status = computer_use_status_for_paths(true, &paths, &exe);
    assert_eq!(status.registered_paths.len(), 1);

    unregister_computer_use_at(&paths, false).unwrap();
    let value: Value = serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    assert!(value["mcpServers"].get(COMPUTER_USE_SERVER_NAME).is_none());
    assert_eq!(value["mcpServers"]["other"]["command"], "x");
}

#[test]
fn register_refuses_unparseable_config() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("claude_desktop_config.json");
    std::fs::write(&config, "{ not json").unwrap();
    let exe = dir.path().join("ccp.exe");
    assert!(register_computer_use_at(&[config.clone()], &exe, false).is_err());
    assert_eq!(std::fs::read_to_string(&config).unwrap(), "{ not json");
}
