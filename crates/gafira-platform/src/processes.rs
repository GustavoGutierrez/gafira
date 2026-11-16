//! Who this process is and whether another one runs (spec 028), with `std`
//! only.
//!
//! Linux answers from `/proc/<pid>`. Windows and macOS have no `std` call
//! for a process id, so there every instance holds a beacon file
//! `<folder>/<pid>` under an OS lock (`File::try_lock`) for as long as it
//! runs: a beacon another process holds is alive, a free or missing one is
//! not, because the OS drops the lock when its process dies. When the check
//! itself fails, the answer is "alive", so nothing of a live instance is
//! taken over.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use gafira_ports::system::{ProcessId, ProcessProbe};

/// The real [`ProcessProbe`] of the desktop app.
pub struct SystemProcesses {
    pid: ProcessId,
    host: String,
    beacons: PathBuf,
    /// This process's beacon, held locked until drop; `None` on Linux or
    /// when it could not be created (other instances then see this one as
    /// dead, the behaviour before spec 028).
    beacon: Option<File>,
}

impl SystemProcesses {
    /// The probe of this process; outside Linux it creates and locks its
    /// beacon in `beacons` (usually `<data_dir>/instances`).
    #[must_use]
    pub fn new(beacons: impl Into<PathBuf>) -> Self {
        let beacons = beacons.into();
        let pid = ProcessId(std::process::id());
        let beacon = if cfg!(target_os = "linux") {
            None
        } else {
            hold_beacon(&beacons, pid).ok()
        };
        Self {
            pid,
            host: host_name(),
            beacons,
            beacon,
        }
    }
}

impl Drop for SystemProcesses {
    fn drop(&mut self) {
        if self.beacon.take().is_some() {
            let _ = std::fs::remove_file(beacon_path(&self.beacons, self.pid));
        }
    }
}

impl ProcessProbe for SystemProcesses {
    fn current(&self) -> ProcessId {
        self.pid
    }

    fn host(&self) -> String {
        self.host.clone()
    }

    fn is_alive(&self, pid: ProcessId) -> bool {
        if pid == self.pid {
            true
        } else if cfg!(target_os = "linux") {
            proc_alive(Path::new("/proc"), pid)
        } else {
            beacon_alive(&self.beacons, pid)
        }
    }
}

fn beacon_path(beacons: &Path, pid: ProcessId) -> PathBuf {
    beacons.join(pid.0.to_string())
}

/// Create `<beacons>/<pid>` and lock it for this process.
fn hold_beacon(beacons: &Path, pid: ProcessId) -> io::Result<File> {
    std::fs::create_dir_all(beacons)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(beacon_path(beacons, pid))?;
    file.try_lock().map_err(io::Error::from)?;
    Ok(file)
}

/// Linux: a process runs while `/proc/<pid>` exists. An error other than
/// "not found" counts as alive.
fn proc_alive(proc: &Path, pid: ProcessId) -> bool {
    match std::fs::metadata(proc.join(pid.0.to_string())) {
        Ok(_) => true,
        Err(error) => error.kind() != io::ErrorKind::NotFound,
    }
}

/// Windows and macOS: alive while another process holds the beacon's lock.
fn beacon_alive(beacons: &Path, pid: ProcessId) -> bool {
    let file = match OpenOptions::new()
        .write(true)
        .open(beacon_path(beacons, pid))
    {
        Ok(file) => file,
        Err(error) => return error.kind() != io::ErrorKind::NotFound,
    };
    // Free: its process died, and the lock goes with `file`. Held, or the
    // check failed: alive.
    file.try_lock().is_err()
}

/// The machine's name: `/proc/sys/kernel/hostname`, then `COMPUTERNAME`
/// (Windows), then `HOSTNAME`; empty when none is there.
fn host_name() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            ["COMPUTERNAME", "HOSTNAME"]
                .iter()
                .find_map(|var| std::env::var(var).ok().filter(|v| !v.is_empty()))
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gafira-platform-processes-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    // S28-07
    #[test]
    fn s28_07_this_process_is_current_and_alive() {
        let dir = scratch("current");
        let probe = SystemProcesses::new(&dir);
        assert_eq!(probe.current(), ProcessId(std::process::id()));
        assert!(probe.is_alive(probe.current()));
        drop(probe);
        assert!(!beacon_path(&dir, ProcessId(std::process::id())).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // S28-07
    #[test]
    fn s28_07_on_linux_a_pid_without_a_proc_entry_is_dead() {
        let dir = scratch("proc");
        std::fs::create_dir_all(dir.join("42")).expect("a fake entry");
        assert!(proc_alive(&dir, ProcessId(42)));
        assert!(!proc_alive(&dir, ProcessId(43)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // S28-07
    #[test]
    fn s28_07_a_held_beacon_is_alive_and_a_free_or_missing_one_is_not() {
        let dir = scratch("beacon");
        let other = ProcessId(7);
        assert!(!beacon_alive(&dir, other), "missing");
        let held = hold_beacon(&dir, other).expect("a beacon");
        // `try_lock` on a second handle of the same file reports it held,
        // as it does for another process.
        assert!(beacon_alive(&dir, other), "held");
        drop(held);
        assert!(!beacon_alive(&dir, other), "free after its holder went");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // S28-07
    #[test]
    fn s28_07_the_host_name_is_stable() {
        assert_eq!(host_name(), host_name());
    }
}
