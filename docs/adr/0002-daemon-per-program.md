# One daemon per program

Each author's program runs its own runtime: a click client (`toggle`) that talks to the program's own long-lived daemon over `$XDG_RUNTIME_DIR/waybar-gpui/<id>.sock`, spawning it when none answers. A process per click would pay GPUI's wgpu/Vulkan and font start-up on every open (~400 ms against ~140 ms warm); a single shared runtime for all programs would make the SDK a service to install and version instead of a library each program links.

## Consequences

- Programs coordinate without a central process: before opening, a runtime sends `close` to every other socket in the shared directory (best effort, stale sockets deleted), so at most one popover is open.
- A warm daemon would keep running an old binary after a rebuild or upgrade, so the daemon reports its executable's device and inode and `toggle` replaces a daemon that isn't its own binary.
