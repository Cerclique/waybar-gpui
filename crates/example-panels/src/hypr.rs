//! The workspace panel's Hyprland client: a request and the event stream,
//! with only the fields the panel reads. The SDK's own Hyprland code is
//! private, so a program brings its own client, like this one, or a crate.
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

/// The name of an event line (`EVENT>>DATA`); the panel only filters on it.
fn event_name(line: &str) -> Option<&str> {
    line.split_once(">>").map(|(name, _)| name)
}

/// Blocking iterator over Hyprland event names. Ends when Hyprland closes the
/// socket.
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
    type Item = io::Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.lines.next()? {
                Ok(line) => {
                    if let Some(name) = event_name(&line) {
                        return Some(Ok(name.to_owned()));
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

/// `j/activeworkspace`
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub monitor: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceRef {
    pub id: i64,
}

/// An entry of `j/clients`.
#[derive(Debug, Clone, Deserialize)]
pub struct Client {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace: WorkspaceRef,
    /// 0 = currently focused, 1 = previously focused, ...
    #[serde(rename = "focusHistoryID")]
    pub focus_history_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_event_names() {
        assert_eq!(event_name("activewindow>>firefox,Some, title"), Some("activewindow"));
        assert_eq!(event_name("garbage"), None);
    }
}
