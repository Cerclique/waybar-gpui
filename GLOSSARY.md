# Waybar GPUI popovers

Clicking an item in Waybar opens a small GPUI UI next to it. This repo
provides what makes that work and lets authors write the UI themselves.

## Language

**Bar module**:
An item in Waybar (a `custom/*` entry): its icon or text, plus the `on-click` that opens a popover.
_Avoid_: module (alone), plugin, widget

**Panel**:
The GPUI UI an author writes, shown when its bar module is clicked.
_Avoid_: module, plugin, popup, view

**Popover**:
A panel while it is on screen, placed under the click and dismissed by Esc or a click outside.
_Avoid_: popup, window

**Runtime**:
What turns a click into a popover and back: the background process, its socket, placement and dismissal.
_Avoid_: daemon (that's one part of it), host, server

**SDK**:
The Rust crate authors build panels against.
_Avoid_: API, framework, library (as a name for it)

**Author**:
Someone writing their own panel with the SDK.
_Avoid_: user (ambiguous with the person clicking the bar)
