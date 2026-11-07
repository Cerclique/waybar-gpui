//! The compositor seam: what the runtime asks the compositor to place a
//! popover, in types that name no compositor. `hypr.rs` answers on Hyprland;
//! the runtime's tests answer with a fake.

use std::io;

/// Where the cursor is and which monitors exist, asked each time a popover
/// opens. Requests run on GPUI's foreground thread: an implementation must
/// not block for long.
pub trait Compositor {
    /// The cursor, in global logical pixels.
    fn cursor(&self) -> io::Result<CursorPos>;

    /// Every monitor, in no particular order.
    fn monitors(&self) -> io::Result<Vec<Monitor>>;
}

/// A monitor, in global logical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// The `wl_output` name (`DP-1`).
    pub name: String,
    /// Top-left corner.
    pub x: i32,
    pub y: i32,
    /// Width and height, rotation and scale applied.
    pub size: (f32, f32),
    /// Has keyboard focus: the fallback when the cursor is over no monitor.
    pub focused: bool,
    /// Space reserved by layer surfaces with an exclusive zone (e.g. Waybar),
    /// as `[left, top, right, bottom]`.
    pub reserved: [i32; 4],
}

impl Monitor {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let (w, h) = self.size;
        let (mx, my) = (self.x as f32, self.y as f32);
        x >= mx && x < mx + w && y >= my && y < my + h
    }
}

/// The cursor, in global logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorPos {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_its_top_left_corner_not_its_far_edges() {
        let m =
            Monitor { name: "DP-1".into(), x: 1920, y: 0, size: (1080.0, 1920.0), focused: false, reserved: [0; 4] };
        assert!(m.contains(1920.0, 0.0));
        assert!(m.contains(2999.0, 1919.0));
        assert!(!m.contains(1919.0, 0.0));
        assert!(!m.contains(3000.0, 0.0));
        assert!(!m.contains(1920.0, 1920.0));
    }
}
