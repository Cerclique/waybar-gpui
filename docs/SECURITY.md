# Security Policy

## Supported versions

Only the latest release receives security fixes.

## Scope

The SDK and its runtime, the starter template, and the programs and bar module
in this repository. The part most worth a look: every program's daemon listens
on a per-user socket in `$XDG_RUNTIME_DIR/waybar-gpui/` (the system temp
directory if `XDG_RUNTIME_DIR` is unset) and accepts `toggle`, `close` and
`quit` from any local process that can connect to it. Anything that lets
another user reach that socket, or makes a daemon do more than those commands
with the panel arguments it is sent, is in scope. How the socket and its
protocol work: [architecture](architecture.md#socket-directory-and-line-protocol).

## Reporting a vulnerability

Please do **not** report security vulnerabilities through public issues, pull
requests or discussions.

Report them privately through GitHub instead:
[**Report a vulnerability**](https://github.com/Cerclique/waybar_gpui_plugin_template/security/advisories/new)
(Security tab → Advisories → Report a vulnerability).

Please include:

- the affected version or commit,
- a description of the issue and its impact,
- steps to reproduce it.

You should get a first response within a week. Once a fix is released, the
advisory is published, crediting you unless you prefer otherwise.
