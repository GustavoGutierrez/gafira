//! The files of the models in their folder, and what the person chose and
//! what was measured (S84-05, S84-06).

use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::vision::VisionError;
use serde::{Deserialize, Serialize};

use crate::catalog::{Library, Model};
use crate::fetch::Fetch;

/// What the person chose and what was measured.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Settings {
    /// The model of each task, by its name.
    pub chosen: BTreeMap<String, String>,
    /// The ms of each runtime with each model.
    pub measured: BTreeMap<String, BTreeMap<String, u32>>,
}

pub(crate) struct Store {
    folder: PathBuf,
}

fn io(error: impl std::fmt::Display) -> VisionError {
    VisionError::Io(error.to_string())
}

impl Store {
    pub fn at(folder: PathBuf) -> Self {
        Self { folder }
    }

    fn model_file(&self, model: &Model) -> PathBuf {
        self.folder.join(format!("{}.onnx", model.id))
    }

    /// The folder of a library, by its version: another one is another.
    fn library_folder(&self, library: &Library) -> PathBuf {
        let version = &library.hash[..12];
        self.folder.join(format!("{}-{version}", library.id))
    }

    /// The file of the library, if it is installed.
    pub fn library_file(&self, library: &Library) -> Option<PathBuf> {
        let (_, name) = library.files.first()?;
        let folder = self.library_folder(library);
        let whole = library
            .files
            .iter()
            .all(|(_, name)| folder.join(name).is_file());
        whole.then(|| folder.join(name))
    }

    pub fn has(&self, model: &Model) -> bool {
        let size = fs::metadata(self.model_file(model)).map(|m| m.len());
        model.bundled.is_some() || size.is_ok_and(|size| size == model.bytes)
    }

    /// The bytes of a model, what they should be.
    pub fn read(&self, model: &Model) -> Result<Vec<u8>, VisionError> {
        if let Some(bytes) = model.bundled {
            return Ok(bytes.to_vec());
        }
        let bytes = fs::read(self.model_file(model))
            .map_err(|_| VisionError::Unavailable(model.id.to_owned()))?;
        if blake3::hash(&bytes).to_hex().as_str() != model.hash {
            return Err(VisionError::Corrupt);
        }
        Ok(bytes)
    }

