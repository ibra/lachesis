use super::{FocusInfo, FocusTracker};
use std::time::Duration;

/// Stub implementation for macOS.
/// Full support requires NSWorkspace bindings and is tracked in issue #18.
pub struct MacOsFocusTracker;

impl MacOsFocusTracker {
    pub fn new() -> Self {
        MacOsFocusTracker
    }
}

impl FocusTracker for MacOsFocusTracker {
    fn get_focused_window(&self) -> Option<FocusInfo> {
        // TODO: implement via NSWorkspace.shared.frontmostApplication (issue #18)
        None
    }

    fn get_idle_duration(&self) -> Duration {
        // TODO: implement via CGEventSourceSecondsSinceLastEventType (issue #18)
        Duration::ZERO
    }
}
