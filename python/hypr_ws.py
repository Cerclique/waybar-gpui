#!/usr/bin/env python3
"""Waybar custom module: active Hyprland workspace, updated on events.

Stdlib-only quick start with the same JSON contract as crates/module.
Run it as a continuous script (no "interval" in the Waybar config).
"""
import json, os, socket, sys

RELEVANT_EVENTS = {"workspacev2", "focusedmonv2", "openwindow", "closewindow",
                   "movewindowv2", "fullscreen"}


def sock_dir():
    return os.path.join(os.environ["XDG_RUNTIME_DIR"], "hypr",
                        os.environ["HYPRLAND_INSTANCE_SIGNATURE"])


def hyprctl_json(cmd):
    # .socket.sock: open, write "j/<cmd>", read to EOF, close (never keep it open)
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.connect(os.path.join(sock_dir(), ".socket.sock"))
        s.sendall(f"j/{cmd}".encode())
        chunks = []
        while data := s.recv(8192):
            chunks.append(data)
    return json.loads(b"".join(chunks))


def emit():
    ws = hyprctl_json("activeworkspace")
    classes = []
    if ws["hasfullscreen"]:
        classes.append("fullscreen")
    if ws["windows"] == 0:
        classes.append("empty")
    print(json.dumps({
        "text": ws["name"],
        "alt": "fullscreen" if ws["hasfullscreen"] else "normal",
        "tooltip": f"{ws['windows']} window(s) on {ws['monitor']}",
        "class": classes,
        "percentage": min(100, ws["windows"] * 20),
    }), flush=True)                     # one object per line, flushed


def main():
    emit()
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as ev:
        ev.connect(os.path.join(sock_dir(), ".socket2.sock"))
        for line in ev.makefile("r", encoding="utf-8", errors="replace"):
            name, _, _data = line.rstrip("\n").partition(">>")
            if name in RELEVANT_EVENTS:
                emit()


if __name__ == "__main__":
    try:
        main()
    except (BrokenPipeError, KeyboardInterrupt):
        sys.exit(0)
