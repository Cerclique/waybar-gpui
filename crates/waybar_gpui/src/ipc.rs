//! One line per connection: `toggle<TAB>panel<TAB>arg…`, `close` or `quit`.
//! Every program's socket lives in one shared directory so runtimes can close
//! each other's popover.

use std::{
    env, fs,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    thread,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Panel name followed by its arguments.
    Toggle(Vec<String>),
    Close,
    Quit,
}

impl Command {
    fn parse(line: &str) -> Option<Self> {
        let mut words = line.trim_end_matches('\n').split('\t');
        match words.next()? {
            "toggle" => {
                let target: Vec<String> = words.map(str::to_owned).collect();
                (!target.is_empty()).then_some(Self::Toggle(target))
            }
            "close" => Some(Self::Close),
            "quit" => Some(Self::Quit),
            _ => None,
        }
    }

    fn to_line(&self) -> String {
        match self {
            Self::Toggle(target) => format!("toggle\t{}\n", target.join("\t")),
            Self::Close => "close\n".into(),
            Self::Quit => "quit\n".into(),
        }
    }
}

fn dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR").map_or_else(env::temp_dir, PathBuf::from).join("waybar-gpui")
}

pub fn socket_path(id: &str) -> PathBuf {
    dir().join(format!("{id}.sock"))
}

pub fn send(id: &str, command: &Command) -> io::Result<()> {
    UnixStream::connect(socket_path(id))?.write_all(command.to_line().as_bytes())
}

/// Best effort: asks every other program's runtime to close its popover, and
/// deletes sockets nobody answers on (left by crashed programs). `include_own`
/// also asks this program's daemon (used by `preview`, which has no socket).
pub fn close_others(id: &str, include_own: bool) {
    let own = socket_path(id);
    let Ok(entries) = fs::read_dir(dir()) else { return };
    for path in entries.flatten().map(|e| e.path()) {
        if (path == own && !include_own) || path.extension().is_none_or(|ext| ext != "sock") {
            continue;
        }
        match UnixStream::connect(&path) {
            Ok(mut stream) => {
                let _ = stream.write_all(Command::Close.to_line().as_bytes());
            }
            Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => {
                eprintln!("{id}: removing stale socket {}", path.display());
                let _ = fs::remove_file(&path);
            }
            Err(_) => {}
        }
    }
}

pub fn listen(id: &str, tx: async_channel::Sender<Command>) -> io::Result<()> {
    let path = socket_path(id);
    fs::create_dir_all(dir())?;
    if UnixStream::connect(&path).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AddrInUse, "daemon already running"));
    }
    let _ = fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;

    let id = id.to_owned();
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
                None => eprintln!("{id}: ignoring unknown command {line:?}"),
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
        for command in [
            Command::Toggle(vec!["workspace".into()]),
            Command::Toggle(vec!["clock".into(), "Paris".into(), "2".into()]),
            Command::Close,
            Command::Quit,
        ] {
            assert_eq!(Command::parse(&command.to_line()), Some(command));
        }
    }

    #[test]
    fn rejects_unknown_and_empty_toggle() {
        assert_eq!(Command::parse("bogus\n"), None);
        assert_eq!(Command::parse("toggle\n"), None);
    }
}
