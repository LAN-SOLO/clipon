//! Windows-only system glue, the counterpart of `macos.rs`: clipboard change
//! counter, the "do not record" clipboard formats that password managers set,
//! the app that put content on the clipboard, foreground-window handling and
//! a synthesized Ctrl+V for the quick picker.
//!
//! Plain Win32 calls (user32/kernel32 are linked by every Windows binary
//! anyway), declared here so no extra crate is needed.

use clipon_core::SourceApp;
use std::ffi::c_void;

type Hwnd = *mut c_void;
type Handle = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct KeybdInput {
    w_vk: u16,
    w_scan: u16,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

/// Only here so the union has the size of the real `INPUT` union
/// (MOUSEINPUT is its largest member).
#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
union InputUnion {
    mi: MouseInput,
    ki: KeybdInput,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Input {
    kind: u32,
    u: InputUnion,
}

const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const VK_CONTROL: u16 = 0x11;
const VK_V: u16 = 0x56;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const GW_OWNER: u32 = 4;

#[link(name = "user32")]
extern "system" {
    fn GetClipboardSequenceNumber() -> u32;
    fn RegisterClipboardFormatW(name: *const u16) -> u32;
    fn IsClipboardFormatAvailable(format: u32) -> i32;
    fn OpenClipboard(owner: Hwnd) -> i32;
    fn CloseClipboard() -> i32;
    fn GetClipboardData(format: u32) -> Handle;
    fn GetClipboardOwner() -> Hwnd;
    fn GetForegroundWindow() -> Hwnd;
    fn SetForegroundWindow(hwnd: Hwnd) -> i32;
    fn IsWindow(hwnd: Hwnd) -> i32;
    fn IsWindowVisible(hwnd: Hwnd) -> i32;
    fn GetWindow(hwnd: Hwnd, cmd: u32) -> Hwnd;
    fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
    fn GetWindowThreadProcessId(hwnd: Hwnd, pid: *mut u32) -> u32;
    fn EnumWindows(cb: extern "system" fn(Hwnd, isize) -> i32, lparam: isize) -> i32;
    fn SendInput(count: u32, inputs: *const Input, size: i32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GlobalLock(mem: Handle) -> *mut c_void;
    fn GlobalUnlock(mem: Handle) -> i32;
    fn GlobalSize(mem: Handle) -> usize;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> Handle;
    fn QueryFullProcessImageNameW(process: Handle, flags: u32, buf: *mut u16, size: *mut u32) -> i32;
    fn CloseHandle(h: Handle) -> i32;
    fn GetCurrentProcessId() -> u32;
    fn GetUserDefaultUILanguage() -> u16;
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn format_id(name: &str) -> u32 {
    let w = wide(name);
    unsafe { RegisterClipboardFormatW(w.as_ptr()) }
}

/// Changes with every clipboard write; `None` when the window station gives
/// no clipboard access (the watcher then simply polls every tick).
pub fn clipboard_sequence() -> Option<i64> {
    match unsafe { GetClipboardSequenceNumber() } {
        0 => None,
        n => Some(n as i64),
    }
}

/// True if the current clipboard content asks clipboard managers not to
/// record it. Conventions used by password managers on Windows (KeePass,
/// KeePassXC, 1Password, Bitwarden, …):
/// - `ExcludeClipboardContentFromMonitorProcessing` (any content)
/// - `Clipboard Viewer Ignore` (older convention)
/// - `CanIncludeInClipboardHistory` with a DWORD value of 0
pub fn clipboard_concealed() -> bool {
    for name in ["ExcludeClipboardContentFromMonitorProcessing", "Clipboard Viewer Ignore"] {
        let id = format_id(name);
        if id != 0 && unsafe { IsClipboardFormatAvailable(id) } != 0 {
            return true;
        }
    }
    let history = format_id("CanIncludeInClipboardHistory");
    if history == 0 || unsafe { IsClipboardFormatAvailable(history) } == 0 {
        return false;
    }
    // the format is present: read its DWORD. If the clipboard cannot be
    // opened right now, err on the side of privacy and skip this copy.
    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return true;
        }
        let mut concealed = true;
        let h = GetClipboardData(history);
        if !h.is_null() && GlobalSize(h) >= 4 {
            let p = GlobalLock(h) as *const u32;
            if !p.is_null() {
                concealed = std::ptr::read_unaligned(p) == 0;
                GlobalUnlock(h);
            }
        }
        CloseClipboard();
        concealed
    }
}

fn pid_of(hwnd: Hwnd) -> Option<u32> {
    if hwnd.is_null() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    (pid != 0).then_some(pid)
}

/// Executable of a process as (file name, name without extension), e.g.
/// ("KeePass.exe", "KeePass").
fn exe_of(pid: u32) -> Option<(String, String)> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return None;
        }
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut len) != 0;
        CloseHandle(h);
        if !ok {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        clipon_core::exe_names(&path)
    }
}

