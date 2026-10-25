//! Returning focus to the previous app and simulating Ctrl+V / Cmd+V.

use std::thread;
use std::time::Duration;

#[cfg(windows)]
mod imp {
    use super::*;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

    const VK_V: VIRTUAL_KEY = 0x56;
    static PREVIOUS: AtomicIsize = AtomicIsize::new(0);

    pub fn remember_foreground() {
        let hwnd = unsafe { GetForegroundWindow() };
        PREVIOUS.store(hwnd as isize, Ordering::SeqCst);
    }

    pub fn has_permission() -> bool {
        true
    }

    fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    pub fn paste_into_previous() {
        let hwnd = PREVIOUS.load(Ordering::SeqCst);
        if hwnd != 0 {
            unsafe { SetForegroundWindow(hwnd as _) };
        }
        thread::sleep(Duration::from_millis(80));
        let inputs = [key(VK_CONTROL, false), key(VK_V, false), key(VK_V, true), key(VK_CONTROL, true)];
        unsafe {
            SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32);
        }
    }

    pub fn request_permission() {}
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::CFString;
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }

    const KEY_V: CGKeyCode = 9;

    /// On macOS the app is hidden after a pick, which hands focus back to the previous app.
    pub fn remember_foreground() {}

    pub fn has_permission() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn paste_into_previous() {
        thread::sleep(Duration::from_millis(150));
        let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else { return };
        for down in [true, false] {
            if let Ok(event) = CGEvent::new_keyboard_event(source.clone(), KEY_V, down) {
                event.set_flags(CGEventFlags::CGEventFlagCommand);
                event.post(CGEventTapLocation::HID);
            }
        }
    }

    /// Adds ClipStash to the Accessibility list (with the system prompt) and opens that settings page.
    pub fn request_permission() {
        let key = CFString::new("AXTrustedCheckOptionPrompt");
        let options = CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
        unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    pub fn remember_foreground() {}
    pub fn has_permission() -> bool {
        true
    }
    pub fn paste_into_previous() {}
    pub fn request_permission() {}
}

pub use imp::*;
