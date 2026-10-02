use std::ffi::{CString, c_char, c_int};
use std::io::Write;

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

const INFO: c_int = 4;
const ERROR: c_int = 6;

pub(crate) fn write(priority: c_int, text: &str) {
    let tag = c"reactive";
    let text = CString::new(text.replace('\0', "\\0")).unwrap_or_default();
    unsafe { __android_log_write(priority, tag.as_ptr(), text.as_ptr()) };
}

/// Sends panic messages to logcat; stderr is discarded on Android.
pub(crate) fn install_panic_hook() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            write(ERROR, &format!("{info}"));
            default(info);
        }));
    });
}

/// A `tracing` writer that sends each formatted event to logcat (tag `reactive`).
///
/// ```ignore
/// tracing_subscriber::fmt().with_writer(ui_core::android::LogcatWriter::default).init();
/// ```
#[derive(Default)]
pub struct LogcatWriter(Vec<u8>);

impl Write for LogcatWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if !self.0.is_empty() {
            write(INFO, String::from_utf8_lossy(&self.0).trim_end());
            self.0.clear();
        }
        Ok(())
    }
}

impl Drop for LogcatWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
