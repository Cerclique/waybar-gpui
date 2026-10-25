//! The registered panels: the one place a panel is looked up by name.

use gpui::{AnyView, AppContext as _, Window};

use crate::Panel;

/// Builds a panel's view as its popover opens.
pub type Build = fn(&[String], &mut Window, &mut gpui::App) -> AnyView;

fn build<P: Panel>(args: &[String], window: &mut Window, cx: &mut gpui::App) -> AnyView {
    cx.new(|cx| P::new(args, window, cx)).into()
}

/// A program's panels by name, in registration order; names are unique.
#[derive(Default)]
pub struct Panels(Vec<(&'static str, Build)>);

impl Panels {
    /// Registers `P` under its [`Panel::NAME`].
    ///
    /// # Panics
    ///
    /// If a panel with the same name is already registered.
    pub fn add<P: Panel>(&mut self) {
        assert!(self.get(P::NAME).is_none(), "waybar_gpui: two panels are named {:?}", P::NAME);
        self.0.push((P::NAME, build::<P>));
    }

    /// The names, in registration order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.0.iter().map(|(name, _)| *name)
    }

    /// The build function of the panel named `name`.
    pub fn get(&self, name: &str) -> Option<Build> {
        self.0.iter().find(|(n, _)| *n == name).map(|(_, build)| *build)
    }
}

/// Defines a panel named `$name` that renders nothing, for tests.
#[cfg(test)]
macro_rules! test_panel {
    ($ty:ident, $name:literal) => {
        struct $ty;
        impl gpui::Render for $ty {
            fn render(&mut self, _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
                gpui::div()
            }
        }
        impl $crate::Panel for $ty {
            const NAME: &'static str = $name;
            fn new(_: &[String], _: &mut gpui::Window, _: &mut gpui::Context<Self>) -> Self {
                $ty
            }
        }
    };
}
#[cfg(test)]
pub(crate) use test_panel;

#[cfg(test)]
mod tests {
    use super::*;

    test_panel!(Workspace, "workspace");
    test_panel!(Clock, "clock");
    test_panel!(OtherClock, "clock");

    fn panels() -> Panels {
        let mut panels = Panels::default();
        panels.add::<Workspace>();
        panels.add::<Clock>();
        panels
    }

    #[test]
    fn names_in_registration_order() {
        assert_eq!(panels().names().collect::<Vec<_>>(), ["workspace", "clock"]);
        assert_eq!(Panels::default().names().count(), 0);
    }

    #[test]
    fn looks_panels_up_by_name() {
        let panels = panels();
        assert!(panels.get("workspace").is_some());
        assert!(panels.get("clock").is_some());
        assert!(panels.get("calendar").is_none());
        assert!(panels.get("Clock").is_none());
    }

    #[test]
    #[should_panic(expected = "waybar_gpui: two panels are named \"clock\"")]
    fn rejects_duplicate_names() {
        panels().add::<OtherClock>();
    }
}
