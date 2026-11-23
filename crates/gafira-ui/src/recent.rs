//! «Recientes» of Inicio (spec 025, 03 §7.2): the port the shell reads and
//! records the recent projects through, and what Inicio shows of them,
//! with the thumbnail of each project's first page (spec 033).

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
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

/// The thumbnail of a project's first page, as the file stores it:
/// straight 8-bit sRGB RGBA, row by row (spec 033).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RecentThumbnail {
    pub width: u32,
    pub height: u32,
    /// `width × height × 4` bytes.
    pub rgba: Vec<u8>,
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

    /// The thumbnail of the first page of the project at `path`, read from
    /// its file without opening it (03 §7.2, spec 033); `None` when it has
    /// none or cannot be read. By default no project has one.
    fn thumbnail(&self, path: &Path) -> Option<RecentThumbnail> {
        let _ = path;
        None
    }
}

/// What Inicio keeps of each thumbnail it shows (a texture, in the
/// shell), by project path and by a hash of the pixels: the same picture
/// is made once, a new one replaces it (S33-07).
pub(crate) struct ThumbCache<T> {
    entries: BTreeMap<PathBuf, (u64, T)>,
}

impl<T> Default for ThumbCache<T> {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl<T> ThumbCache<T> {
    /// What `thumb` of the project at `path` became: the kept one when its
    /// pixels did not change, otherwise `make(thumb)`, kept from now on.
    pub(crate) fn get(
        &mut self,
        path: &Path,
        thumb: &RecentThumbnail,
        make: impl FnOnce(&RecentThumbnail) -> T,
    ) -> &T {
        let key = key(thumb);
        let fresh = self.entries.get(path).is_none_or(|(kept, _)| *kept != key);
        if fresh {
            self.entries.insert(path.to_path_buf(), (key, make(thumb)));
        }
        &self.entries[path].1
    }

    /// Forget every project but `paths`.
    pub(crate) fn keep_only<'a>(&mut self, paths: impl IntoIterator<Item = &'a Path>) {
        let keep: Vec<&Path> = paths.into_iter().collect();
        self.entries
            .retain(|path, _| keep.contains(&path.as_path()));
    }
}

/// A hash of a thumbnail's size and pixels.
fn key(thumb: &RecentThumbnail) -> u64 {
    let mut hasher = DefaultHasher::new();
    thumb.hash(&mut hasher);
    hasher.finish()
}

/// The size of a `width` × `height` picture drawn whole and as large as it
/// fits in `space` (logical px), keeping its proportion (S33-07).
pub(crate) fn fit(width: u32, height: u32, space: (f32, f32)) -> (f32, f32) {
    if width == 0 || height == 0 {
        return (0.0, 0.0);
    }
    let (w, h) = (width as f32, height as f32);
    let scale = (space.0 / w).min(space.1 / h);
    (w * scale, h * scale)
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

    fn picture(value: u8) -> RecentThumbnail {
        RecentThumbnail {
            width: 1,
            height: 1,
            rgba: vec![value; 4],
        }
    }

    // S33-07: a store that keeps no thumbnails has none.
    #[test]
    fn s33_07_a_store_has_no_thumbnail_by_default() {
        assert_eq!(Memory::default().thumbnail(Path::new("/w/a.gaf")), None);
    }

    // S33-07: the same picture of the same project is made once; new pixels
    // make it again.
    #[test]
    fn s33_07_the_cache_makes_each_picture_once() {
        let mut cache = ThumbCache::default();
        let mut made = 0;
        let a = Path::new("/w/a.gaf");
        let b = Path::new("/w/b.gaf");
        let mut get = |cache: &mut ThumbCache<u8>, path: &Path, value: u8| {
            *cache.get(path, &picture(value), |thumb| {
                made += 1;
                thumb.rgba[0]
            })
        };
        assert_eq!(get(&mut cache, a, 1), 1);
        assert_eq!(get(&mut cache, a, 1), 1);
        assert_eq!(get(&mut cache, b, 1), 1, "another project");
        assert_eq!(get(&mut cache, a, 2), 2, "the project was saved again");
        assert_eq!(made, 3);
    }

    #[test]
    fn s33_07_the_cache_forgets_the_projects_no_longer_shown() {
        let mut cache = ThumbCache::default();
        let mut made = 0;
        for path in ["/w/a.gaf", "/w/b.gaf"] {
            cache.get(Path::new(path), &picture(1), |_| made += 1);
        }
        cache.keep_only([Path::new("/w/b.gaf")]);
        cache.get(Path::new("/w/b.gaf"), &picture(1), |_| made += 1);
        assert_eq!(made, 2, "b was kept");
        cache.get(Path::new("/w/a.gaf"), &picture(1), |_| made += 1);
        assert_eq!(made, 3, "a was forgotten");
    }

    // S33-07: the picture fills the placeholder on one side, in proportion.
    #[test]
    fn s33_07_the_thumbnail_fits_the_placeholder_in_proportion() {
        assert_eq!(fit(512, 256, (176.0, 110.0)), (176.0, 88.0));
        assert_eq!(fit(288, 512, (176.0, 110.0)), (61.875, 110.0));
        assert_eq!(fit(16, 10, (176.0, 110.0)), (176.0, 110.0));
        assert_eq!(fit(0, 10, (176.0, 110.0)), (0.0, 0.0));
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
