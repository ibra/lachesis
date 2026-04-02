use super::{normalize_process_name, FocusInfo, FocusTracker};
use std::time::Duration;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
};

pub struct WindowsFocusTracker;

impl WindowsFocusTracker {
    pub fn new() -> Self {
        WindowsFocusTracker
    }
}

impl FocusTracker for WindowsFocusTracker {
    fn get_focused_window(&self) -> Option<FocusInfo> {
        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }

            // get the process id from the window handle
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return None;
            }

            // open the process to query its executable path
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

            let mut buf = [0u16; MAX_PATH as usize];
            let mut len = buf.len() as u32;

            let exe_path = if QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                PWSTR(buf.as_mut_ptr()),
                &mut len,
            )
            .is_ok()
            {
                Some(String::from_utf16_lossy(&buf[..len as usize]))
            } else {
                None
            };

            let _ = CloseHandle(handle);

            // extract the process name from the path
            let process_name = exe_path
                .as_ref()
                .and_then(|p| p.rsplit('\\').next())
                .map(normalize_process_name)
                .unwrap_or_default();

            if process_name.is_empty() {
                return None;
            }

            // get the window title
            let title_len = GetWindowTextLengthW(hwnd);
            let window_title = if title_len > 0 {
                let mut title_buf = vec![0u16; (title_len + 1) as usize];
                let actual_len = GetWindowTextW(hwnd, &mut title_buf);
                if actual_len > 0 {
                    Some(String::from_utf16_lossy(&title_buf[..actual_len as usize]))
                } else {
                    None
                }
            } else {
                None
            };

            Some(FocusInfo {
                process_name,
                exe_path,
                window_title,
            })
        }
    }

    fn get_idle_duration(&self) -> Duration {
        unsafe {
            let mut info = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };

            if GetLastInputInfo(&mut info).as_bool() {
                let now = GetTickCount();
                let idle_ms = now.wrapping_sub(info.dwTime);
                Duration::from_millis(idle_ms as u64)
            } else {
                Duration::ZERO
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_focused_window_returns_something() {
        let tracker = WindowsFocusTracker::new();
        // on a machine with a desktop session, this should return Some
        // in CI without a desktop, it may return None -- that's fine
        let result = tracker.get_focused_window();
        if let Some(info) = result {
            assert!(!info.process_name.is_empty());
        }
    }

    #[test]
    fn test_get_idle_duration_does_not_panic() {
        let tracker = WindowsFocusTracker::new();
        let _duration = tracker.get_idle_duration();
    }
}
