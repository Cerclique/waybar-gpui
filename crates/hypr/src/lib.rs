//! Minimal Hyprland IPC client.
//!
//! Hyprland exposes two Unix sockets under
//! `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/`:
//!
//! - `.socket.sock` answers hyprctl-style requests (`j/<command>` returns JSON).
//!   Hyprland serves it synchronously, so every request opens, writes, reads to
//!   EOF and closes. Never keep this socket open.
//! - `.socket2.sock` streams `EVENT>>DATA\n` lines to every connected client.
//!
//! See <https://wiki.hypr.land/IPC/>.

use std::{
    env,
    io::{self, BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use serde::{Deserialize, de::DeserializeOwned};

/// Directory holding this Hyprland instance's sockets.
pub fn socket_dir() -> io::Result<PathBuf> {
    let his = env::var_os("HYPRLAND_INSTANCE_SIGNATURE").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "HYPRLAND_INSTANCE_SIGNATURE is not set (not running under Hyprland?)",
        )
    })?;
    let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    // Hyprland falls back to /tmp/hypr when $XDG_RUNTIME_DIR/hypr is missing.
    let base = runtime
        .map(|r| r.join("hypr"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from("/tmp/hypr"));
    Ok(base.join(his))
}

/// Sends a raw request (e.g. `"j/activeworkspace"`) and returns the reply.
pub fn request_raw(command: &str) -> io::Result<String> {
    let mut stream = UnixStream::connect(socket_dir()?.join(".socket.sock"))?;
    stream.write_all(command.as_bytes())?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    Ok(reply)
}

/// Sends `j/<command>` and deserializes the JSON reply.
pub fn request<T: DeserializeOwned>(command: &str) -> io::Result<T> {
    let reply = request_raw(&format!("j/{command}"))?;
    serde_json::from_str(&reply).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// One line from the event socket, split at `>>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub name: String,
    pub data: String,
}

impl Event {
    pub fn parse(line: &str) -> Option<Self> {
        let (name, data) = line.trim_end_matches('\n').split_once(">>")?;
        Some(Self { name: name.into(), data: data.into() })
    }
}

/// Blocking iterator over Hyprland events. Ends when Hyprland closes the socket.
pub struct Events {
    lines: io::Lines<BufReader<UnixStream>>,
}

impl Events {
    pub fn connect() -> io::Result<Self> {
        let stream = UnixStream::connect(socket_dir()?.join(".socket2.sock"))?;
        Ok(Self { lines: BufReader::new(stream).lines() })
    }
}

impl Iterator for Events {
    type Item = io::Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.lines.next()? {
                Ok(line) => {
                    if let Some(event) = Event::parse(&line) {
                        return Some(Ok(event));
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

// Only the fields this template uses; add more as needed (`hyprctl -j <cmd>`
// shows the full objects).

/// `j/activeworkspace`
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub monitor: String,
    pub windows: u32,
    #[serde(rename = "hasfullscreen")]
    pub has_fullscreen: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceRef {
    pub id: i64,
    pub name: String,
}

/// An entry of `j/clients`.
#[derive(Debug, Clone, Deserialize)]
pub struct Client {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace: WorkspaceRef,
    pub floating: bool,
    /// 0 = currently focused, 1 = previously focused, ...
    #[serde(rename = "focusHistoryID")]
    pub focus_history_id: i64,
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
    fn parses_events() {
        assert_eq!(
            Event::parse("activewindow>>firefox,Some, title\n"),
            Some(Event { name: "activewindow".into(), data: "firefox,Some, title".into() })
        );
        assert_eq!(Event::parse("garbage"), None);
    }

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
