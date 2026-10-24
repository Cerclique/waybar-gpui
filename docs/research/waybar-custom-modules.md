# Custom Waybar modules ("plugins") under Hyprland

Research date: 2026-10-24. Target system: Arch Linux, Waybar **v0.15.0** (`waybar 0.15.0-3`, `/usr/bin/waybar`), Hyprland **v0.56.2**.

Every claim below is backed by a primary source: Waybar source at tag `0.15.0`, Waybar man pages installed on this machine, the Waybar wiki, the Hyprland wiki, or the repo/registry page of the library being described. Where something was checked by running it on this machine, the text says so.

> **About versions.** Waybar `0.15.0` was tagged on 2026-02-06. `master` is now about 1,090 commits ahead and is heading toward `0.16.0` ([commit f72f84e0 "...man-page build fix for 0.16.0"](https://github.com/Alexays/Waybar/commit/f72f84e0)). The **Waybar wiki module pages are generated automatically from `man/*.scd` on `master`** ([wiki: Module: CFFI banner](https://github.com/Alexays/Waybar/wiki/Module:-CFFI); [CONTRIBUTING.md "Documentation"](https://github.com/Alexays/Waybar/blob/master/CONTRIBUTING.md)). So the wiki can describe features that v0.15.0 lacks. The man pages installed locally (`/usr/share/man/man5/waybar-*.5.gz`) match 0.15.0. The "Differences in v0.15.0" notes below list each mismatch found.

---

## TL;DR

- **Waybar has no plugin system as such.** It offers two ways to extend it at runtime:
  1. **`custom/<name>`**: Waybar runs a program and renders the text it prints, either plain lines or one JSON object per line (`text`, `alt`, `tooltip`, `class`, `percentage`). The program can be polled (`interval`), triggered by a signal (`signal` + `pkill -RTMIN+N waybar`), or **continuous**: no `interval`, and it prints a new line whenever something changes.
  2. **`cffi/<name>`**: Waybar `dlopen()`s a shared library that exports a small C ABI (`wbcffi_version`, `wbcffi_init`, `wbcffi_deinit`, plus optional `wbcffi_update`, `wbcffi_refresh` and `wbcffi_doaction`). The library gets a raw `GtkContainer*` and can build any GTK3 widget tree. CFFI arrived in **v0.10.0**. ABI **v2**, where every config value is passed as JSON, arrived in **v0.13.0**. v0.15.0 accepts ABI 1 and ABI 2.
- Two more options exist: the **`image`** module, which shows an image from a path or a script, and **upstreaming a compiled-in C++ module** into the Waybar tree.
- **For Hyprland, use events, not polling.** Connect to `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock`, read `EVENT>>DATA\n` lines, and print a fresh JSON line for each relevant event. Fetch current state from `.socket.sock` with a `j/<command>` request, opening and closing that socket for each request. A ~50-line Python script doing this ran successfully against Hyprland 0.56.2 and Waybar 0.15.0 on this machine (see §2.3).
- **Libraries checked:** `waybar-cffi` (Rust, 0.1.1, last release 2025-03-06, low activity, uses ABI v1, README shows the wrong config key); `hyprland` (Rust, stable 0.3.13 / 0.4.0-beta.3, repo active 2026-07); `hyprland-go` (Go, v0.4.1, 2025-03); `hyprpy` (Python, 0.2.1, 2026-01); `hyprland-py` (Python, **archived**).
- **Recommendation for `waybar_gpui_plugin_template`:** build it on **`custom/*` with a continuous, event-driven program that writes JSON lines**. Use **Rust** for the reference implementation: one binary, low memory, and a crash does not take the bar down. Talk to the Hyprland sockets directly, which is about 60 lines, instead of depending on the beta `hyprland` crate. Ship a **dependency-free Python variant** as a quick start. Leave **CFFI** as an optional, advanced example, because it runs inside Waybar's process, is GTK3-only, and needs a full Waybar restart to pick up a rebuilt library.

---

## 1. Extension mechanisms

### 1.1 `custom/<name>`: external program, text out

Configuration reference: `waybar-custom(5)` (local `/usr/share/man/man5/waybar-custom.5.gz`; source [man/waybar-custom.5.scd @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/man/waybar-custom.5.scd)). Implementation: [src/modules/custom.cpp @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp).

**Key options** (man page, "CONFIGURATION"):

| Option | Meaning |
|---|---|
| `exec` | Command to run. Waybar runs it via `/bin/sh -c`, so `$HOME` and `~` expand ([include/util/command.hpp L69-L124 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/include/util/command.hpp#L69-L124)). |
| `exec-if` | Guard command. `exec` runs only when it exits 0. |
| `return-type` | `"json"`, or anything else for the i3blocks style (`text\ntooltip\nclass`). |
| `interval` | Polling period in seconds. Floats are allowed, minimum 1 ms. `"once"` runs only at startup. |
| `signal` | Integer N. `pkill -RTMIN+N waybar` triggers a re-run. With `signal` and no `interval`, the command runs once at startup and then only on that signal. |
| `restart-interval` | Only for continuous scripts: how long to wait before re-launching after the script exits. Float allowed. Cannot be combined with `interval`. |
| `exec-on-event` | Default `true`: re-run `exec` after an `on-click`/`on-scroll-*` command. |
| `on-click`, `on-click-middle`, `on-click-right`, `on-scroll-up`, `on-scroll-down`, `on-update`, `smooth-scrolling-threshold` | Mouse and update hooks. |
| `format` (default `{text}`), `format-icons`, `tooltip`, `tooltip-format`, `escape`, `hide-empty-text`, `max-length`, `min-length`, `rotate`, `align`, `justify`, `expand`, `menu`, `menu-file`, `menu-actions` | Presentation. |

**Which update mode applies.** The constructor picks exactly one worker ([custom.cpp L21-L28 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L21-L28)):
- `signal` set, with neither `interval` nor `restart-interval`: **waiting worker**. It runs once, then sleeps until the signal arrives ([L118-L136](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L118-L136)).
- `interval` > 0: **delay worker**. It runs `exec` and sleeps for `interval` ([L39-L64](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L39-L64)).
- Otherwise, with `exec` set: **continuous worker**. Waybar keeps the process open and **each line it prints triggers an update**. On EOF with a non-zero exit code it logs `"... stopped unexpectedly, is it endless?"`. It re-spawns the process only if `restart-interval` is set ([L66-L116](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L66-L116)).
- A signal wakes the worker only when it equals `SIGRTMIN + signal` ([L138-L142](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L138-L142)). A click or scroll wakes it unless `exec-on-event` is `false` ([L144-L148](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L144-L148)). Waking a continuous worker does not restart the process.

**JSON schema.** `parseOutputJson` ([L257-L292 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L257-L292)) reads these fields:

```json
{"text": "…", "alt": "…", "tooltip": "…", "class": "x" | ["x","y"], "percentage": 0-100}
```

- **One JSON object per line.** Waybar reads the output with `getline()`, so a pretty-printed object breaks. The master man page says the same thing ("must be printed on a single line… `jq --unbuffered --compact-output`").
- `text`, `alt` and `tooltip` are Pango markup unless `"escape": true` is set.
- `percentage` is rounded to an int. It drives `{percentage}` and an **array** `format-icons`.
- `alt` drives `{alt}` and an **object** `format-icons` (`{"alt-value": "icon", "default": "…"}`). Arrays nested inside the object are indexed by percentage ([ALabel::getIcon, src/ALabel.cpp L132-L152 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/ALabel.cpp#L132-L152)).
- Format placeholders: `{text}`, `{alt}`, `{icon}`, `{percentage}` ([custom.cpp L175-L177](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L175-L177)). Waybar refuses to mix `{}` with named placeholders ([L209-L215](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L209-L215)).
- **Empty output or a non-zero exit hides the module** ([L164-L166](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L164-L166)).
- **Styling:** the widget is `#custom-<name>`. The `class` values from your output are added as CSS classes after the previous ones are removed, along with `flat`, `text-button` and `module` ([L195-L206](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L195-L206)). Selectors are `#custom-<name>` and `#custom-<name>.<class>` (man page, "STYLE").
- **Environment:** the child process gets `WAYBAR_OUTPUT_NAME`, the monitor name ([command.hpp L111-L115 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/include/util/command.hpp#L111-L115)). Each process runs in its own process group (`setpgid`) and gets `SIGTERM` via `killpg` when the module is destroyed ([custom.cpp L31-L37](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp#L31-L37)).
- **One process per bar.** Waybar creates a bar per output, so with two monitors your `exec` runs twice. *Checked on this machine:* two monitors produced two instances of each module. The factory passes `bar_.output->name` to `Custom` ([src/factory.cpp L358-L360 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/factory.cpp#L358-L360)).

**Differences in v0.15.0** (code at the 0.15.0 tag compared with master man and wiki):
- `alt`, `{alt}`, object-valued `format-icons` and `WAYBAR_OUTPUT_NAME` all **work in 0.15.0** (code cited above). The 0.15.0 man page simply doesn't document them. They were documented on master later ([`git diff 0.15.0 master -- man/waybar-custom.5.scd`](https://github.com/Alexays/Waybar/compare/0.15.0...master)).
- `image-path`, `image-name` and `icon-size` in `custom/*` (master switched it to `AIconLabel`) are **not in 0.15.0**.
- The **`custom-graph/<name>`** module documented on the wiki ([Module: Custom](https://github.com/Alexays/Waybar/wiki/Module:-Custom)) is **not in 0.15.0**. `man/waybar-custom-graph.5.scd` exists only on master.

**Trade-offs**
- ✅ Any language. A crash only kills your process; Waybar logs it and optionally restarts it with `restart-interval`. Styling uses ordinary CSS classes. You can test it from a terminal by just running the program.
- ✅ A continuous process is effectively instant: one line, then a `Glib::Dispatcher` hop to the GTK thread.
- ⚠️ Polling with `interval` forks `/bin/sh` plus your program on every tick. That costs CPU and adds up to `interval` of delay. Prefer continuous mode or `signal`.
- ⚠️ The UI is limited to **one label** (plus an image on master): markup, tooltip, classes and icons. There are no custom widgets.
- ⚠️ Remember to **flush stdout** after every line. Languages that buffer stdout when it isn't a terminal show nothing otherwise (master man page, "CONTINUOUS SCRIPTS").

### 1.2 `cffi/<name>`: in-process shared library with GTK access

Reference: `waybar-cffi(5)` (local; [man/waybar-cffi.5.scd @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/man/waybar-cffi.5.scd)). Header: [resources/custom_modules/cffi_example/waybar_cffi_module.h](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h). Loader: [src/modules/cffi.cpp @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp).

**History.** CFFI was added in [088ca6b9 "Added cffi/* module for third-party advanced modules" (2023-10-24)](https://github.com/Alexays/Waybar/commit/088ca6b9), first released in **0.10.0**. ABI v2 ([90617040 "cffi: always return config values as JSON", 2025-03-05](https://github.com/Alexays/Waybar/commit/90617040)) first shipped in **0.13.0**. The header has not changed between 0.15.0 and master.

**The ABI** (header, with line numbers):

| Symbol | Required | Signature / meaning |
|---|---|---|
| `wbcffi_version` | yes | `extern const size_t wbcffi_version;` must be `1` or `2` ([h L11](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h#L11); loader accepts 1 or 2 only, [cffi.cpp L31](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp#L31)) |
| `wbcffi_init` | yes | `void* wbcffi_init(const wbcffi_init_info*, const wbcffi_config_entry*, size_t len)`. Returns your instance pointer; returning NULL makes Waybar disable the module ([h L58](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h#L58), [cffi.cpp L89-L95](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp#L89-L95)) |
| `wbcffi_deinit` | yes | `void wbcffi_deinit(void* instance)`. Called when the module is destroyed: on exit or on reload ([h L66](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h#L66)) |
| `wbcffi_update` | no | `void wbcffi_update(void*)`. Runs on the GTK main loop after you call `init_info->queue_update(obj)` ([h L74](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h#L74)) |
| `wbcffi_refresh` | no | `void wbcffi_refresh(void*, int signal)`. Called for **every** real-time signal Waybar receives (`SIGRTMIN+1..SIGRTMAX`). The value is the **raw signal number**, not an offset ([h L82](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h#L82); [src/main.cpp L118-L124 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/main.cpp#L118-L124)). *Checked on this machine:* `pkill -RTMIN+8` arrived as `signal 42` (SIGRTMIN = 34). |
| `wbcffi_doaction` | no | `void wbcffi_doaction(void*, const char* name)`. Called with the action names you map in the module's `"actions"` block ([h L91](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h#L91); [wiki Configuration: Module actions config](https://github.com/Alexays/Waybar/wiki/Configuration#module-actions-config)) |

`wbcffi_init_info` ([h L16-L32](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h#L16-L32)) contains:
- `obj`: an opaque handle.
- `waybar_version`.
- `get_root_widget(obj)`: returns the module's `GtkContainer*`, which is Waybar's `Gtk::EventBox` ([cffi.cpp L81-L84](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp#L81-L84)). You add your own widgets to it.
- `queue_update(obj)`: emits a `Glib::Dispatcher` ([L85](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp#L85); [include/AModule.hpp L24](https://github.com/Alexays/Waybar/blob/0.15.0/include/AModule.hpp#L24)). It is safe to call from your own worker threads. Touch GTK only inside `wbcffi_update` or other main-loop callbacks.

**Config passing.** Waybar passes every key in the module's JSON object, `module_path` included, as a `{key, value}` C-string pair. In ABI v2 each value is `Json::Value::toStyledString()`, which means **JSON-encoded and ending in `\n`**: strings arrive quoted, as in `"baz"\n` ([cffi.cpp L55-L77](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp#L55-L77)). In ABI v1, string values arrive bare and everything else arrives as JSON. *Checked on this machine:* the C example printed `bar = "baz"⏎`, `foo = 42⏎`, `module_path = "/…/wb_cffi_example.so"⏎`, with keys in alphabetical order. The pointers are valid **only during `wbcffi_init`** (header comment, [h L53-L54](https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h#L53-L54)).

**`module_path`** is handed straight to `dlopen(path, RTLD_LAZY)` ([cffi.cpp L14-L22](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp#L14-L22)). Waybar does no `~` or `$HOME` expansion, and a relative path resolves against Waybar's working directory. **Use an absolute path.** The man page example `".config/waybar/cffi/…"` works only when Waybar starts from `$HOME`. **Styling:** "The classes and IDs are managed by the cffi dynamic library" (man page). Set your own CSS names with `gtk_widget_set_name` and `gtk_style_context_add_class`.

**The C example** ([resources/custom_modules/cffi_example/](https://github.com/Alexays/Waybar/tree/0.15.0/resources/custom_modules/cffi_example)): `main.c` builds a `GtkBox` containing a label and a "dice" button, and logs each hook call. `meson.build` builds `shared_library('wb_cffi_example', …, dependencies: gtk+-3.0 >=3.22, name_prefix: '')`. *Checked on this machine:* without meson installed, this also builds and loads in Waybar 0.15.0:

```sh
gcc -shared -fPIC -o wb_cffi_example.so main.c $(pkg-config --cflags --libs gtk+-3.0)
```

```jsonc
"cffi/c_example": { "module_path": "/home/kiwi/.config/waybar/cffi/wb_cffi_example.so" }
```

**Differences in v0.15.0**
- **Bug in 0.15.0:** the "missing `wbcffi_deinit`" check tests `hooks_.init` a second time ([cffi.cpp L37-L40 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp#L37-L40)). A library without `wbcffi_deinit` therefore loads and then crashes Waybar on teardown. Fixed on master in [ef3d5598](https://github.com/Alexays/Waybar/commit/ef3d5598), which is not in 0.15.0. **Always export `wbcffi_deinit`.**
- The Arch package **does not ship the header**: `pacman -Ql waybar` lists no `.h` and no `resources/`. Vendor `waybar_cffi_module.h` (MIT) into your project.

**Trade-offs**
- ✅ Full GTK3 widgets (buttons, sliders, popovers, drawing areas), no fork or exec, zero IPC latency, and state that persists for the life of the module.
- ❌ **No crash isolation.** A segfault, an abort, or a Rust panic crossing the FFI boundary kills the whole bar.
- ❌ GTK **3** only, and your library must use the same GTK instance as Waybar, so the language needs GTK3 bindings (C, C++/gtkmm, Rust `gtk` 0.18/0.19, …).
- ❌ **Reloading is awkward.** Waybar never calls `dlclose` (no `dlclose` anywhere in `src/`). After a `SIGUSR2` reload, `dlopen` of the same path returns the copy already loaded, so a rebuilt `.so` is picked up only after a full Waybar restart. *This is inferred from the source plus how glibc `dlopen` behaves, not tested.*
- ❌ One instance per bar/output. Any `static` state in your library is shared across instances, as the C example's `instance_count` demonstrates.

### 1.3 `image` module

Reference: `waybar-image(5)` (local; [man/waybar-image.5.scd @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/man/waybar-image.5.scd)).
- Options: `path`, or `exec` (used only when `path` is unset) whose script prints `$path\n$tooltip`; plus `size`, `interval`, `signal`, `on-click*`, `on-scroll-*`, `tooltip`, `expand`.
- CSS selectors: `#image`, `#image.empty`.
- It suits "render a file that some other program writes", for example album art or a generated sparkline PNG. Your program writes the file and calls `pkill -RTMIN+N waybar`.

**Differences in v0.15.0:** master rewrote `image` to render multiple images and to expand `~`/`$HOME` in `path` (commits [c7f499e3](https://github.com/Alexays/Waybar/commit/c7f499e3) and [16d7c9d8](https://github.com/Alexays/Waybar/commit/16d7c9d8), neither in 0.15.0). In 0.15.0, use absolute paths.

### 1.4 Contributing a compiled-in module upstream

From [CONTRIBUTING.md](https://github.com/Alexays/Waybar/blob/master/CONTRIBUTING.md):
- Build with `meson setup build && ninja -C build`. Use C++20 and the Google style enforced by `.clang-format`.
- Modules derive from `AModule`, `ALabel` or `AIconLabel`. Worker threads must marshal to the GTK thread with `Glib::Dispatcher` / `SafeSignal`.
- New modules need a `man/*.scd` page and an entry in `.github/wiki/mapping.json`.
- Modules are registered in `src/factory.cpp`. The Hyprland ones, for example, are at [L207-L221 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/factory.cpp#L207-L221).
- Trade-off: the best integration and no ABI to maintain. But you get Waybar's release cadence (0.14 to 0.15 took months), C++ only, and the module must be general enough for upstream to accept it.

### 1.5 Comparison

| | `custom/*` continuous | `custom/*` interval | `custom/*` signal | `cffi/*` | `image` | upstream |
|---|---|---|---|---|---|---|
| Latency | event-driven, ~instant | ≤ interval | instant once signalled | instant | ≤ interval / signal | instant |
| Process model | 1 long-lived proc per bar | fork+exec per tick | fork+exec per signal | in-process | fork+exec (if `exec`) | in-process |
| UI | label + markup + tooltip + classes | same | same | any GTK3 widget | image | anything |
| Crash isolation | ✅ (+`restart-interval`) | ✅ | ✅ | ❌ | ✅ | ❌ |
| Language | any | any | any | C ABI + GTK3 bindings | any | C++ |
| Dev loop | run in terminal | run in terminal | run in terminal | rebuild + restart Waybar | — | rebuild Waybar |

---

## 2. Hyprland integration

### 2.1 IPC sockets (Hyprland wiki, [IPC](https://wiki.hypr.land/IPC/))

- The **Hyprland Instance Signature (HIS)** is `$HYPRLAND_INSTANCE_SIGNATURE`.
- **`$XDG_RUNTIME_DIR/hypr/[HIS]/.socket.sock`** takes hyprctl-style requests: "write `[flag(s)]/command args`". The `j` flag means "output in JSON" ([wiki: Using hyprctl, Flags](https://wiki.hypr.land/Configuring/Advanced-and-Cool/Using-hyprctl/)). The wiki warns that Hyprland handles this socket **synchronously**, so "any unclosed connections will cause Hyprland to freeze until the five-second timeout". **Open, write, read, close, every time.**
- **`$XDG_RUNTIME_DIR/hypr/[HIS]/.socket2.sock`** is the event stream. Hyprland writes `EVENT>>DATA\n` to every connected client, for example `workspace>>2`.
- Waybar's own Hyprland backend uses the same paths, falling back to `/tmp/hypr` if `$XDG_RUNTIME_DIR/hypr` is missing ([src/modules/hyprland/backend.cpp L19-L43, L98, L198 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/hyprland/backend.cpp#L19-L43)). Its source notes "Hyprland socket2 events are max 1024 bytes" ([L115](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/hyprland/backend.cpp#L115)), and it sends JSON requests as `"j/" + rq` ([L236-L237](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/hyprland/backend.cpp#L236-L237)).
- *Checked on this machine:* both sockets exist under `/run/user/1000/hypr/<HIS>/`, and `.socket2.sock` delivered `windowtitle>>559653744a30` and `windowtitlev2>>559653744a30,<title>`.

**Events** (full table on [wiki.hypr.land/IPC](https://wiki.hypr.land/IPC/#events-list); `v2` variants add IDs):
- Workspaces: `workspace`/`workspacev2`, `focusedmon`/`focusedmonv2`, `createworkspace(v2)`, `destroyworkspace(v2)`, `moveworkspace(v2)`, `renameworkspace`, `activespecial(v2)`.
- Windows: `activewindow` (`CLASS,TITLE`), `activewindowv2` (`ADDRESS`), `openwindow` (`ADDRESS,WORKSPACENAME,CLASS,TITLE`), `closewindow`, `kill`, `movewindow(v2)`, `windowtitle(v2)`, `fullscreen` (0/1), `changefloatingmode`, `urgent`, `pin`, `minimized`.
- Groups: `togglegroup`, `moveintogroup`, `moveoutofgroup`, `ignoregrouplock`, `lockgroups`.
- Misc: `monitoradded(v2)`, `monitorremoved(v2)`, `activelayout` (`KEYBOARD,LAYOUT`), `submap`, `openlayer`, `closelayer`, `screencast(v2)`, `configreloaded`, `bell`.
- The wiki warns that `fullscreen` "is not guaranteed to fire on/off once in succession". Treat events as "something changed, re-query" and don't track state from deltas.

### 2.2 Built-in `hyprland/*` modules in Waybar 0.15.0 (reference implementations)

- `hyprland/workspaces`, `hyprland/window`, `hyprland/windowcount`, `hyprland/language`, `hyprland/submap` ([factory.cpp L207-L221 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/factory.cpp#L207-L221); sources under [src/modules/hyprland/](https://github.com/Alexays/Waybar/tree/0.15.0/src/modules/hyprland)).
- They share one singleton IPC backend thread that reads `.socket2.sock` line by line and fans events out to registered `EventHandler`s ([backend.cpp L95-L140](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/hyprland/backend.cpp#L95-L140)).
- Man pages: `waybar-hyprland-{workspaces,window,language,submap}(5)` are installed locally. `man/waybar-hyprland-windowcount.5.scd` exists in the 0.15.0 source tree but is not in Arch's package file list.
- Check these modules first: if one can already do what you need with `format`/`rewrite` options, don't write a plugin.

### 2.3 Event-driven custom module: working example

*Checked on this machine* (Python 3 stdlib only). Run directly, it printed `{"text": "1", "alt": "normal", "tooltip": "1 window(s) on DP-1", "class": [], "percentage": 20}`. A throwaway Waybar 0.15.0 instance loaded it without errors.

```python
#!/usr/bin/env python3
"""Waybar custom module: active workspace, updated on Hyprland events (no polling)."""
import json, os, socket, sys

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
    print(json.dumps({
        "text": ws["name"],
        "alt": "fullscreen" if ws["hasfullscreen"] else "normal",
        "tooltip": f"{ws['windows']} window(s) on {ws['monitor']}",
        "class": ["fullscreen"] if ws["hasfullscreen"] else [],
        "percentage": min(100, ws["windows"] * 20),
    }), flush=True)                     # one object per line, flushed

INTERESTING = {"workspacev2", "focusedmonv2", "openwindow", "closewindow",
               "movewindowv2", "fullscreen"}

def main():
    emit()                              # initial state
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as ev:
        ev.connect(os.path.join(sock_dir(), ".socket2.sock"))
        for line in ev.makefile("r", encoding="utf-8", errors="replace"):
            name, _, _data = line.rstrip("\n").partition(">>")
            if name in INTERESTING:
                emit()

if __name__ == "__main__":
    try:
        main()
    except (BrokenPipeError, KeyboardInterrupt):
        sys.exit(0)
```

```jsonc
// ~/.config/waybar/config.jsonc
"modules-left": ["hyprland/workspaces", "custom/hypr-ws"],
"custom/hypr-ws": {
  "exec": "python3 -I ~/.config/waybar/scripts/hypr-workspace.py",
  "return-type": "json",
  "restart-interval": 5,          // no "interval": this is a continuous script
  "format": "{icon} {text}",
  "format-icons": { "normal": "", "fullscreen": "", "default": "" }
}
```

```css
/* ~/.config/waybar/style.css */
#custom-hypr-ws            { padding: 0 8px; }
#custom-hypr-ws.fullscreen { color: #f38ba8; }
```

Notes:
- `restart-interval` re-spawns the script if it exits, for example when Hyprland restarts and the socket closes.
- If you would rather use bash, the wiki's example is `socat -U - UNIX-CONNECT:$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock | while read -r line; do …; done` ([wiki IPC: How to use socket2 with bash](https://wiki.hypr.land/IPC/)). `socat` is **not installed** on this machine.
- **Environment:** `HYPRLAND_INSTANCE_SIGNATURE` must reach Waybar. Starting Waybar from Hyprland with `exec-once` passes it on. The packaged `waybar.service` (`/usr/lib/systemd/user/waybar.service`, `PartOf=graphical-session.target`, `ExecReload=kill -SIGUSR2 $MAINPID`) depends on the session importing that variable into the systemd user environment. `waybar.service` is currently `disabled` on this machine.

---

## 3. Libraries, bindings and templates (verified)

| Name | Lang | Purpose | Latest | Activity | Notes |
|---|---|---|---|---|---|
| [`waybar-cffi`](https://crates.io/crates/waybar-cffi) + [`waybar-cffi-sys`](https://crates.io/crates/waybar-cffi-sys) ([repo LawnGnome/waybar-cffi-rs](https://github.com/LawnGnome/waybar-cffi-rs)) | Rust | Safe CFFI wrapper: `Module` trait (`init`, `update`, `refresh`, `do_action`) + `waybar_module!` macro; re-exports `gtk` 0.18; config is deserialised with serde | 0.1.1 (2025-03-06) | Last push 2025-03-06; 7★. README: "I probably won't be devoting a tonne of time to this". | Exports **`wbcffi_version = 1`** (ABI v1, still accepted by 0.15.0). Built against Waybar 0.12.0 (`waybar-cffi/WAYBAR_VERSION`). **README example uses `"module-path"`, but Waybar reads `"module_path"`** ([cffi.cpp L14](https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp#L14)). Use the underscore. The master man page names it as the Rust binding. |
| [`hyprland`](https://crates.io/crates/hyprland) ([hyprland-community/hyprland-rs](https://github.com/hyprland-community/hyprland-rs)) | Rust | Hyprland IPC: `data`, `event_listener`, `dispatch`, `keyword`, `ctl` | stable 0.3.13; README recommends **0.4.0-beta.3** (2025-09-24) | Last commit 2026-07-26; 417★ | The 0.4 line has been in beta since 2024; pin exact versions. |
| [`hyprland-go`](https://github.com/thiagokokada/hyprland-go) | Go | IPC requests + event listener, zero deps | v0.4.1 (2025-03-17) | Last push 2025-09-30; 45★; self-labelled alpha; badge says Hyprland 0.44 | |
| [`hyprpy`](https://pypi.org/project/hyprpy/) ([ulinja/hyprpy](https://github.com/ulinja/hyprpy)) | Python | Typed bindings + event monitor | 0.2.1 (2026-01-24) | Last push 2026-01-25; AUR `python-hyprpy` | |
| [`hyprland-ipc`](https://pypi.org/project/hyprland-ipc/) ([peppapig450/hyprland-ipc](https://github.com/peppapig450/hyprland-ipc)) | Python | Typed IPC client | 0.1.0 (2025-06-05) | Last push 2026-10-19; 1★ | Very small project |
| [`hyprland`](https://pypi.org/project/hyprland/) ([hyprland-community/hyprland-py](https://github.com/hyprland-community/hyprland-py)) | Python | Async IPC | 0.2.54 (2023-09-25) | **Archived** | Avoid |
| [`pyprland`](https://github.com/hyprland-community/pyprland) | Python | Plugin daemon for Hyprland (not Waybar) | 3.4.6 (2026-10-18) | Active; 710★ | Not a Waybar library |
| [`waybar-module-pacman-updates`](https://github.com/coffebar/waybar-module-pacman-updates) | Rust | Real-world continuous `custom/*` module | 0.2.14 (2026-07-30) | Active; listed on the [wiki's custom plugin page](https://github.com/Alexays/Waybar/wiki/Add-a-custom-plugin.) | Good structural reference |

- **Searched for, not found:** a crate named `waybar` (crates.io: "does not exist"), and a Zig CFFI binding. The master man page says Zig bindings exist, but a GitHub search for "waybar zig" / "waybar cffi" found no binding library. [erffy/zig-waybar-contrib](https://github.com/erffy/zig-waybar-contrib) is a collection of `custom/*` modules written in Zig, not a binding.
- **Small CFFI module repos exist** (search "waybar cffi"), for example `MasonRhodesDev/waybar-workspace-buttons` (C, Hyprland, pushed 2026-10-12). They all have 0–5 stars and none is a maintained template.
- **No official template** exists. The only "official" starting points are the C CFFI example in-tree and the man page snippets.

---

## 4. Recommendation for `waybar_gpui_plugin_template`

### Option A: **`custom/*` continuous module in Rust** (recommended)
- Event-driven JSON-lines program. A direct Unix-socket client for Hyprland is about 60 lines with `std::os::unix::net::UnixStream`. Optionally offer a feature flag that uses the `hyprland` crate instead.
- The output schema is a typed `serde` struct `{text, alt, tooltip, class: Vec<String>, percentage: Option<u8>}`, printed with `serde_json::to_string` plus `println!`. `println!` flushes on newline only when stdout is a terminal, so lock stdout and flush explicitly.
- ✅ Crash-isolated, a single binary, a few MB of memory, instant updates, testable from a terminal, and unaffected by Waybar's ABI or GTK version.
- ⚠️ You only get a label, tooltip, classes and icons. There is no Rust toolchain cost for users who install a prebuilt binary.

### Option B: **`custom/*` continuous module in Python (stdlib only)**
- The §2.3 script, generalised.
- ✅ Zero build step, the easiest to hack on, and already verified on this machine.
- ⚠️ Each instance is a Python interpreter, roughly 10–20 MB RSS, and one runs per monitor. There is no type safety on the output schema.
- Good as the "quick start" flavour of the template.

### Option C: **`cffi/*` module in Rust (`waybar-cffi`) or C**
- ✅ Real GTK widgets, no subprocesses.
- ❌ Brings down the bar on a crash or panic. The binding has a single maintainer, low activity and ABI v1. Picking up a new build requires a full Waybar restart. The library must use the same GTK3 Waybar uses.
- Worth it only when you need interactive widgets (buttons, sliders, drawing).
- If chosen: vendor `waybar_cffi_module.h`, always export `wbcffi_deinit` (because of the 0.15.0 bug), use an absolute `module_path`, and call `queue_update` from threads instead of touching GTK there.

**Recommendation:** build the template around **Option A, with Option B shipped alongside as a quick start**. Both use the same JSON contract, the same config snippet shape and the same CSS. Add Option C later as an `examples/cffi-rust/` folder for widget-heavy use cases.

### Suggested layout

```
waybar_gpui_plugin_template/
├── Cargo.toml                  # bin crate (Option A)
├── src/
│   ├── main.rs                 # loop: emit(); for event in hypr_events() { if relevant { emit() } }
│   ├── hypr.rs                 # socket paths, request("j/activeworkspace"), event iterator
│   └── output.rs               # #[derive(Serialize)] struct Output { text, alt, tooltip, class, percentage }
├── python/
│   └── module.py               # stdlib-only variant (Option B)
├── examples/
│   ├── config.jsonc            # snippet to paste into ~/.config/waybar/config.jsonc
│   └── style.css               # #custom-<name> and #custom-<name>.<class> rules
├── scripts/
│   └── dev.sh                  # build + run a second waybar with examples/ config, -l debug
├── Makefile / justfile         # build, install (~/.local/bin), reload (pkill -SIGUSR2 waybar)
└── docs/research/waybar-custom-modules.md
```

### Wiring and development loop
- **Config files:** Waybar reads `config` / `config.jsonc` from `$XDG_CONFIG_HOME/waybar/`, `~/.config/waybar/`, `~/waybar/` or `/etc/xdg/waybar/`, in JSONC format ([wiki Configuration](https://github.com/Alexays/Waybar/wiki/Configuration)). Add `"custom/<name>"` to a `modules-*` array and add a `"custom/<name>": {…}` block. The user's current config already uses `hyprland/workspaces` and `hyprland/window`.
- **Isolated test bar:** run a second bar with `waybar -c examples/config.jsonc -s examples/style.css -l debug` (CLI flags in [src/client.cpp L249-L253 @0.15.0](https://github.com/Alexays/Waybar/blob/0.15.0/src/client.cpp#L249-L253)). Before that, run the module alone in a terminal to check that its JSON lines are well-formed.
- **Reload:** `pkill -SIGUSR2 waybar` reloads the config and rebuilds all modules (`on-sigusr2` default `reload`, `waybar(5)` "SIGNALS"; [main.cpp L86-L110](https://github.com/Alexays/Waybar/blob/0.15.0/src/main.cpp#L86-L110)). Custom-module processes are killed and re-spawned, which picks up a rebuilt binary. `"reload_style_on_change": true` reloads CSS automatically whenever the file changes (`waybar(5)`). `systemctl --user reload waybar` does the same as `SIGUSR2` if you use the service.
- **Manual refresh:** for `signal`-driven modules, run `pkill -RTMIN+N waybar`.
- **Pitfall:** CFFI needs a full restart (§1.2).
- **Pitfall:** stdout from a CFFI library goes into Waybar's own stdout and is block-buffered when redirected. *Seen on this machine:* `printf` lines were lost when Waybar was killed. Log to stderr or use `-l debug`.

---

## Sources

**Waybar (source at tag 0.15.0 unless noted)**
- https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/custom.cpp
- https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/cffi.cpp
- https://github.com/Alexays/Waybar/blob/0.15.0/include/modules/cffi.hpp
- https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/waybar_cffi_module.h
- https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/main.c
- https://github.com/Alexays/Waybar/blob/0.15.0/resources/custom_modules/cffi_example/meson.build
- https://github.com/Alexays/Waybar/blob/0.15.0/src/ALabel.cpp
- https://github.com/Alexays/Waybar/blob/0.15.0/include/AModule.hpp
- https://github.com/Alexays/Waybar/blob/0.15.0/include/util/command.hpp
- https://github.com/Alexays/Waybar/blob/0.15.0/src/main.cpp
- https://github.com/Alexays/Waybar/blob/0.15.0/src/client.cpp
- https://github.com/Alexays/Waybar/blob/0.15.0/src/factory.cpp
- https://github.com/Alexays/Waybar/blob/0.15.0/src/modules/hyprland/backend.cpp
- https://github.com/Alexays/Waybar/blob/0.15.0/man/waybar-custom.5.scd, `waybar-cffi.5.scd`, `waybar-image.5.scd`, `waybar.5.scd.in`
- https://github.com/Alexays/Waybar/blob/master/CONTRIBUTING.md
- https://github.com/Alexays/Waybar/compare/0.15.0...master (master man page additions; commits 088ca6b9, 90617040, ef3d5598, c7f499e3, 16d7c9d8, f72f84e0)
- Wiki: https://github.com/Alexays/Waybar/wiki/Module:-Custom, https://github.com/Alexays/Waybar/wiki/Module:-CFFI, https://github.com/Alexays/Waybar/wiki/Configuration, https://github.com/Alexays/Waybar/wiki/Add-a-custom-plugin.
- Local: `/usr/share/man/man5/waybar-custom.5.gz`, `waybar-cffi.5.gz`, `waybar-image.5.gz`; `pacman -Ql waybar`; `/usr/lib/systemd/user/waybar.service`; `waybar --version` (v0.15.0)

**Hyprland**
- https://wiki.hypr.land/IPC/ (sockets, event list, socat example; page last updated 2026-08-26)
- https://wiki.hypr.land/Configuring/Advanced-and-Cool/Using-hyprctl/ (flags: `j` = JSON, `--batch`)
- Local: `hyprctl version` (v0.56.2), `/run/user/1000/hypr/<HIS>/.socket.sock`, `.socket2.sock`

**Libraries**
- https://github.com/LawnGnome/waybar-cffi-rs, https://crates.io/crates/waybar-cffi, https://crates.io/crates/waybar-cffi-sys
- https://github.com/hyprland-community/hyprland-rs, https://crates.io/crates/hyprland
- https://github.com/thiagokokada/hyprland-go
- https://github.com/ulinja/hyprpy, https://pypi.org/project/hyprpy/
- https://github.com/peppapig450/hyprland-ipc, https://pypi.org/project/hyprland-ipc/
- https://github.com/hyprland-community/hyprland-py (archived), https://pypi.org/project/hyprland/
- https://github.com/hyprland-community/pyprland
- https://github.com/coffebar/waybar-module-pacman-updates
- https://github.com/erffy/zig-waybar-contrib
- https://crates.io/crates/gtk (gtk3-rs bindings, 0.19.0 released 2026-09-08)
