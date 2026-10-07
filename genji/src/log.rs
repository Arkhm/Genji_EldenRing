//! Plain-text log file. Every line is flushed immediately so a crash still leaves the log behind.
//!
//! Location: `<project>/logs/genji.log` when the DLL sits in `<project>/build/`, otherwise next to the DLL.
//! The previous run's log is kept as `genji.prev.log`.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static LOG: Mutex<Option<File>> = Mutex::new(None);
static START: OnceLock<Instant> = OnceLock::new();

/// Opens the log file. `dll_path` is the full path of genji.dll.
pub fn init(dll_path: &Path) -> Option<PathBuf> {
    START.get_or_init(Instant::now);
    let dll_dir = dll_path.parent()?;
    let dir = match dll_dir.file_name().and_then(|n| n.to_str()) {
        Some(n) if n.eq_ignore_ascii_case("build") => dll_dir.parent()?.join("logs"),
        _ => dll_dir.to_path_buf(),
    };
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join("genji.log");
    if path.exists() {
        let _ = fs::rename(&path, dir.join("genji.prev.log"));
    }
    let file = File::create(&path).ok()?;
    *LOG.lock().unwrap_or_else(|e| e.into_inner()) = Some(file);
    Some(path)
}

/// Seconds since the DLL was loaded.
pub fn uptime() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

pub fn write(tag: &str, msg: &str) {
    let line = format!("[{:>9.3}] {:<6} {}\n", uptime(), tag, msg);
    if let Some(file) = LOG.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }
}

/// `glog!("TAG", "format {}", args)` writes one line to the log.
#[macro_export]
macro_rules! glog {
    ($tag:expr, $($arg:tt)*) => {
        $crate::log::write($tag, &format!($($arg)*))
    };
}