    /// Downloads `url` to `part`: `bytes` of them, of this `hash`.
    fn download(
        &self,
        fetch: &dyn Fetch,
        (url, bytes, hash): (&str, u64, &str),
        part: &Path,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<(), VisionError> {
        fs::create_dir_all(&self.folder).map_err(io)?;
        let mut file = fs::File::create(part).map_err(io)?;
        let mut hasher = blake3::Hasher::new();
        let mut count = 0_u64;
        let mut told = 0;
        fetch.fetch(url, &mut |chunk: &[u8]| {
            cancel.check().map_err(|_| VisionError::Cancelled)?;
            count += chunk.len() as u64;
            if count > bytes {
                return Err(VisionError::Corrupt);
            }
            hasher.update(chunk);
            file.write_all(chunk).map_err(io)?;
            // A hundred times is enough.
            let share = count * 100 / bytes.max(1);
            if share > told {
                told = share;
                // At most 100: the cast is exact.
                progress.report(share as f32 / 100.0 * 0.98);
            }
            Ok(())
        })?;
        file.sync_all().map_err(io)?;
        if count != bytes || hasher.finalize().to_hex().as_str() != hash {
            return Err(VisionError::Corrupt);
        }
        Ok(())
    }

    pub fn install_model(
        &self,
        fetch: &dyn Fetch,
        model: &Model,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<(), VisionError> {
        if self.has(model) {
            progress.report(1.0);
            return Ok(());
        }
        let part = self.folder.join(format!("{}.part", model.id));
        let done = self
            .download(
                fetch,
                (model.url, model.bytes, model.hash),
                &part,
                progress,
                cancel,
            )
            .and_then(|()| fs::rename(&part, self.model_file(model)).map_err(io));
        if done.is_err() {
            let _ = fs::remove_file(&part);
        }
        done?;
        progress.report(1.0);
        Ok(())
    }

    pub fn install_library(
        &self,
        fetch: &dyn Fetch,
        library: &Library,
        progress: &dyn Progress,
        cancel: &CancelToken,
    ) -> Result<(), VisionError> {
        if self.library_file(library).is_some() {
            progress.report(1.0);
            return Ok(());
        }
        let part = self.folder.join(format!("{}.part", library.id));
        let fresh = self.folder.join(format!("{}.new", library.id));
        let done = self
            .download(
                fetch,
                (library.url, library.bytes, library.hash),
                &part,
                progress,
                cancel,
            )
            .and_then(|()| unpack(&part, library, &fresh))
            .and_then(|()| {
                let folder = self.library_folder(library);
                let _ = fs::remove_dir_all(&folder);
                fs::rename(&fresh, folder).map_err(io)
            });
        let _ = fs::remove_file(&part);
        let _ = fs::remove_dir_all(&fresh);
        done?;
        progress.report(1.0);
        Ok(())
    }

    pub fn remove_model(&self, model: &Model) -> Result<(), VisionError> {
        removed(fs::remove_file(self.model_file(model)))
    }

    pub fn remove_library(&self, library: &Library) -> Result<(), VisionError> {
        removed(fs::remove_dir_all(self.library_folder(library)))
    }

    /// What was chosen; nothing when it cannot be read.
    pub fn settings(&self) -> Settings {
        fs::read(self.folder.join("vision.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, settings: &Settings) -> Result<(), VisionError> {
        fs::create_dir_all(&self.folder).map_err(io)?;
        let bytes = serde_json::to_vec_pretty(settings).map_err(io)?;
        let part = self.folder.join("vision.json.part");
        fs::write(&part, bytes).map_err(io)?;
        fs::rename(part, self.folder.join("vision.json")).map_err(io)
    }
}

fn removed(result: std::io::Result<()>) -> Result<(), VisionError> {
    match result {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(io(error)),
        _ => Ok(()),
    }
}

/// The largest file of a library, in bytes.
const MOST_UNPACKED: u64 = 200_000_000;

/// The files of `library` in the ZIP at `part`, in the folder `to`. Their
/// names are the catalogue's, not the ZIP's.
fn unpack(part: &Path, library: &Library, to: &Path) -> Result<(), VisionError> {
    let bytes = fs::read(part).map_err(io)?;
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| VisionError::Corrupt)?;
    fs::create_dir_all(to).map_err(io)?;
    for (from, name) in library.files {
        let entry = zip.by_name(from).map_err(|_| VisionError::Corrupt)?;
        let mut content = Vec::new();
        entry
            .take(MOST_UNPACKED)
            .read_to_end(&mut content)
            .map_err(|_| VisionError::Corrupt)?;
        fs::write(to.join(name), content).map_err(io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use gafira_doc::contract::NoProgress;

    use super::*;
    use crate::catalog::Shape;

    /// Gives these bytes, some at a time, and counts the times.
    struct Fake(Vec<u8>, Mutex<u32>);

    impl Fetch for Fake {
        fn fetch(&self, _: &str, sink: &mut crate::fetch::Sink<'_>) -> Result<(), VisionError> {
            *self.1.lock().unwrap() += 1;
            self.0.chunks(7).try_for_each(sink)
        }
    }

    fn fake(bytes: &[u8]) -> Fake {
        Fake(bytes.to_vec(), Mutex::new(0))
    }

    fn model(bytes: &[u8]) -> Model {
        Model {
            id: "m",
            name: "M",
            about: "",
            license: "MIT",
            source: "",
            bytes: bytes.len() as u64,
            hash: Box::leak(blake3::hash(bytes).to_hex().to_string().into_boxed_str()),
            url: "https://example.invalid/m",
            bundled: None,
            shape: Shape::Upscale { tile: 8, scale: 2 },
        }
    }

    fn store(name: &str) -> (Store, PathBuf) {
        let folder =
            std::env::temp_dir().join(format!("gafira-vision-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        (Store::at(folder.clone()), folder)
    }

    // S84-06: what is downloaded is there, whole, once; deleting it is
    // deleting it.
    #[test]
    fn s84_06_a_model_is_installed() {
        let (store, folder) = store("install");
        let bytes: Vec<u8> = (0..200).collect();
        let (m, fetch) = (model(&bytes), fake(&bytes));
        assert!(!store.has(&m) && store.read(&m).is_err());
        let run = || store.install_model(&fetch, &m, &NoProgress, &CancelToken::new());
        assert_eq!((run(), run()), (Ok(()), Ok(())));
        assert_eq!(*fetch.1.lock().unwrap(), 1, "once");
        assert_eq!(store.read(&m).unwrap(), bytes);
        // Changed on disk, it is not used.
        fs::write(folder.join("m.onnx"), vec![9; 200]).unwrap();
        assert_eq!(store.read(&m), Err(VisionError::Corrupt));
        assert_eq!(
            (store.remove_model(&m), store.remove_model(&m)),
            (Ok(()), Ok(()))
        );
        assert!(!store.has(&m));
        fs::remove_dir_all(folder).unwrap();
    }

    // S84-06: short, long, other or stopped: nothing is left.
    #[test]
    fn s84_06_a_wrong_download_leaves_nothing() {
        let (store, folder) = store("wrong");
        let bytes: Vec<u8> = (0..200).collect();
        let m = model(&bytes);
        let mut other = bytes.clone();
        other[100] = 0;
        let cancel = CancelToken::new();
        for wrong in [&bytes[..150], &[&bytes[..], &[1]].concat(), &other] {
            let out = store.install_model(&fake(wrong), &m, &NoProgress, &cancel);
            assert_eq!(out, Err(VisionError::Corrupt));
        }
        cancel.cancel();
        let out = store.install_model(&fake(&bytes), &m, &NoProgress, &cancel);
        assert_eq!(out, Err(VisionError::Cancelled));
        let out = store.install_model(
            &crate::fetch::NoNetwork,
            &m,
            &NoProgress,
            &CancelToken::new(),
        );
        assert_eq!(out, Err(VisionError::Offline));
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 0);
        fs::remove_dir_all(folder).unwrap();
    }

    // S84-06: a library is taken out of its ZIP, with the names it is told.
    #[test]
    fn s84_06_a_library_is_unpacked() {
        let (store, folder) = store("library");
        let mut bytes = Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut bytes);
        for name in ["a/lib.so", "../evil"] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(name.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        let bytes = bytes.into_inner();
        let of = model(&bytes);
        let library = Library {
            id: "lib",
            name: "",
            about: "",
            license: "MIT",
            source: "",
            bytes: of.bytes,
            hash: of.hash,
            url: of.url,
            files: &[("a/lib.so", "lib.so")],
        };
        assert_eq!(store.library_file(&library), None);
        let run = |library: &Library| {
            store.install_library(&fake(&bytes), library, &NoProgress, &CancelToken::new())
        };
        assert_eq!(run(&library), Ok(()));
        let file = store.library_file(&library).unwrap();
        assert_eq!(fs::read(file).unwrap(), b"a/lib.so");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        store.remove_library(&library).unwrap();
        assert_eq!(store.library_file(&library), None);
        let missing = Library {
            files: &[("b", "b")],
            ..library
        };
        assert_eq!(run(&missing), Err(VisionError::Corrupt));
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 0);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn s84_05_settings() {
        let (store, folder) = store("settings");
        assert_eq!(store.settings(), Settings::default());
        let mut settings = Settings::default();
        settings.chosen.insert("cutout".into(), "u2net".into());
        store.save(&settings).unwrap();
        assert_eq!(store.settings(), settings);
        fs::write(folder.join("vision.json"), "{").unwrap();
        assert_eq!(store.settings(), Settings::default());
        fs::remove_dir_all(folder).unwrap();
    }
}
