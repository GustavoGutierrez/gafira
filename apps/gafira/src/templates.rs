//! The templates the app carries inside (spec 091): written out once to a
//! folder of the data directory, so the engine opens them as any `.gaf`,
//! and listed from there with their `template` (D-81). Each folder under
//! `assets/templates/` groups templates; its name is one more word to find
//! them by.

use std::io::BufReader;
use std::path::{Path, PathBuf};

use gafira_format::{Limits, read_template_info, read_thumbnail};
use gafira_ports::repository::RepositoryError;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

/// The bundled templates, under `dir`.
pub(crate) struct Templates {
    pub(crate) dir: PathBuf,
}

impl Templates {
    /// Write every bundled template under `dir` that is missing or differs,
    /// and remove the `.gaf` files no build carries any more.
    fn write_out(&self) {
        for (relative, bytes) in BUNDLED {
            let path = self.dir.join(relative);
            if std::fs::read(&path).is_ok_and(|old| old == *bytes) {
                continue;
            }
            let written = path
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(&path, bytes));
            if let Err(error) = written {
                eprintln!("template {}: {error}", path.display());
            }
        }
        let mut present = Vec::new();
        gafs(&self.dir, &mut present);
        for path in present {
            let known = path
                .strip_prefix(&self.dir)
                .ok()
                .map(relative)
                .is_some_and(|r| BUNDLED.iter().any(|(name, _)| *name == r));
            if !known && let Err(error) = std::fs::remove_file(&path) {
                eprintln!("template {}: {error}", path.display());
            }
        }
    }
}

/// `path` with `/` between its parts.
fn relative(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every `.gaf` under `dir`.
fn gafs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            gafs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "gaf") {
            out.push(path);
        }
    }
}

impl gafira_ui::TemplateStore for Templates {
    fn list(&self) -> Vec<gafira_ui::TemplateListing> {
        self.write_out();
        let mut paths = Vec::new();
        gafs(&self.dir, &mut paths);
        paths.sort();
        paths
            .into_iter()
            .filter_map(|path| {
                let read = std::fs::File::open(&path)
                    .map_err(RepositoryError::Io)
                    .and_then(|file| read_template_info(BufReader::new(file), Limits::default()));
                match read {
                    Ok(Some(info)) => {
                        let mut meta = info.meta;
                        // The folders it is in are words to find it by.
                        if let Ok(inside) = path.strip_prefix(&self.dir) {
                            let folders = inside.parent().map(relative).unwrap_or_default();
                            meta.tags.extend(
                                folders
                                    .split('/')
                                    .filter(|f| !f.is_empty())
                                    .map(str::to_owned),
                            );
                        }
                        Some(gafira_ui::TemplateListing { path, meta })
                    }
                    Ok(None) => None,
                    Err(error) => {
                        eprintln!("template {}: {error}", path.display());
                        None
                    }
                }
            })
            .collect()
    }

    fn thumbnail(&self, path: &Path) -> Option<gafira_ui::RecentThumbnail> {
        let read = std::fs::File::open(path)
            .map_err(RepositoryError::Io)
            .and_then(|file| read_thumbnail(BufReader::new(file), Limits::default()));
        match read {
            Ok(thumb) => thumb.map(|thumb| gafira_ui::RecentThumbnail {
                width: thumb.width,
                height: thumb.height,
                rgba: thumb.rgba,
            }),
            Err(error) => {
                eprintln!("template thumbnail {}: {error}", path.display());
                None
            }
        }
    }
}
