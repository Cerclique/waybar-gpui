//! A first panel: text in a card and a button that closes the popover.
//! Try it with `cargo run -- preview hello`.

{% if components -%}
use waybar_gpui::{
    Panel, card, close,
    components::button::{Button, ButtonVariants as _},
    gpui::{Context, IntoElement, Window, prelude::*},
};
{%- else -%}
use waybar_gpui::{
    Panel, Theme, card, close,
    gpui::{Context, IntoElement, Window, div, prelude::*},
};
{%- endif %}

pub struct Hello;

impl Panel for Hello {
    const NAME: &'static str = "hello";

    fn new(_args: &[String], _: &mut Window, _: &mut Context<Self>) -> Self {
        Hello
    }
}

impl Render for Hello {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
{%- if components %}
        let button = Button::new("close")
            .primary()
            .label("Close")
            .on_click(|_, _, cx| close(cx));
        card(cx).child("Hello from {{project-name}}").child(button)
{%- else %}
        let theme = Theme::get(cx);
        let button = div()
            .id("close")
            .px_3()
            .py_1()
            .rounded_md()
            .bg(theme.surface)
            .hover(|style| style.bg(theme.overlay))
            .on_click(|_, _, cx| close(cx))
            .child("Close");
        card(cx).child("Hello from {{project-name}}").child(button)
{%- endif %}
    }
}
