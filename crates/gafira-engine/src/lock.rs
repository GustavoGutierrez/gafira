//! The file lock beside an open project (02 §7.2 «Bloqueos de archivo»,
//! spec 028): `<file>.lock`, three lines `pid`, `host` and the wall time the
//! session started, written atomically through `ports::FileSystem`.
//!
//! A lock says which instance has the file open. The engine reads it before
//! it takes a file: a live other process, or any process of another machine,
//! makes the session read-only (ARQ-55); a dead or unreadable one is taken
//! over. There is no OS lock on the file (the `fs4` lock of §7.2 is out of
//! spec 028's scope).

use std::path::{Path, PathBuf};

use gafira_ports::common::UnixTime;
use gafira_ports::system::ProcessId;

/// Who holds a project's lock file (spec 028).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileLock {
    /// The holding process.
    pub pid: ProcessId,
    /// The machine it runs on; empty when its platform gave no name.
    pub host: String,
    /// When its session took the file.
    pub started: UnixTime,
}

/// `<file>.lock`, in the same folder.
pub(crate) fn lock_path(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(".lock");
    PathBuf::from(name)
}

/// `"<pid>\n<host>\n<started>"`.
pub(crate) fn lock_text(lock: &FileLock) -> String {
    format!("{}\n{}\n{}", lock.pid.0, lock.host, lock.started.0)
}

/// The lock `text` describes; `None` when it is not one.
pub(crate) fn parse_lock(text: &str) -> Option<FileLock> {
    let mut lines = text.split('\n');
    let pid = ProcessId(lines.next()?.trim().parse().ok()?);
    let host = lines.next()?.trim().to_owned();
    let started = UnixTime(lines.next()?.trim().parse().ok()?);
    if lines.any(|line| !line.trim().is_empty()) {
        return None;
    }
    Some(FileLock { pid, host, started })
}

#[cfg(test)]
mod tests {
    use super::*;

    // S28-04
    #[test]
    fn s28_04_a_lock_round_trips_and_anything_else_is_none() {
        let lock = FileLock {
            pid: ProcessId(42),
            host: "studio".to_owned(),
            started: UnixTime(1_700_000_000),
        };
        assert_eq!(lock_text(&lock), "42\nstudio\n1700000000");
        assert_eq!(parse_lock(&lock_text(&lock)), Some(lock));
        let nameless = "1\n\n5";
        assert_eq!(parse_lock(nameless).map(|l| l.host), Some(String::new()));
        for bad in ["", "garbage", "x\nh\n5", "1\nh", "1\nh\nx", "1\nh\n5\nmore"] {
            assert_eq!(parse_lock(bad), None, "{bad:?}");
        }
        assert_eq!(
            lock_path(Path::new("/work/a.gaf")),
            PathBuf::from("/work/a.gaf.lock")
        );
    }
}
