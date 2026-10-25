//! Watches the clipboard on a background thread and saves new text copies.

use crate::{emit_state, AppState};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const POLL: Duration = Duration::from_millis(400);

pub fn start(app: AppHandle) {
    thread::spawn(move || {
        let mut clipboard = loop {
            match arboard::Clipboard::new() {
                Ok(c) => break c,
                Err(_) => thread::sleep(Duration::from_secs(1)),
            }
        };
        #[cfg(windows)]
        let mut last_seq = 0u32;

        loop {
            thread::sleep(POLL);

            // On Windows, only open the clipboard when it actually changed,
            // and respect apps (like password managers) that ask not to be recorded.
            #[cfg(windows)]
            {
                let seq = win::sequence_number();
                if seq == last_seq {
                    continue;
                }
                if win::is_excluded() {
                    last_seq = seq;
                    continue;
                }
            }

            let text = match clipboard.get_text() {
                Ok(t) => t,
                // Another app has the clipboard open, try again on the next tick.
                Err(arboard::Error::ClipboardOccupied) => continue,
                // Not text (an image, files, empty).
                Err(_) => {
                    #[cfg(windows)]
                    {
                        last_seq = win::sequence_number();
                    }
                    continue;
                }
            };
            #[cfg(windows)]
            {
                last_seq = win::sequence_number();
            }

            let state = app.state::<AppState>();
            let changed = {
                let mut store = state.store.lock().unwrap();
                if store.last_seen == text {
                    false
                } else {
                    store.last_seen = text.clone();
                    !store.data.paused && store.add(text)
                }
            };
            if changed {
                emit_state(&app);
            }
        }
    });
}

#[cfg(windows)]
mod win {
    use std::sync::OnceLock;
    use windows_sys::Win32::System::DataExchange::{
        GetClipboardSequenceNumber, IsClipboardFormatAvailable, RegisterClipboardFormatW,
    };

    pub fn sequence_number() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    /// The standard marker apps set when a copy must not be saved by clipboard tools.
    pub fn is_excluded() -> bool {
        static FORMAT: OnceLock<u32> = OnceLock::new();
        let format = *FORMAT.get_or_init(|| {
            let name: Vec<u16> = "ExcludeClipboardContentFromMonitorProcessing\0".encode_utf16().collect();
            unsafe { RegisterClipboardFormatW(name.as_ptr()) }
        });
        format != 0 && unsafe { IsClipboardFormatAvailable(format) } != 0
    }
}