fn source_of_pid(pid: u32) -> Option<SourceApp> {
    if pid == unsafe { GetCurrentProcessId() } {
        return None;
    }
    let (file, stem) = exe_of(pid)?;
    Some(SourceApp {
        bundle_id: Some(file),
        name: Some(stem),
    })
}

/// The app that put the current content on the clipboard: the clipboard
/// owner window if there is one, otherwise the foreground window (apps that
/// open the clipboard without a window leave no owner).
pub fn clipboard_source() -> Option<SourceApp> {
    let owner = pid_of(unsafe { GetClipboardOwner() });
    let pid = owner.or_else(|| pid_of(unsafe { GetForegroundWindow() }))?;
    source_of_pid(pid)
}

extern "system" fn collect_window_pid(hwnd: Hwnd, lparam: isize) -> i32 {
    unsafe {
        let pids = &mut *(lparam as *mut Vec<u32>);
        if IsWindowVisible(hwnd) != 0
            && GetWindowTextLengthW(hwnd) > 0
            && GetWindow(hwnd, GW_OWNER).is_null()
        {
            if let Some(pid) = pid_of(hwnd) {
                if !pids.contains(&pid) {
                    pids.push(pid);
                }
            }
        }
    }
    1
}

/// Apps with a visible top-level window as (exe file name, name), sorted by
/// name — the Windows counterpart of the macOS "running apps" list.
pub fn running_apps() -> Vec<(String, String)> {
    let mut pids: Vec<u32> = Vec::new();
    unsafe { EnumWindows(collect_window_pid, &mut pids as *mut Vec<u32> as isize) };
    let mut apps: Vec<(String, String)> = pids
        .into_iter()
        .filter_map(|pid| {
            let s = source_of_pid(pid)?;
            Some((s.bundle_id?, s.name?))
        })
        .collect();
    apps.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    apps.dedup_by(|a, b| a.0.eq_ignore_ascii_case(&b.0));
    apps
}

/// Foreground window right now (as an integer, so it can live in a Mutex).
pub fn foreground_window() -> Option<isize> {
    let h = unsafe { GetForegroundWindow() };
    (!h.is_null()).then_some(h as isize)
}

/// Asks Windows to bring `hwnd` to the front. Call on the main (UI) thread
/// while clipon itself is still in front — Windows only lets the foreground
/// process hand over the focus.
pub fn request_foreground(hwnd: isize) -> bool {
    let h = hwnd as Hwnd;
    unsafe { IsWindow(h) != 0 && SetForegroundWindow(h) != 0 }
}

/// Waits (up to ~0.5 s) until `hwnd` really is the foreground window.
pub fn wait_foreground(hwnd: isize) -> bool {
    for _ in 0..20 {
        if unsafe { GetForegroundWindow() } as isize == hwnd {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    false
}

fn key(vk: u16, up: bool) -> Input {
    Input {
        kind: INPUT_KEYBOARD,
        u: InputUnion {
            ki: KeybdInput {
                w_vk: vk,
                w_scan: 0,
                dw_flags: if up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dw_extra_info: 0,
            },
        },
    }
}

/// Types Ctrl+V into the foreground window. No permission needed on Windows
/// (elevated target windows silently ignore it — UIPI).
pub fn send_ctrl_v() -> bool {
    let inputs = [
        key(VK_CONTROL, false),
        key(VK_V, false),
        key(VK_V, true),
        key(VK_CONTROL, true),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<Input>() as i32,
        )
    };
    sent == inputs.len() as u32
}

/// UI language of the Windows user account is German.
pub fn ui_language_is_german() -> bool {
    // primary language id 0x07 = LANG_GERMAN
    (unsafe { GetUserDefaultUILanguage() } & 0x3ff) == 0x07
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_struct_has_win32_size() {
        // sizeof(INPUT) is 40 on x64 and 28 on x86
        let expected = if cfg!(target_pointer_width = "64") { 40 } else { 28 };
        assert_eq!(std::mem::size_of::<Input>(), expected);
    }
}
