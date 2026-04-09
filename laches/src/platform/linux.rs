use super::{normalize_process_name, FocusInfo, FocusTracker};
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::screensaver;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

/// Pre-interned X11 atoms for window property queries.
/// Interned once at connection time to avoid per-poll round-trips.
struct Atoms {
    net_active_window: Atom,
    net_wm_pid: Atom,
    net_wm_name: Atom,
    wm_name: Atom,
    utf8_string: Atom,
}

/// Holds a live X11 connection, the root window, and cached atoms.
struct X11Connection {
    conn: RustConnection,
    root: Window,
    atoms: Atoms,
}

impl X11Connection {
    /// Read a single 32-bit cardinal property from a window.
    fn get_property_u32(
        &self,
        window: Window,
        property: Atom,
        type_: impl Into<Atom>,
    ) -> Option<u32> {
        self.conn
            .get_property(false, window, property, type_, 0, 1)
            .ok()?
            .reply()
            .ok()?
            .value32()?
            .next()
    }

    /// Read a variable-length string property from a window.
    fn get_property_string(
        &self,
        window: Window,
        property: Atom,
        type_: impl Into<Atom>,
    ) -> Option<String> {
        let reply = self
            .conn
            .get_property(false, window, property, type_, 0, 1024)
            .ok()?
            .reply()
            .ok()?;

        if reply.value.is_empty() {
            return None;
        }

        Some(String::from_utf8_lossy(&reply.value).into_owned())
    }

    /// Get the window title, preferring _NET_WM_NAME (UTF-8) over WM_NAME (latin-1).
    fn get_window_title(&self, window: Window) -> Option<String> {
        self.get_property_string(window, self.atoms.net_wm_name, self.atoms.utf8_string)
            .or_else(|| self.get_property_string(window, self.atoms.wm_name, AtomEnum::STRING))
    }
}

/// X11-based focus tracker for Linux.
///
/// Connects to the X display on construction via `x11rb::RustConnection`
/// (pure Rust, no native library dependencies). If the connection fails
/// (e.g. `$DISPLAY` is unset on a Wayland-only session), both trait methods
/// gracefully degrade to `None` / `Duration::ZERO`.
pub struct LinuxFocusTracker {
    x11: Option<X11Connection>,
}

impl LinuxFocusTracker {
    pub fn new() -> Self {
        LinuxFocusTracker {
            x11: Self::connect().ok(),
        }
    }

    fn connect() -> Result<X11Connection, Box<dyn std::error::Error>> {
        let (conn, screen_num) = RustConnection::connect(None)?;
        let root = conn.setup().roots[screen_num].root;

        // pipeline all intern_atom requests before reading replies
        // to minimize client-server round-trips
        let c_active = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW")?;
        let c_pid = conn.intern_atom(false, b"_NET_WM_PID")?;
        let c_name = conn.intern_atom(false, b"_NET_WM_NAME")?;
        let c_wm = conn.intern_atom(false, b"WM_NAME")?;
        let c_utf8 = conn.intern_atom(false, b"UTF8_STRING")?;

        let atoms = Atoms {
            net_active_window: c_active.reply()?.atom,
            net_wm_pid: c_pid.reply()?.atom,
            net_wm_name: c_name.reply()?.atom,
            wm_name: c_wm.reply()?.atom,
            utf8_string: c_utf8.reply()?.atom,
        };

        Ok(X11Connection { conn, root, atoms })
    }
}

impl FocusTracker for LinuxFocusTracker {
    fn get_focused_window(&self) -> Option<FocusInfo> {
        let x11 = self.x11.as_ref()?;

        // read _NET_ACTIVE_WINDOW from the root window (EWMH standard)
        let window =
            x11.get_property_u32(x11.root, x11.atoms.net_active_window, AtomEnum::WINDOW)?;
        if window == 0 {
            return None;
        }

        // read _NET_WM_PID from the focused window
        let pid = x11.get_property_u32(window, x11.atoms.net_wm_pid, AtomEnum::CARDINAL)?;

        // resolve the executable path via /proc
        let exe_path = std::fs::read_link(format!("/proc/{}/exe", pid))
            .ok()
            .map(|p| p.to_string_lossy().into_owned());

        // extract process name from the full path (e.g. "/usr/bin/firefox" -> "firefox")
        let process_name = exe_path
            .as_ref()
            .and_then(|p| p.rsplit('/').next())
            .map(normalize_process_name)
            .unwrap_or_default();

        if process_name.is_empty() {
            return None;
        }

        let window_title = x11.get_window_title(window);

        Some(FocusInfo {
            process_name,
            exe_path,
            window_title,
        })
    }

    fn get_idle_duration(&self) -> Duration {
        let x11 = match self.x11.as_ref() {
            Some(x) => x,
            None => return Duration::ZERO,
        };

        // XScreenSaver extension provides ms_since_user_input
        let reply = match screensaver::query_info(&x11.conn, x11.root) {
            Ok(cookie) => match cookie.reply() {
                Ok(r) => r,
                Err(_) => return Duration::ZERO,
            },
            Err(_) => return Duration::ZERO,
        };

        Duration::from_millis(reply.ms_since_user_input as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_does_not_panic() {
        // on a machine without X11, inner will be None -- that's fine
        let _tracker = LinuxFocusTracker::new();
    }

    #[test]
    fn test_get_focused_window_returns_something_or_none() {
        let tracker = LinuxFocusTracker::new();
        let result = tracker.get_focused_window();
        if let Some(info) = result {
            assert!(!info.process_name.is_empty());
        }
    }

    #[test]
    fn test_get_idle_duration_does_not_panic() {
        let tracker = LinuxFocusTracker::new();
        let _duration = tracker.get_idle_duration();
    }
}
