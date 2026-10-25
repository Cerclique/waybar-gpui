//! gpui-component widgets inside a popover, themed from the SDK `Theme`, plus
//! a dropdown menu that overflows the card without dismissing the popover.
//! Needs the `components` feature of waybar_gpui.

use waybar_gpui::{
    Panel, card, close,
    components::{
        button::{Button, ButtonVariants as _, DropdownButton},
        menu::PopupMenuItem,
        switch::Switch,
    },
    gpui::{self, Context, IntoElement, Window, div, prelude::*},
};

pub struct ControlsPanel {
    wifi: bool,
    bluetooth: bool,
    picked: &'static str,
}

impl Panel for ControlsPanel {
    const NAME: &'static str = "controls";

    fn new(_args: &[String], _: &mut Window, _: &mut Context<Self>) -> Self {
        Self { wifi: true, bluetooth: false, picked: "Balanced" }
    }
}

impl Render for ControlsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let wifi_entity = entity.clone();
        let bt_entity = entity.clone();
        card(cx)
            .w(gpui::px(280.))
            .child(div().flex().justify_between().child("Wi-Fi").child(
                Switch::new("wifi").checked(self.wifi).on_click(move |on, _, cx| {
                    wifi_entity.update(cx, |p, cx| {
                        p.wifi = *on;
                        cx.notify();
                    })
                }),
            ))
            .child(div().flex().justify_between().child("Bluetooth").child(
                Switch::new("bt").checked(self.bluetooth).on_click(move |on, _, cx| {
                    bt_entity.update(cx, |p, cx| {
                        p.bluetooth = *on;
                        cx.notify();
                    })
                }),
            ))
            .child(div().flex().justify_between().items_center().child("Power profile").child({
                let picked = self.picked;
                let entity = entity.clone();
                DropdownButton::new("profile").button(Button::new("profile-btn").label(picked)).dropdown_menu(
                    move |menu, _, _| {
                        ["Power saver", "Balanced", "Performance"].into_iter().fold(menu, |menu, name| {
                            let entity = entity.clone();
                            menu.item(PopupMenuItem::new(name).on_click(move |_, _, cx| {
                                entity.update(cx, |p, cx| {
                                    p.picked = name;
                                    cx.notify();
                                })
                            }))
                        })
                    },
                )
            }))
            .child(Button::new("done").primary().label("Done").on_click(|_, _, cx| close(cx)))
    }
}
