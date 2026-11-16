//! The recent projects of Inicio as a JSON list in the data folder
//! (spec 025, 03 §7.2): `<data_dir>/recent.json`, written atomically.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How many projects the list keeps.
pub const MAX_RECENT: usize = 12;

/// A project the person opened or saved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    /// When it was last opened or saved, in seconds since the Unix epoch.
    pub opened_at: i64,
}

#[derive(Default, Serialize, Deserialize)]
struct RecentFile {
    projects: Vec<RecentProject>,
}

/// `list` with `project` in front: an entry for the same path leaves its
/// place, and the oldest beyond [`MAX_RECENT`] fall off (S25-01).
#[must_use]
pub fn push(list: Vec<RecentProject>, project: RecentProject) -> Vec<RecentProject> {
    let mut out = Vec::with_capacity(MAX_RECENT);
    out.push(project);
    out.extend(list);
    tidy(out)
}

/// `list` without repeated paths (the first one stays) and cut to
/// [`MAX_RECENT`].
fn tidy(list: Vec<RecentProject>) -> Vec<RecentProject> {
    let mut out: Vec<RecentProject> = Vec::with_capacity(MAX_RECENT);
    for project in list {
        if out.len() == MAX_RECENT {
            break;
        }
        if !out.iter().any(|kept| kept.path == project.path) {
            out.push(project);
        }
    }
    out
}

/// The recent projects in a JSON file.
pub struct RecentProjects {
    path: PathBuf,
}

