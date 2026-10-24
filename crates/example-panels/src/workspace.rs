//! The active workspace and its windows, updated live while open. Shows:
//! - live updates from a blocking source (Hyprland events) with plain GPUI
//! - closing from inside the panel after an action
//! - GPUI derive macros through the SDK's re-export

use waybar_gpui::{
    Panel, Theme, card, close,
    gpui::{self, Context, FontWeight, IntoElement, RenderOnce, Task, Window, div, prelude::*},
};

/// Events after which the panel re-reads Hyprland.
const RELEVANT: &[&str] = &["workspacev2", "openwindow", "closewindow", "movewindowv2", "activewindowv2", "windowtitlev2"];

pub struct WorkspacePanel {
    workspace: Result<hypr::Workspace, String>,
    clients: Vec<hypr::Client>,
    updates: u32,
    /// Dropped with the panel on close, which stops the updates.
    _events: Task<()>,
}

fn read() -> (Result<hypr::Workspace, String>, Vec<hypr::Client>) {
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
    (workspace, clients)
}

impl Panel for WorkspacePanel {
    const NAME: &str = "workspace";

    fn new(_args: &[String], _: &mut Window, cx: &mut Context<Self>) -> Self {
        let (workspace, clients) = read();
        // The documented live-data pattern: block on a background thread,
        // update the panel on the foreground one.
        let _events = cx.spawn(async move |this, cx| {
            let Ok(mut events) = hypr::Events::connect() else { return };
            loop {
                let (back, event) = cx
                    .background_executor()
                    .spawn(async move {
                        let event = events.next();
                        (events, event)
                    })
                    .await;
                events = back;
                let Some(Ok(event)) = event else { return };
                if !RELEVANT.contains(&event.name.as_str()) {
                    continue;
                }
                let alive = this.update(cx, |panel, cx| {
                    (panel.workspace, panel.clients) = read();
                    panel.updates += 1;
                    cx.notify();
                });
                if alive.is_err() {
                    return;
                }
            }
        });
        Self { workspace, clients, updates: 0, _events }
    }
}

/// A derive-macro user: `#[derive(IntoElement)]` expands to `gpui::…` paths,
/// resolved here through `use waybar_gpui::gpui`.
#[derive(IntoElement)]
struct Row {
    client: hypr::Client,
    theme: Theme,
}

impl RenderOnce for Row {
    fn render(self, _: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        let theme = self.theme;
        let address = self.client.address.clone();
        div()
            .id(gpui::ElementId::Name(address.clone().into()))
            .flex()
            .flex_col()
            .px_2()
            .py_1()
            .rounded_md()
            .when(self.client.focus_history_id == 0, |row| row.bg(theme.surface))
            .hover(|style| style.bg(theme.overlay))
            .on_click(move |_, _, cx| {
                // Actions are the author's own business: run yours here, then close.
                eprintln!("example-panels: clicked window {address}");
                close(cx);
            })
            .child(div().font_weight(FontWeight::SEMIBOLD).child(self.client.class.clone()))
            .child(div().text_xs().text_color(theme.subtext).truncate().child(self.client.title.clone()))
    }
}

impl Render for WorkspacePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::get(cx);
        // The card fits its content: no size declared. Width capped, list scrolls.
        let frame = card(cx).w(gpui::px(340.)).max_h(gpui::px(420.));
        let ws = match &self.workspace {
            Ok(ws) => ws,
            Err(e) => return frame.child(format!("Hyprland unavailable: {e}")),
        };

        let header = div()
            .flex()
            .justify_between()
            .pb_1()
            .border_b_1()
            .border_color(theme.overlay)
            .child(div().font_weight(FontWeight::BOLD).child(format!("Workspace {}", ws.name)))
            .child(div().text_color(theme.subtext).child(format!("{} · {} updates", ws.monitor, self.updates)));

        let rows = self.clients.iter().map(|client| Row { client: client.clone(), theme });

        frame
            .child(header)
            .child(div().id("rows").flex().flex_col().gap_1().overflow_y_scroll().children(rows))
            .when(self.clients.is_empty(), |frame| frame.child(div().text_color(theme.subtext).child("No windows")))
    }
}
