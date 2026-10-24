# Waybar GPUI popovers

Clicking an item in Waybar opens a small GPUI UI next to it. This repo
provides what makes that work and lets authors write the UI themselves.

## Language

### What the person at the bar sees

**Bar module**:
An item in Waybar (a `custom/*` entry): its icon or text, plus the `on-click` that toggles a panel.
_Avoid_: module (alone), plugin, widget

**Popover**:
A panel while it is on screen, placed next to the clicked bar module. At most one is open at a time, across all programs.
_Avoid_: popup, window

**Card**:
The visible rectangle of a popover, sized to the panel's content, where the panel is drawn.
_Avoid_: frame, window

**Backdrop**:
The invisible rest of the popover's surface around the card, covering the whole monitor; a click that lands on it dismisses the popover.
_Avoid_: overlay, background

**Dismiss**:
Closing a popover from outside the panel: Esc the panel doesn't handle, a click on the backdrop, or a click on another monitor.
_Avoid_: hide, cancel

**Toggle**:
What a bar module's click asks for: open the panel, or close it if the same panel with the same panel arguments is already open.
_Avoid_: show, open (when closing is also meant)

### What an author builds

**Author**:
Someone writing their own panels with the SDK, in their own repository.
_Avoid_: user (ambiguous with the person clicking the bar)

**Panel**:
The GPUI UI an author writes, built fresh each time its popover opens.
_Avoid_: module, plugin, popup, view

**Panel arguments**:
The words after the panel's name in a bar module's `on-click`, handed to the panel when it opens.
_Avoid_: options, parameters, config

**Preview**:
Opening a panel from a terminal, without a bar module, until it is dismissed.
_Avoid_: demo, test mode

**Program**:
An author's executable that registers one or more panels; each program runs its own runtime.
_Avoid_: binary, app, plugin

**Program ID**:
The stable name an author gives their program, by which programs tell each other apart.
_Avoid_: app id, socket name

**Theme**:
The colours a program's cards, panels and widgets share.
_Avoid_: style, palette

**SDK**:
The Rust crate authors build panels and programs against.
_Avoid_: API, framework, library (as a name for it)

### What makes it work

**Runtime**:
The part of every program that turns a toggle into a popover and back: its daemon, placement and dismissal.
_Avoid_: host, server

**Daemon**:
A program's background process that shows its popovers and stays running between opens until told to quit.
_Avoid_: server, service, runtime (that's the whole, not this process)