impl RecentProjects {
    #[must_use]
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// The list, newest first. A missing or unreadable file reads as empty:
    /// the list is a convenience, never a reason to fail (S25-02).
    #[must_use]
    pub fn load(&self) -> Vec<RecentProject> {
        fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<RecentFile>(&bytes).ok())
            .map(|file| tidy(file.projects))
            .unwrap_or_default()
    }

    /// Put `project` in front of the list and write it.
    ///
    /// # Errors
    /// When the file cannot be written.
    pub fn record(&self, project: RecentProject) -> io::Result<()> {
        self.write(push(self.load(), project))
    }

    /// Take `path` out of the list.
    ///
    /// # Errors
    /// When the file cannot be written.
    pub fn remove(&self, path: &Path) -> io::Result<()> {
        let mut list = self.load();
        list.retain(|project| project.path != path);
        self.write(list)
    }

    /// The list without the projects whose file is gone; the file is
    /// rewritten when one went.
    ///
    /// # Errors
    /// When the pruned list cannot be written.
    pub fn existing(&self) -> io::Result<Vec<RecentProject>> {
        let list = self.load();
        let before = list.len();
        let kept: Vec<RecentProject> = list
            .into_iter()
            .filter(|project| project.path.try_exists().unwrap_or(false))
            .collect();
        if kept.len() != before {
            self.write(kept.clone())?;
        }
        Ok(kept)
    }

    fn write(&self, list: Vec<RecentProject>) -> io::Result<()> {
        let text =
            serde_json::to_vec_pretty(&RecentFile { projects: list }).map_err(io::Error::other)?;
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder)?;
        }
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(path: &str, opened_at: i64) -> RecentProject {
        RecentProject {
            path: PathBuf::from(path),
            name: path.trim_end_matches(".gaf").to_owned(),
            opened_at,
        }
    }

    fn paths(list: &[RecentProject]) -> Vec<&str> {
        list.iter()
            .map(|p| p.path.to_str().expect("utf-8"))
            .collect()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gafira-recent-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn s25_01_the_new_entry_goes_first() {
        let list = push(Vec::new(), project("a.gaf", 1));
        let list = push(list, project("b.gaf", 2));
        assert_eq!(paths(&list), ["b.gaf", "a.gaf"]);
    }

    #[test]
    fn s25_01_a_repeated_path_moves_to_the_front_once() {
        let list = [
            project("a.gaf", 1),
            project("b.gaf", 2),
            project("c.gaf", 3),
        ]
        .into_iter()
        .fold(Vec::new(), push);
        let list = push(list, project("a.gaf", 4));
        assert_eq!(paths(&list), ["a.gaf", "c.gaf", "b.gaf"]);
        assert_eq!(list[0].opened_at, 4, "the newer entry wins");
    }

    #[test]
    fn s25_01_never_more_than_twelve_and_the_oldest_falls_off() {
        let list = (0..20)
            .map(|i| project(&format!("{i}.gaf"), i))
            .fold(Vec::new(), push);
        assert_eq!(list.len(), MAX_RECENT);
        assert_eq!(list[0].path, PathBuf::from("19.gaf"));
        assert_eq!(list[MAX_RECENT - 1].path, PathBuf::from("8.gaf"));
    }

    #[test]
    fn s25_02_missing_and_garbage_read_as_empty() {
        let dir = scratch("garbage");
        let store = RecentProjects::at(dir.join("recent.json"));
        assert!(store.load().is_empty(), "missing");
        fs::create_dir_all(&dir).expect("folder");
        fs::write(dir.join("recent.json"), b"{nope").expect("garbage");
        assert!(store.load().is_empty(), "garbage");
        fs::write(dir.join("recent.json"), b"[1, 2]").expect("wrong shape");
        assert!(store.load().is_empty(), "wrong shape");
        fs::remove_dir_all(dir).expect("clean up");
    }

    #[test]
    fn s25_02_recorded_reads_back_and_removed_goes() {
        let dir = scratch("roundtrip");
        let store = RecentProjects::at(dir.join("nested/recent.json"));
        store.record(project("a.gaf", 1)).expect("records");
        store.record(project("b.gaf", 2)).expect("records");
        store.record(project("a.gaf", 3)).expect("records");
        assert_eq!(store.load(), vec![project("a.gaf", 3), project("b.gaf", 2)]);

        store.remove(Path::new("a.gaf")).expect("removes");
        assert_eq!(store.load(), vec![project("b.gaf", 2)]);
        assert!(
            !dir.join("nested/recent.json.tmp").exists(),
            "no temporary file stays"
        );
        fs::remove_dir_all(dir).expect("clean up");
    }

    #[test]
    fn s25_02_a_file_read_by_hand_keeps_the_rules() {
        let dir = scratch("by-hand");
        fs::create_dir_all(&dir).expect("folder");
        let many: Vec<RecentProject> = (0..15)
            .map(|i| project(&format!("{}.gaf", i % 14), 100 - i))
            .collect();
        let text = serde_json::to_vec(&RecentFile { projects: many }).expect("json");
        fs::write(dir.join("recent.json"), text).expect("written by hand");
        let list = RecentProjects::at(dir.join("recent.json")).load();
        assert_eq!(list.len(), MAX_RECENT);
        assert_eq!(list[0].path, PathBuf::from("0.gaf"));
        assert_eq!(list[0].opened_at, 100, "the first of a repeated path wins");
        fs::remove_dir_all(dir).expect("clean up");
    }

    #[test]
    fn s25_02_gone_files_are_pruned_and_the_list_rewritten() {
        let dir = scratch("prune");
        fs::create_dir_all(&dir).expect("folder");
        let kept = dir.join("kept.gaf");
        fs::write(&kept, b"x").expect("a project");
        let gone = dir.join("gone.gaf");
        let store = RecentProjects::at(dir.join("recent.json"));
        let entry = |path: &Path, at| RecentProject {
            path: path.to_path_buf(),
            name: "p".to_owned(),
            opened_at: at,
        };
        store.record(entry(&kept, 1)).expect("records");
        store.record(entry(&gone, 2)).expect("records");

        let list = store.existing().expect("prunes");
        assert_eq!(list, vec![entry(&kept, 1)]);
        assert_eq!(store.load(), vec![entry(&kept, 1)], "rewritten");
        fs::remove_dir_all(dir).expect("clean up");
    }
}
