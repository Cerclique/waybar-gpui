//! Minimal Hyprland IPC: the requests the runtime needs to place a popover
//! (`j/cursorpos`, `j/monitors`), behind the [`Compositor`] seam.
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
    time::Duration,
};

use serde::{Deserialize, de::DeserializeOwned};

use crate::compositor::{Compositor, CursorPos, Monitor};

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

/// How long to wait for Hyprland's reply. Requests run on GPUI's foreground
/// thread, so a stalled Hyprland must not freeze the daemon.
const REPLY_TIMEOUT: Duration = Duration::from_secs(1);

/// Sends `j/<command>` and deserializes the JSON reply.
fn request<T: DeserializeOwned>(command: &str) -> io::Result<T> {
    let mut stream = UnixStream::connect(socket_dir()?.join(".socket.sock"))?;
    stream.set_read_timeout(Some(REPLY_TIMEOUT))?;
    stream.write_all(format!("j/{command}").as_bytes())?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    serde_json::from_str(&reply).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// The compositor on Hyprland: one request per question.
pub struct Hyprland;

impl Compositor for Hyprland {
    fn cursor(&self) -> io::Result<CursorPos> {
        let CursorReply { x, y } = request("cursorpos")?;
        Ok(CursorPos { x, y })
    }

    fn monitors(&self) -> io::Result<Vec<Monitor>> {
        let monitors: Vec<MonitorReply> = request("monitors")?;
        Ok(monitors.into_iter().map(Monitor::from).collect())
    }
}

/// An entry of `j/monitors`. Position is in global logical pixels, size in
/// physical pixels (divide by `scale`).
#[derive(Debug, Clone, Deserialize)]
struct MonitorReply {
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale: f32,
    transform: u8,
    focused: bool,
    reserved: [i32; 4],
}

impl From<MonitorReply> for Monitor {
    fn from(m: MonitorReply) -> Self {
        // Logical size: divide by scale, swap for 90°/270° rotations.
        let (w, h) = (m.width as f32 / m.scale, m.height as f32 / m.scale);
        let size = if m.transform % 2 == 1 { (h, w) } else { (w, h) };
        Self { name: m.name, x: m.x, y: m.y, size, focused: m.focused, reserved: m.reserved }
    }
}

/// `j/cursorpos`, in global logical pixels.
#[derive(Debug, Clone, Copy, Deserialize)]
struct CursorReply {
    x: f32,
    y: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_geometry() {
        let m: MonitorReply = serde_json::from_str(
            r#"{"name":"DP-1","x":1920,"y":0,"width":3840,"height":2160,"scale":2,
                "transform":1,"focused":true,"reserved":[0,36,0,0]}"#,
        )
        .unwrap();
        let m = Monitor::from(m);
        assert_eq!(m.size, (1080.0, 1920.0));
        assert!(m.contains(1920.0, 0.0));
        assert!(!m.contains(1919.0, 0.0));
        assert_eq!((m.name.as_str(), m.focused, m.reserved), ("DP-1", true, [0, 36, 0, 0]));
    }
}
