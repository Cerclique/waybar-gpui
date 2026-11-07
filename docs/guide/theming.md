# Theming

The runtime paints nothing: the popover's surface is transparent, and the
**card** is whatever your panel draws. The SDK gives you two opt-in pieces.

## `Theme`

A plain struct of colours, stored as a GPUI global and read with
`Theme::get(cx)`. The default is Catppuccin Mocha.

| Field | Used for |
|---|---|
| `base` | Card background (translucent by default). |
| `surface` | Raised rows, selected items. |
| `overlay` | Borders, hover. |
| `text` | Main text. |
| `subtext` | Secondary text. |
| `accent` | Highlights, focus rings, primary buttons. |

Give your program its own by changing the default's colours. (`Theme` may
gain colours in a later release, so a `Theme { … }` literal doesn't compile.)

```rust
use waybar_gpui::{gpui::{rgb, rgba}, Theme};

let mut theme = Theme::default();
theme.base = rgba(0x2e3440f0);
theme.accent = rgb(0x88c0d0);
waybar_gpui::Program::new("my-panels").theme(theme).panel::<Hello>().run()
```

Use it in your panels so they follow it:

```rust
let theme = Theme::get(cx);
div().text_color(theme.subtext).child("secondary")
```

## `card()`

`card(cx)` returns a `div` with the standard chrome: a column with gaps and
padding, rounded corners, a 1 px `overlay` border, `base` background, `text`
colour, small text. It's a starting point you can chain onto
(`card(cx).w(px(300.))`), or skip entirely and draw your own shape: the
runtime places whatever your panel's root element is.

## Waybar's style

Your panels don't read Waybar's `style.css`; match its colours through
`Theme`. The optional Hyprland rule in [Installing](installing.md#hyprland)
adds blur behind the card.

With the [`components`](components.md) feature, `Theme` also colours
gpui-component's widgets.
