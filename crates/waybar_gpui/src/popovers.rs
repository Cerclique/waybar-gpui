//! The popover rules, without GPUI: what a command or a close does to the open
//! popover, and when the process quits. `runtime.rs` carries out the
//! [`Effect`]s and reports back whether an open worked.

use crate::ipc::{Command, Target};

/// The open popover, if any, and what the process runs as. `W` is the window
/// the runtime opened for it, handed back in [`Effect::Close`].
pub struct Popovers<W> {
    open: Option<(Target, W)>,
    /// Running as `preview`: closing the popover quits.
    preview: bool,
}

/// What the runtime does next, in order.
#[derive(Debug, PartialEq, Eq)]
pub enum Effect<W> {
    /// Remove `window` and log `closed <target> (<reason>)`.
    Close { target: Target, window: W, reason: &'static str },
    /// Build the panel fresh and open its popover, then report
    /// [`Popovers::opened`] or [`Popovers::open_failed`].
    Open(Target),
    /// Remove the daemon's socket file.
    RemoveSocket,
    /// Stop the process; `failed` makes it exit with failure.
    Quit { failed: bool },
}

impl<W> Popovers<W> {
    /// Nothing open yet.
    pub fn new(preview: bool) -> Self {
        Self { open: None, preview }
    }

    /// A command from the socket (or the target a daemon or preview starts
    /// with, as a toggle).
    pub fn on(&mut self, command: Command) -> Vec<Effect<W>> {
        match command {
            Command::Toggle(target) => {
                let same = self.open.as_ref().is_some_and(|(open, _)| *open == target);
                let mut effects = self.close("toggle");
                // Same panel with the same arguments: close. Anything else:
                // switch, unless that close ended a preview.
                if !same && !effects.iter().any(|e| matches!(e, Effect::Quit { .. })) {
                    effects.push(Effect::Open(target));
                }
                effects
            }
            Command::Close => self.close("close command"),
            Command::Quit => {
                let mut effects: Vec<_> = self.take_open("quit").into_iter().collect();
                effects.extend([Effect::RemoveSocket, Effect::Quit { failed: false }]);
                effects
            }
        }
    }

    /// The popover for `target` is open in `window`.
    pub fn opened(&mut self, target: Target, window: W) {
        debug_assert!(self.open.is_none(), "opened {target} over another popover");
        self.open = Some((target, window));
    }

    /// The last [`Effect::Open`] failed (already logged by the runtime).
    pub fn open_failed(&mut self) -> Vec<Effect<W>> {
        if self.preview { vec![Effect::Quit { failed: true }] } else { Vec::new() }
    }

    /// Closes the open popover, if any, for `reason`: a dismissal (Esc, click
    /// outside, click on another monitor) or the panel's own `close(cx)`.
    pub fn close(&mut self, reason: &'static str) -> Vec<Effect<W>> {
        let Some(close) = self.take_open(reason) else { return Vec::new() };
        let mut effects = vec![close];
        if self.preview {
            effects.push(Effect::Quit { failed: false });
        }
        effects
    }

    fn take_open(&mut self, reason: &'static str) -> Option<Effect<W>> {
        let (target, window) = self.open.take()?;
        Some(Effect::Close { target, window, reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(panel: &str, args: &[&str]) -> Target {
        Target { panel: panel.into(), args: args.iter().map(|a| a.to_string()).collect() }
    }

    fn toggle(panel: &str, args: &[&str]) -> Command {
        Command::Toggle(target(panel, args))
    }

    fn close(panel: &str, args: &[&str], window: u32, reason: &'static str) -> Effect<u32> {
        Effect::Close { target: target(panel, args), window, reason }
    }

    /// A daemon (or preview) with `clock Paris` open in window 1.
    fn with_clock_open(preview: bool) -> Popovers<u32> {
        let mut popovers = Popovers::new(preview);
        assert_eq!(popovers.on(toggle("clock", &["Paris"])), [Effect::Open(target("clock", &["Paris"]))]);
        popovers.opened(target("clock", &["Paris"]), 1);
        popovers
    }

    #[test]
    fn toggle_opens_when_nothing_is_open() {
        with_clock_open(false);
    }

    #[test]
    fn toggle_of_the_open_target_closes_it() {
        let mut popovers = with_clock_open(false);
        assert_eq!(popovers.on(toggle("clock", &["Paris"])), [close("clock", &["Paris"], 1, "toggle")]);
        // Closed: the next toggle opens again.
        assert_eq!(popovers.on(toggle("clock", &["Paris"])), [Effect::Open(target("clock", &["Paris"]))]);
    }

    #[test]
    fn toggle_of_another_target_switches() {
        for other in [target("clock", &["Tokyo"]), target("clock", &[]), target("workspace", &["Paris"])] {
            let mut popovers = with_clock_open(false);
            assert_eq!(
                popovers.on(Command::Toggle(other.clone())),
                [close("clock", &["Paris"], 1, "toggle"), Effect::Open(other.clone())]
            );
            popovers.opened(other.clone(), 2);
            assert_eq!(
                popovers.on(Command::Close),
                [Effect::Close { target: other, window: 2, reason: "close command" }]
            );
        }
    }

    #[test]
    fn close_command_closes() {
        let mut popovers = with_clock_open(false);
        assert_eq!(popovers.on(Command::Close), [close("clock", &["Paris"], 1, "close command")]);
        assert_eq!(popovers.on(Command::Close), []);
    }

    #[test]
    fn quit_closes_removes_the_socket_and_stops() {
        let mut popovers = with_clock_open(false);
        assert_eq!(
            popovers.on(Command::Quit),
            [close("clock", &["Paris"], 1, "quit"), Effect::RemoveSocket, Effect::Quit { failed: false }]
        );
        let mut idle = Popovers::<u32>::new(false);
        assert_eq!(idle.on(Command::Quit), [Effect::RemoveSocket, Effect::Quit { failed: false }]);
    }

    #[test]
    fn daemon_keeps_running_after_a_close_or_a_failed_open() {
        let mut popovers = with_clock_open(false);
        assert_eq!(popovers.close("escape"), [close("clock", &["Paris"], 1, "escape")]);
        // A second dismissal of the same popover (Esc and a click in one frame).
        assert_eq!(popovers.close("click outside"), []);
        assert_eq!(popovers.on(toggle("nope", &[])), [Effect::Open(target("nope", &[]))]);
        assert_eq!(popovers.open_failed(), []);
        assert_eq!(popovers.on(toggle("clock", &["Paris"])), [Effect::Open(target("clock", &["Paris"]))]);
    }

    #[test]
    fn preview_quits_when_its_popover_closes() {
        for reason in ["escape", "click outside", "click on another monitor", "panel"] {
            let mut popovers = with_clock_open(true);
            assert_eq!(popovers.close(reason), [close("clock", &["Paris"], 1, reason), Effect::Quit { failed: false }]);
        }
        let mut popovers = with_clock_open(true);
        assert_eq!(
            popovers.on(Command::Close),
            [close("clock", &["Paris"], 1, "close command"), Effect::Quit { failed: false }]
        );
    }

    #[test]
    fn preview_fails_when_its_panel_cannot_open() {
        let mut popovers = Popovers::<u32>::new(true);
        assert_eq!(popovers.on(toggle("clock", &[])), [Effect::Open(target("clock", &[]))]);
        assert_eq!(popovers.open_failed(), [Effect::Quit { failed: true }]);
    }
}
