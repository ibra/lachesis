use super::{FocusInfo, FocusTracker};
use std::time::Duration;

/// Stub implementation for Linux.
/// Full X11 support requires the x11rb crate and is tracked in issue #17.
pub struct LinuxFocusTracker;

impl LinuxFocusTracker {
    pub fn new() -> Self {
        LinuxFocusTracker
    }
}

impl FocusTracker for LinuxFocusTracker {
    fn get_focused_window(&self) -> Option<FocusInfo> {
        // TODO: implement via _NET_ACTIVE_WINDOW (issue #17)
        None
    }

    fn get_idle_duration(&self) -> Duration {
        // TODO: implement via XScreenSaverQueryInfo (issue #17)
        Duration::ZERO
    }
}
