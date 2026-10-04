use serde::{Deserialize, Serialize};

// ── Geometry ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn left(&self) -> i32 {
        self.x
    }

    pub fn top(&self) -> i32 {
        self.y
    }

    pub fn right(&self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }
}

// ── Window ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub pid: u32,
    pub hwnd: usize,
    pub title: String,
    pub exe_name: String,
    pub rect: Rect,
    pub visible: bool,
    pub is_foreground: bool,
}

// ── Element type ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum ElementType {
    Window,
    Button,
    SplitButton,
    Edit,
    Text,
    CheckBox,
    RadioButton,
    ComboBox,
    ListBox,
    ListItem,
    TreeView,
    TreeItem,
    Menu,
    MenuBar,
    MenuItem,
    TabControl,
    TabItem,
    ToolBar,
    StatusBar,
    ScrollBar,
    Slider,
    Spinner,
    ProgressBar,
    Image,
    Link,
    Group,
    Pane,
    Dialog,
    Document,
    DataGrid,
    DataItem,
    Header,
    HeaderItem,
    Table,
    TitleBar,
    ToolTip,
    Separator,
    Calendar,
    Thumb,
    Custom,
    Unknown,
}

// ── State types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToggleState {
    Off,
    On,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExpandState {
    Collapsed,
    Expanded,
    PartiallyExpanded,
    LeafNode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeInfo {
    pub value: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub step: f64,
    pub read_only: bool,
}

// ── UI Element ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    /// Unique ID in format "pid.runtime_id"
    pub id: String,

    #[serde(rename = "type")]
    pub element_type: ElementType,

    /// Accessible name/label
    pub label: String,

    /// Current value (for Edit, ComboBox, etc.)
    pub value: Option<String>,

    /// Full text content (for Document elements)
    pub text_content: Option<String>,

    /// Bounding box in screen coordinates
    pub rect: Rect,

    // ── State ──────────────────────────────────────────────────────────────
    pub enabled: bool,
    pub focused: bool,
    pub is_keyboard_focusable: bool,

    pub toggle_state: Option<ToggleState>,
    pub is_selected: Option<bool>,
    pub expand_state: Option<ExpandState>,
    pub range: Option<RangeInfo>,

    // ── Metadata ───────────────────────────────────────────────────────────
    pub automation_id: Option<String>,
    pub class_name: Option<String>,
    pub help_text: Option<String>,
    pub keyboard_shortcut: Option<String>,

    /// Available actions on this element
    pub actions: Vec<String>,

    /// Child elements
    pub children: Vec<UiElement>,
}

impl UiElement {
    pub fn new(id: String, element_type: ElementType) -> Self {
        Self {
            id,
            element_type,
            label: String::new(),
            value: None,
            text_content: None,
            rect: Rect::default(),
            enabled: true,
            focused: false,
            is_keyboard_focusable: false,
            toggle_state: None,
            is_selected: None,
            expand_state: None,
            range: None,
            automation_id: None,
            class_name: None,
            help_text: None,
            keyboard_shortcut: None,
            actions: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn depth_limit_placeholder(id: String) -> Self {
        let mut e = Self::new(id, ElementType::Unknown);
        e.enabled = false;
        e
    }
}

// ── ElementType helpers ───────────────────────────────────────────────────────

impl ElementType {
    pub const ALL: &'static [ElementType] = &[
        ElementType::Window,
        ElementType::Button,
        ElementType::SplitButton,
        ElementType::Edit,
        ElementType::Text,
        ElementType::CheckBox,
        ElementType::RadioButton,
        ElementType::ComboBox,
        ElementType::ListBox,
        ElementType::ListItem,
        ElementType::TreeView,
        ElementType::TreeItem,
        ElementType::Menu,
        ElementType::MenuBar,
        ElementType::MenuItem,
        ElementType::TabControl,
        ElementType::TabItem,
        ElementType::ToolBar,
        ElementType::StatusBar,
        ElementType::ScrollBar,
        ElementType::Slider,
        ElementType::Spinner,
        ElementType::ProgressBar,
        ElementType::Image,
        ElementType::Link,
        ElementType::Group,
        ElementType::Pane,
        ElementType::Dialog,
        ElementType::Document,
        ElementType::DataGrid,
        ElementType::DataItem,
        ElementType::Header,
        ElementType::HeaderItem,
        ElementType::Table,
        ElementType::TitleBar,
        ElementType::ToolTip,
        ElementType::Separator,
        ElementType::Calendar,
        ElementType::Thumb,
        ElementType::Custom,
        ElementType::Unknown,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ElementType::Window => "Window",
            ElementType::Button => "Button",
            ElementType::SplitButton => "SplitButton",
            ElementType::Edit => "Edit",
            ElementType::Text => "Text",
            ElementType::CheckBox => "CheckBox",
            ElementType::RadioButton => "RadioButton",
            ElementType::ComboBox => "ComboBox",
            ElementType::ListBox => "ListBox",
            ElementType::ListItem => "ListItem",
            ElementType::TreeView => "TreeView",
            ElementType::TreeItem => "TreeItem",
            ElementType::Menu => "Menu",
            ElementType::MenuBar => "MenuBar",
            ElementType::MenuItem => "MenuItem",
            ElementType::TabControl => "TabControl",
            ElementType::TabItem => "TabItem",
            ElementType::ToolBar => "ToolBar",
            ElementType::StatusBar => "StatusBar",
            ElementType::ScrollBar => "ScrollBar",
            ElementType::Slider => "Slider",
            ElementType::Spinner => "Spinner",
            ElementType::ProgressBar => "ProgressBar",
            ElementType::Image => "Image",
            ElementType::Link => "Link",
            ElementType::Group => "Group",
            ElementType::Pane => "Pane",
            ElementType::Dialog => "Dialog",
            ElementType::Document => "Document",
            ElementType::DataGrid => "DataGrid",
            ElementType::DataItem => "DataItem",
            ElementType::Header => "Header",
            ElementType::HeaderItem => "HeaderItem",
            ElementType::Table => "Table",
            ElementType::TitleBar => "TitleBar",
            ElementType::ToolTip => "ToolTip",
            ElementType::Separator => "Separator",
            ElementType::Calendar => "Calendar",
            ElementType::Thumb => "Thumb",
            ElementType::Custom => "Custom",
            ElementType::Unknown => "Unknown",
        }
    }
}

impl std::str::FromStr for ElementType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let wanted = s.trim();
        if let Some(t) = Self::ALL
            .iter()
            .find(|t| t.name().eq_ignore_ascii_case(wanted))
        {
            return Ok(*t);
        }
        let alias = match wanted.to_ascii_lowercase().as_str() {
            "hyperlink" => Some(ElementType::Link),
            "list" => Some(ElementType::ListBox),
            "tree" => Some(ElementType::TreeView),
            "tab" => Some(ElementType::TabControl),
            "textbox" | "textfield" | "input" => Some(ElementType::Edit),
            "checkbutton" => Some(ElementType::CheckBox),
            "label" | "statictext" => Some(ElementType::Text),
            "spinbutton" => Some(ElementType::Spinner),
            _ => None,
        };
        alias.ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown element type '{wanted}'. Valid types: {}",
                Self::ALL
                    .iter()
                    .map(|t| t.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
    }
}

// ── Find parameters ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct FindParams {
    pub query: Option<String>,
    pub element_type: Option<ElementType>,
    pub interactive_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitUntil {
    Appears,
    Disappears,
}
