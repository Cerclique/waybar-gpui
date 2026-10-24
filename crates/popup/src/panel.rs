//! Popover contents. To add a panel: add a `PanelKind` variant, give it a name
//! and size, write a view (wrap it in `frame`, like `WorkspacePanel`), and add
//! it to the `match` in `daemon::open`. The daemon handles positioning, Esc
//! and click-outside.

use gpui::{Context, Div, FontWeight, Window, div, prelude::*, rgb, rgba};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelKind {
    Workspace,
}

impl PanelKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "workspace" => Some(Self::Workspace),
            _ => None,
        }
    }

    /// Card size in logical pixels.
    pub fn size(self) -> (f32, f32) {
        match self {
            Self::Workspace => (340., 260.),
        }
    }
}

// Catppuccin Mocha; swap for your Waybar theme.
const BASE: u32 = 0x1e1e2ef2;
const SURFACE: u32 = 0x313244;
const OVERLAY: u32 = 0x45475a;
const TEXT: u32 = 0xcdd6f4;
const SUBTEXT: u32 = 0xa6adc8;

/// Shared card chrome: rounded corners, border, theme colours.
fn frame() -> Div {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .overflow_hidden()
        .rounded_xl()
        .border_1()
        .border_color(rgb(OVERLAY))
        .bg(rgba(BASE))
        .text_color(rgb(TEXT))
        .text_sm()
}

/// Windows on the active workspace, most recently focused first.
pub struct WorkspacePanel {
    workspace: Result<hypr::Workspace, String>,
    clients: Vec<hypr::Client>,
}

impl WorkspacePanel {
    pub fn new(_: &mut Context<Self>) -> Self {
        let workspace = hypr::request::<hypr::Workspace>("activeworkspace").map_err(|e| e.to_string());
        let mut clients: Vec<hypr::Client> = match &workspace {
            Ok(ws) => hypr::request::<Vec<hypr::Client>>("clients")
                .unwrap_or_default()
                .into_iter()
                .filter(|c| c.workspace.id == ws.id)
                .collect(),
            Err(_) => Vec::new(),
        };
        clients.sort_by_key(|c| c.focus_history_id);
        Self { workspace, clients }
    }
}

impl Render for WorkspacePanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let frame = frame();
        let ws = match &self.workspace {
            Ok(ws) => ws,
            Err(e) => return frame.child(format!("Hyprland unavailable: {e}")),
        };

        let header = div()
            .flex()
            .justify_between()
            .pb_1()
            .border_b_1()
            .border_color(rgb(OVERLAY))
            .child(div().font_weight(FontWeight::BOLD).child(format!("Workspace {}", ws.name)))
            .child(div().text_color(rgb(SUBTEXT)).child(ws.monitor.clone()));

        let rows = self.clients.iter().map(|client| {
            div()
                .flex()
                .flex_col()
                .px_2()
                .py_1()
                .rounded_md()
                .when(client.focus_history_id == 0, |row| row.bg(rgb(SURFACE)))
                .hover(|style| style.bg(rgb(OVERLAY)))
                .child(div().font_weight(FontWeight::SEMIBOLD).child(client.class.clone()))
                .child(div().text_xs().text_color(rgb(SUBTEXT)).truncate().child(client.title.clone()))
        });

        frame
            .child(header)
            .children(rows)
            .when(self.clients.is_empty(), |frame| {
                frame.child(div().text_color(rgb(SUBTEXT)).child("No windows"))
            })
    }
}
