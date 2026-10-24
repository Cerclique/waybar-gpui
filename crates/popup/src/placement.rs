//! Where to put the popover so it looks attached to the clicked Waybar icon.
//!
//! Layer surfaces can't be placed at absolute coordinates, only anchored to
//! output edges with margins. We anchor to the bar's edge and the left side,
//! and compute margins from the cursor position at click time.

use hypr::{CursorPos, Monitor};

#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Hyprland output name, matched to a GPUI display.
    pub monitor: String,
    /// Anchor to the bottom edge (bar at the bottom) instead of the top.
    pub bottom: bool,
    /// CSS order: top, right, bottom, left.
    pub margin: (f32, f32, f32, f32),
}

/// `size` is the popover size and `gap` the space kept between it and the bar
/// or the screen edge, both in logical pixels.
pub fn place(cursor: CursorPos, monitors: &[Monitor], size: (f32, f32), gap: f32) -> Option<Placement> {
    let monitor = monitors
        .iter()
        .find(|m| m.contains(cursor.x, cursor.y))
        .or_else(|| monitors.iter().find(|m| m.focused))?;
    let (mon_w, mon_h) = monitor.logical_size();
    let [res_left, res_top, res_right, res_bottom] = monitor.reserved.map(|r| r as f32);
    let (local_x, local_y) = (cursor.x - monitor.x as f32, cursor.y - monitor.y as f32);

    // Centre under the cursor, kept inside the usable area.
    let min_left = res_left + gap;
    let max_left = (mon_w - res_right - size.0 - gap).max(min_left);
    let left = (local_x - size.0 / 2.0).clamp(min_left, max_left);

    // The click happened on the bar, so the bar is on the nearer edge.
    let bottom = local_y > mon_h / 2.0;
    let margin = if bottom {
        (0.0, 0.0, res_bottom + gap, left)
    } else {
        (res_top + gap, 0.0, 0.0, left)
    };

    Some(Placement { monitor: monitor.name.clone(), bottom, margin })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: i32, reserved: [i32; 4]) -> Monitor {
        Monitor {
            name: name.into(),
            x,
            y: 0,
            width: 1920,
            height: 1080,
            scale: 1.0,
            transform: 0,
            focused: false,
            reserved,
        }
    }

    #[test]
    fn centres_under_cursor_below_top_bar() {
        let monitors = [monitor("DP-2", 0, [0, 36, 0, 0]), monitor("DP-1", 1920, [0, 36, 0, 0])];
        let p = place(CursorPos { x: 2727.0, y: 18.0 }, &monitors, (320.0, 200.0), 6.0).unwrap();
        assert_eq!(p.monitor, "DP-1");
        assert!(!p.bottom);
        assert_eq!(p.margin, (42.0, 0.0, 0.0, 647.0));
    }

    #[test]
    fn clamps_to_screen_edge_and_handles_bottom_bar() {
        let monitors = [monitor("DP-1", 0, [0, 0, 0, 30])];
        let p = place(CursorPos { x: 1910.0, y: 1070.0 }, &monitors, (320.0, 200.0), 6.0).unwrap();
        assert!(p.bottom);
        assert_eq!(p.margin, (0.0, 0.0, 36.0, 1594.0));
    }
}
