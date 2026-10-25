# No gpui-component `Root` around popovers

With the `components` feature the runtime initialises gpui-component and themes it, but does not wrap popovers in its `Root`. On Linux `Root` always draws `window_border()` (a 20 px inset per side and a shadow) because layer surfaces get client-side decorations; on our full-monitor surface that grew a 1920×1080 surface to 1960×1120 and veiled the whole output.

## Consequences

gpui-component dialogs and notifications need `Root` and are unsupported. Menus and in-window popovers work without it.
