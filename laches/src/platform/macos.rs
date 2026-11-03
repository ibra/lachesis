use super::{normalize_process_name, FocusInfo, FocusTracker};
use objc2_app_kit::NSWorkspace;
use objc2_application_services::{AXError, AXIsProcessTrustedWithOptions, AXUIElement};
use objc2_core_foundation::{CFBoolean, CFDictionary, CFRetained, CFString, CFType};
use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};
use objc2_foundation::{NSDate, NSRunLoop};
use std::ptr::{self, NonNull};
use std::time::Duration;

/// `kCGAnyInputEventType` is `~0` in the C headers.
const ANY_INPUT_EVENT_TYPE: CGEventType = CGEventType(u32::MAX);

/// Upper bound on how long a single AX query may block on an unresponsive app.
const AX_MESSAGING_TIMEOUT_SECS: f32 = 0.25;

/// macOS focus tracker.
///
/// - frontmost app: `NSWorkspace.frontmostApplication` (no permission needed)
/// - window title: Accessibility API (`AXFocusedWindow` -> `AXTitle`); requires
///   Accessibility permission, degrades to `None` without it
/// - idle time: `CGEventSourceSecondsSinceLastEventType` (no permission needed)
pub struct MacOsFocusTracker;

impl MacOsFocusTracker {
    pub fn new() -> Self {
        MacOsFocusTracker
    }
}

/// Returns whether this process is a trusted accessibility client.
/// With `prompt`, macOS shows its "grant access" dialog if not yet trusted
/// (asynchronously; the return value reflects the current state).
pub fn accessibility_trusted(prompt: bool) -> bool {
    // kAXTrustedCheckOptionPrompt's value is the string "AXTrustedCheckOptionPrompt"
    let key = CFString::from_static_str("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::new(prompt);
    let options = CFDictionary::<CFString, CFBoolean>::from_slices(&[&*key], &[value]);
    unsafe { AXIsProcessTrustedWithOptions(Some(options.as_opaque())) }
}

/// Process pending run loop sources without blocking.
///
/// NSWorkspace updates `frontmostApplication` from notifications delivered on
/// the run loop. The daemon never runs one, so without this the value goes stale.
fn drain_run_loop() {
    NSRunLoop::currentRunLoop().runUntilDate(&NSDate::now());
}

/// Copy an AX attribute and downcast it to `T`.
fn ax_attribute<T: objc2_core_foundation::ConcreteType>(
    element: &AXUIElement,
    attribute: &str,
) -> Option<CFRetained<T>> {
    let attribute = CFString::from_str(attribute);
    let mut value: *const CFType = ptr::null();
    let err = unsafe { element.copy_attribute_value(&attribute, NonNull::from(&mut value)) };
    if err != AXError::Success {
        return None;
    }
    let value = NonNull::new(value as *mut CFType)?;
    // the Copy rule: we own the returned reference
    let value = unsafe { CFRetained::from_raw(value) };
    value.downcast::<T>().ok()
}

/// Read the focused window's title through the Accessibility API.
/// Returns None if permission isn't granted, the app has no focused window,
/// or the title is empty.
fn get_window_title(pid: i32) -> Option<String> {
    let app = unsafe { AXUIElement::new_application(pid) };
    unsafe { app.set_messaging_timeout(AX_MESSAGING_TIMEOUT_SECS) };

    let window = ax_attribute::<AXUIElement>(&app, "AXFocusedWindow")?;
    let title = ax_attribute::<CFString>(&window, "AXTitle")?.to_string();

    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

impl FocusTracker for MacOsFocusTracker {
    fn get_focused_window(&self) -> Option<FocusInfo> {
        drain_run_loop();

        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let pid = app.processIdentifier();
        if pid <= 0 {
            return None;
        }

        let exe_path = app
            .executableURL()
            .and_then(|url| url.path())
            .map(|p| p.to_string());

        // executable filename, e.g. ".../Safari.app/Contents/MacOS/Safari" -> "Safari"
        let process_name = exe_path
            .as_ref()
            .and_then(|p| p.rsplit('/').next())
            .map(normalize_process_name)
            .unwrap_or_default();

        if process_name.is_empty() {
            return None;
        }

        let window_title = get_window_title(pid);

        Some(FocusInfo {
            process_name,
            exe_path,
            window_title,
        })
    }

    fn get_idle_duration(&self) -> Duration {
        let secs = CGEventSource::seconds_since_last_event_type(
            CGEventSourceStateID::CombinedSessionState,
            ANY_INPUT_EVENT_TYPE,
        );
        if secs.is_finite() && secs > 0.0 {
            Duration::from_secs_f64(secs)
        } else {
            Duration::ZERO
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_does_not_panic() {
        let _tracker = MacOsFocusTracker::new();
    }

    #[test]
    fn test_get_focused_window_returns_something_or_none() {
        // in CI without a GUI session this may return None
        let tracker = MacOsFocusTracker::new();
        if let Some(info) = tracker.get_focused_window() {
            assert!(!info.process_name.is_empty());
            assert!(!info.process_name.contains('/'));
        }
    }

    #[test]
    fn test_get_idle_duration_does_not_panic() {
        let tracker = MacOsFocusTracker::new();
        let _duration = tracker.get_idle_duration();
    }

    #[test]
    fn test_accessibility_trusted_without_prompt_does_not_panic() {
        let _trusted = accessibility_trusted(false);
    }
}
