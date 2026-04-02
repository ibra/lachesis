use std::time::Duration;

/// Information about the currently focused window.
#[derive(Debug, Clone, PartialEq)]
pub struct FocusInfo {
    pub process_name: String,
    pub exe_path: Option<String>,
    pub window_title: Option<String>,
}

/// Platform-specific interface for getting the focused window and idle state.
pub trait FocusTracker {
    /// Get information about the currently focused/foreground window.
    /// Returns None if no window is focused or if the query fails.
    fn get_focused_window(&self) -> Option<FocusInfo>;

    /// Get how long the user has been idle (no keyboard/mouse input).
    fn get_idle_duration(&self) -> Duration;
}

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

/// Create a FocusTracker for the current platform.
pub fn create_tracker() -> Box<dyn FocusTracker> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsFocusTracker::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxFocusTracker::new())
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsFocusTracker::new())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        compile_error!("unsupported platform: lachesis requires windows, linux, or macos")
    }
}
