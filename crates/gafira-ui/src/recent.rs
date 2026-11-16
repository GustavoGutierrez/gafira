//! «Recientes» of Inicio (spec 025, 03 §7.2): the port the shell reads and
//! records the recent projects through, and what Inicio shows of them.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::files::{self, FileAction};

/// A project the person opened or saved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    /// When it was last opened or saved, in seconds since the Unix epoch.
    pub opened_at: i64,
}

/// Where the recent projects live. The composition root supplies it (the
/// file of `gafira-platform`); the shell does no I/O of its own (02 §2.1).
/// A failure is the store's to log: the list is a convenience.
pub trait RecentStore {
    /// The list, newest first, without the projects whose file is gone
    /// (they leave the list).
    fn list(&self) -> Vec<RecentProject>;

    /// Put `path`, named `name`, in front of the list, opened now.
    fn record(&self, path: &Path, name: &str);
}

/// How many cards Inicio shows.
pub(crate) const SHOWN: usize = 4;

/// The projects Inicio shows: the first [`SHOWN`] of the list (S25-04).
pub(crate) fn shown(list: &[RecentProject]) -> &[RecentProject] {
    &list[..list.len().min(SHOWN)]
}

/// How long ago `opened_at` was at `now`; a clock that went back reads as
/// no time at all (S25-04).
pub(crate) fn age(opened_at: i64, now: i64) -> Duration {
    Duration::from_secs(u64::try_from(now.saturating_sub(opened_at)).unwrap_or(0))
}

/// Whether `action`, once done, puts the project in the list: a copy does
/// not change which project is open (S25-03).
pub(crate) fn records(action: FileAction) -> bool {
    action != FileAction::SaveCopy
}

/// Record the project at `path` under its file name without `.gaf`.
pub(crate) fn remember(store: &dyn RecentStore, path: &Path) {
    let name = files::project_name(Some(path)).unwrap_or_default();
    store.record(path, &name);
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gafira_ports::i18n::Locale;

    use super::*;
    use crate::text::Texts;

    /// A store in memory, as the composition root would hand one.
    #[derive(Default)]
    struct Memory(RefCell<Vec<RecentProject>>);

    impl RecentStore for Memory {
        fn list(&self) -> Vec<RecentProject> {
            self.0.borrow().clone()
        }

        fn record(&self, path: &Path, name: &str) {
            let mut list = self.0.borrow_mut();
            list.retain(|p| p.path != path);
            list.insert(
                0,
                RecentProject {
                    path: path.to_path_buf(),
                    name: name.to_owned(),
                    opened_at: 0,
                },
            );
        }
    }

    fn project(name: &str) -> RecentProject {
        RecentProject {
            path: PathBuf::from(format!("/w/{name}.gaf")),
            name: name.to_owned(),
            opened_at: 0,
        }
    }

    #[test]
    fn s25_03_open_and_save_record_but_a_copy_does_not() {
        assert!(records(FileAction::Open));
        assert!(records(FileAction::Save));
        assert!(records(FileAction::SaveAs));
        assert!(!records(FileAction::SaveCopy));
    }

    #[test]
    fn s25_03_the_name_is_the_file_name_without_gaf() {
        let store = Memory::default();
        remember(&store, Path::new("/w/Mi cartel.gaf"));
        remember(&store, Path::new("/w/otro.gaf"));
        let names: Vec<String> = store.list().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["otro", "Mi cartel"]);
    }

    #[test]
    fn s25_04_at_most_four_cards_in_list_order() {
        let list: Vec<RecentProject> = ["a", "b", "c", "d", "e", "f"]
            .into_iter()
            .map(project)
            .collect();
        let names: Vec<&str> = shown(&list).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c", "d"]);
        assert_eq!(shown(&list[..2]).len(), 2);
        assert!(shown(&[]).is_empty());
    }

    #[test]
    fn s25_04_the_relative_time_comes_from_the_catalog() {
        let es = Texts::new(Locale::Es);
        assert_eq!(es.relative(age(1_000, 1_000 + 7_200)), "hace 2 h");
        assert_eq!(es.relative(age(1_000, 900)), "ahora mismo", "clock behind");
        let en = Texts::new(Locale::En);
        assert_ne!(
            en.relative(age(0, 3 * 86_400)),
            es.relative(age(0, 3 * 86_400))
        );
    }
}
