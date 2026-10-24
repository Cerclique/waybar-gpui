//! One fixed rule: centre on the cursor along the bar, `GAP` px off the
//! bar, clamped to the usable area. GPUI's `anchored()` measures the card and
//! applies this in the same frame, so the runtime only computes the anchor.

use hypr::{CursorPos, Monitor};

pub const GAP: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub monitor: String,
    pub monitor_size: (f32, f32),
    pub edge: Edge,
    /// Anchor point, output-local: the middle of the card's bar-side edge.
    pub anchor: (f32, f32),
    /// Space to keep free on each side: reserved area + gap. [left, top, right, bottom].
    pub margins: [f32; 4],
}

pub fn place(cursor: CursorPos, monitors: &[Monitor]) -> Option<Placement> {
    let monitor = monitors
        .iter()
        .find(|m| m.contains(cursor.x, cursor.y))
        .or_else(|| monitors.iter().find(|m| m.focused))?;
    let (w, h) = monitor.logical_size();
    let [left, top, right, bottom] = monitor.reserved.map(|r| r as f32);
    let (x, y) = (cursor.x - monitor.x as f32, cursor.y - monitor.y as f32);

    // The click happened on the bar: the reserved edge closest to the cursor.
    // With nothing reserved (bar not exclusive), fall back to the nearest edge.
    let candidates = [(Edge::Top, top, y), (Edge::Bottom, bottom, h - y), (Edge::Left, left, x), (Edge::Right, right, w - x)];
    let nearest = |reserved_only: bool| {
        candidates
            .iter()
            .filter(|(_, reserved, _)| !reserved_only || *reserved > 0.0)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(edge, _, _)| *edge)
    };
    let edge = nearest(true).or_else(|| nearest(false))?;

    let anchor = match edge {
        Edge::Top => (x, top + GAP),
        Edge::Bottom => (x, h - bottom - GAP),
        Edge::Left => (left + GAP, y),
        Edge::Right => (w - right - GAP, y),
    };
    Some(Placement {
        monitor: monitor.name.clone(),
        monitor_size: (w, h),
        edge,
        anchor,
        margins: [left + GAP, top + GAP, right + GAP, bottom + GAP],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: i32, reserved: [i32; 4]) -> Monitor {
        Monitor { name: name.into(), x, y: 0, width: 1920, height: 1080, scale: 1.0, transform: 0, focused: false, reserved }
    }

    #[test]
    fn top_bar_on_second_monitor() {
        let monitors = [monitor("DP-2", 0, [0, 36, 0, 0]), monitor("DP-1", 1920, [0, 36, 0, 0])];
        let p = place(CursorPos { x: 2727.0, y: 18.0 }, &monitors).unwrap();
        assert_eq!(p.monitor, "DP-1");
        assert_eq!(p.monitor_size, (1920.0, 1080.0));
        assert_eq!(p.edge, Edge::Top);
        assert_eq!(p.anchor, (807.0, 42.0));
        assert_eq!(p.margins, [6.0, 42.0, 6.0, 6.0]);
    }

    #[test]
    fn bottom_bar() {
        let p = place(CursorPos { x: 1910.0, y: 1070.0 }, &[monitor("DP-1", 0, [0, 0, 0, 30])]).unwrap();
        assert_eq!(p.edge, Edge::Bottom);
        assert_eq!(p.anchor, (1910.0, 1044.0));
    }

    #[test]
    fn left_and_right_bars_centre_on_cursor_y() {
        let monitors = [monitor("DP-1", 0, [40, 36, 40, 0])];
        let left = place(CursorPos { x: 20.0, y: 500.0 }, &monitors).unwrap();
        assert_eq!(left.edge, Edge::Left);
        assert_eq!(left.anchor, (46.0, 500.0));
        let right = place(CursorPos { x: 1900.0, y: 700.0 }, &monitors).unwrap();
        assert_eq!(right.edge, Edge::Right);
        assert_eq!(right.anchor, (1874.0, 700.0));
    }

    #[test]
    fn nothing_reserved_falls_back_to_nearest_edge() {
        let p = place(CursorPos { x: 300.0, y: 1075.0 }, &[monitor("DP-1", 0, [0, 0, 0, 0])]).unwrap();
        assert_eq!(p.edge, Edge::Bottom);
        assert_eq!(p.anchor, (300.0, 1074.0));
    }
}
