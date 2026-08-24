//! macOS-only system glue: pasteboard inspection (things `arboard` cannot
//! see), frontmost-app tracking, app activation, Accessibility check and
//! synthesized ⌘V. Every function has a no-op twin for other platforms so
//! callers stay free of `cfg` noise.
//!
//! Threading: NSWorkspace is only touched on the main thread — the activation
//! observer writes into `AppState::frontmost`, and the watcher thread reads
//! that cache.

use clipon_core::SourceApp;

#[cfg(target_os = "macos")]
mod imp {
    use super::SourceApp;
    use crate::state::AppState;
    use objc2_app_kit::{
        NSApplicationActivationOptions, NSApplicationActivationPolicy, NSPasteboard,
        NSRunningApplication, NSWorkspace, NSWorkspaceApplicationKey,
        NSWorkspaceDidActivateApplicationNotification,
    };
    use objc2_foundation::{NSNotification, NSOperationQueue};
    use std::ptr::NonNull;
    use tauri::{AppHandle, Manager};

    pub fn pasteboard_change_count() -> i64 {
        NSPasteboard::generalPasteboard().changeCount() as i64
    }

    pub fn pasteboard_has_concealed_types() -> bool {
        let pb = NSPasteboard::generalPasteboard();
        match pb.types() {
            Some(types) => types
                .iter()
                .any(|t| clipon_core::is_concealed_type(&t.to_string())),
            None => false,
        }
    }

    fn describe(app: &NSRunningApplication) -> (SourceApp, i32) {
        let source = SourceApp {
            bundle_id: app.bundleIdentifier().map(|s| s.to_string()),
            name: app.localizedName().map(|s| s.to_string()),
        };
        (source, app.processIdentifier())
    }

    /// Direct read — main thread only (used once at startup to seed the cache).
    pub fn frontmost_app_now() -> Option<(SourceApp, i32)> {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        Some(describe(&app))
    }

    /// Keeps `AppState::frontmost` current. Must be called on the main thread.
    pub fn install_frontmost_observer(handle: AppHandle) {
        let block = block2::RcBlock::new(move |n: NonNull<NSNotification>| {
            let n = unsafe { n.as_ref() };
            let Some(info) = n.userInfo() else { return };
            let key = unsafe { NSWorkspaceApplicationKey };
            let Some(obj) = info.objectForKey(key) else { return };
            let Ok(app) = obj.downcast::<NSRunningApplication>() else { return };
            let st = handle.state::<AppState>();
            *st.frontmost.lock().unwrap() = Some(describe(&app));
        });
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        let token = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceDidActivateApplicationNotification),
                None,
                Some(&NSOperationQueue::mainQueue()),
                &block,
            )
        };
        // the observer lives as long as the process
        std::mem::forget(token);
    }

    /// Regular (Dock-visible) apps as (bundle id, name), sorted by name.
    /// Main thread only.
    pub fn running_apps() -> Vec<(String, String)> {
        let ws = NSWorkspace::sharedWorkspace();
        let mut apps: Vec<(String, String)> = ws.runningApplications()
            .iter()
            .filter(|a| a.activationPolicy() == NSApplicationActivationPolicy::Regular)
            .filter_map(|a| {
                let (s, _) = describe(&a);
                Some((s.bundle_id?, s.name?))
            })
            .collect();
        apps.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
        apps.dedup_by(|a, b| a.0 == b.0);
        apps
    }

    pub fn activate_pid(pid: i32) -> bool {
        match NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
            Some(app) => app.activateWithOptions(NSApplicationActivationOptions::empty()),
            None => false,
        }
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(
            options: core_foundation::dictionary::CFDictionaryRef,
        ) -> bool;
        static kAXTrustedCheckOptionPrompt: core_foundation::string::CFStringRef;
    }

    /// Accessibility permission; `prompt` shows the system dialog if missing.
    pub fn ax_trusted(prompt: bool) -> bool {
        use core_foundation::base::TCFType;
        use core_foundation::boolean::CFBoolean;
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::string::CFString;
        unsafe {
            let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
            let dict = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::from(prompt))]);
            AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef())
        }
    }

    /// Posts ⌘V to the frontmost app. Needs Accessibility permission.
    pub fn send_cmd_v() -> bool {
        use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
        const KEY_V: u16 = 9;
        let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else {
            return false;
        };
        let Ok(down) = CGEvent::new_keyboard_event(src.clone(), KEY_V, true) else {
            return false;
        };
        let Ok(up) = CGEvent::new_keyboard_event(src, KEY_V, false) else {
            return false;
        };
        down.set_flags(CGEventFlags::CGEventFlagCommand);
        up.set_flags(CGEventFlags::CGEventFlagCommand);
        down.post(CGEventTapLocation::HID);
        up.post(CGEventTapLocation::HID);
        true
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::SourceApp;
    use tauri::AppHandle;

    pub fn pasteboard_change_count() -> i64 {
        -1
    }
    pub fn pasteboard_has_concealed_types() -> bool {
        false
    }
    pub fn frontmost_app_now() -> Option<(SourceApp, i32)> {
        None
    }
    pub fn install_frontmost_observer(_handle: AppHandle) {}
    pub fn running_apps() -> Vec<(String, String)> {
        Vec::new()
    }
    pub fn activate_pid(_pid: i32) -> bool {
        false
    }
    pub fn ax_trusted(_prompt: bool) -> bool {
        false
    }
    pub fn send_cmd_v() -> bool {
        false
    }
}

pub use imp::*;

pub const IS_MACOS: bool = cfg!(target_os = "macos");
