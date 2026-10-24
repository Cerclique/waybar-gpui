//! Where to put the popover so it looks attached to the clicked Waybar icon.
//!
//! The popover surface covers the whole output; this computes where inside it
//! the card goes, from the cursor position at click time and the space Waybar
//! reserves.

use hypr::{CursorPos, Monitor};

#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Hyprland output name, matched to a GPUI display.
    pub monitor: String,
    /// Output size in logical pixels: the size of the popover surface.
    pub monitor_size: (f32, f32),
    /// Card's top-left corner, relative to the output.
    pub origin: (f32, f32),
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
    let top = if local_y > mon_h / 2.0 {
        mon_h - res_bottom - gap - size.1
    } else {
        res_top + gap
    };

    Some(Placement { monitor: monitor.name.clone(), monitor_size: (mon_w, mon_h), origin: (left, top) })
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
        assert_eq!(p.monitor_size, (1920.0, 1080.0));
        assert_eq!(p.origin, (647.0, 42.0));
    }

    #[test]
    fn clamps_to_screen_edge_and_handles_bottom_bar() {
        let monitors = [monitor("DP-1", 0, [0, 0, 0, 30])];
        let p = place(CursorPos { x: 1910.0, y: 1070.0 }, &monitors, (320.0, 200.0), 6.0).unwrap();
        assert_eq!(p.origin, (1594.0, 844.0));
    }
}
