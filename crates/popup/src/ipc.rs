//! Line protocol between `waybar-popup toggle …` (client) and the daemon.
//!
//! One command per connection, one line: `toggle <panel>`, `close` or `quit`.

use std::{
    env, fs,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    thread,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Toggle(String),
    Close,
    Quit,
}

impl Command {
    pub fn parse(line: &str) -> Option<Self> {
        match line.trim().split_once(' ') {
            Some(("toggle", panel)) => Some(Self::Toggle(panel.trim().to_owned())),
            None if line.trim() == "close" => Some(Self::Close),
            None if line.trim() == "quit" => Some(Self::Quit),
            _ => None,
        }
    }

    fn to_line(&self) -> String {
        match self {
            Self::Toggle(panel) => format!("toggle {panel}\n"),
            Self::Close => "close\n".into(),
            Self::Quit => "quit\n".into(),
        }
    }
}

pub fn socket_path() -> PathBuf {
    let dir = env::var_os("XDG_RUNTIME_DIR").map_or_else(env::temp_dir, PathBuf::from);
    dir.join("waybar-popup.sock")
}

/// Sends a command to a running daemon. Fails with `NotFound` or
/// `ConnectionRefused` when no daemon is listening.
pub fn send(command: &Command) -> io::Result<()> {
    UnixStream::connect(socket_path())?.write_all(command.to_line().as_bytes())
}

/// Binds the socket and forwards parsed commands to `tx` from a background
/// thread. Fails if another daemon is already listening.
pub fn listen(tx: async_channel::Sender<Command>) -> io::Result<()> {
    let path = socket_path();
    if UnixStream::connect(&path).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AddrInUse, "waybar-popup daemon already running"));
    }
    // A socket file nobody answers on is left over from a crashed daemon.
    let _ = fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;

    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(stream).read_line(&mut line).is_err() {
                continue;
            }
            match Command::parse(&line) {
                Some(command) => {
                    if tx.send_blocking(command).is_err() {
                        break;
                    }
                }
                None => eprintln!("waybar-popup: ignoring unknown command {line:?}"),
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_commands() {
        for command in [Command::Toggle("workspace".into()), Command::Close, Command::Quit] {
            assert_eq!(Command::parse(&command.to_line()), Some(command));
        }
        assert_eq!(Command::parse("bogus"), None);
    }
}
