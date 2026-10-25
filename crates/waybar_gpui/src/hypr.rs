//! Minimal Hyprland IPC: the requests the runtime needs to place a popover
//! (`j/cursorpos`, `j/monitors`).
//!
//! Hyprland answers hyprctl-style requests on
//! `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`
//! (`j/<command>` returns JSON). It serves that socket synchronously, so every
//! request opens, writes, reads to EOF and closes. Never keep it open.
//!
//! See <https://wiki.hypr.land/IPC/>.

use std::{
    env,
    io::{self, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use serde::{Deserialize, de::DeserializeOwned};

/// Directory holding this Hyprland instance's sockets.
fn socket_dir() -> io::Result<PathBuf> {
    let his = env::var_os("HYPRLAND_INSTANCE_SIGNATURE").ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "HYPRLAND_INSTANCE_SIGNATURE is not set (not running under Hyprland?)")
    })?;
    let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    // Hyprland falls back to /tmp/hypr when $XDG_RUNTIME_DIR/hypr is missing.
    let base = runtime.map(|r| r.join("hypr")).filter(|p| p.is_dir()).unwrap_or_else(|| PathBuf::from("/tmp/hypr"));
    Ok(base.join(his))
}

/// Sends `j/<command>` and deserializes the JSON reply.
pub fn request<T: DeserializeOwned>(command: &str) -> io::Result<T> {
    let mut stream = UnixStream::connect(socket_dir()?.join(".socket.sock"))?;
    stream.write_all(format!("j/{command}").as_bytes())?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    serde_json::from_str(&reply).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// An entry of `j/monitors`. Position is in global logical pixels, size in
/// physical pixels (divide by `scale`).
#[derive(Debug, Clone, Deserialize)]
pub struct Monitor {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub transform: u8,
    pub focused: bool,
    /// Space reserved by layer surfaces with an exclusive zone (e.g. Waybar),
    /// as `[left, top, right, bottom]`.
    pub reserved: [i32; 4],
}

impl Monitor {
    /// Size in logical pixels, accounting for scale and 90°/270° rotation.
    pub fn logical_size(&self) -> (f32, f32) {
        let (w, h) = (self.width as f32 / self.scale, self.height as f32 / self.scale);
        if self.transform % 2 == 1 { (h, w) } else { (w, h) }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        let (w, h) = self.logical_size();
        let (mx, my) = (self.x as f32, self.y as f32);
        x >= mx && x < mx + w && y >= my && y < my + h
    }
}

/// `j/cursorpos`, in global logical pixels.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct CursorPos {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_geometry() {
        let m: Monitor = serde_json::from_str(
            r#"{"name":"DP-1","x":1920,"y":0,"width":3840,"height":2160,"scale":2,
                "transform":1,"focused":true,"reserved":[0,36,0,0]}"#,
        )
        .unwrap();
        assert_eq!(m.logical_size(), (1080.0, 1920.0));
        assert!(m.contains(1920.0, 0.0));
        assert!(!m.contains(1919.0, 0.0));
    }
}
