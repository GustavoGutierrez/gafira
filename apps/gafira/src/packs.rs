//! The installed asset packages (spec 097) in the data folder:
//! `packages/<id>/<version>/` holds the files of each package as they were
//! in its `.gafpack`, with `install.json` beside them. The folders are the
//! truth; the list is read from them.
//!
//! A package is never read into memory whole: its index is checked
//! without unpacking it, then each file is read, checked or written on its
//! own. The checks of the assets share a few threads (fewer than the
//! machine's cores), each with its own reader, so a large package keeps
//! the memory of a few files and leaves the machine responsive. Installing
//! writes to a staging folder and moves it into place, so a failed install
//! leaves the previous version as it was.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufReader, Read as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use gafira_ui::{
    InstalledPack, PackInspection, PackManifest, PackProblem, PackProgress, PackSigner,
    PackStore as _, PackVersion,
};

/// The most entries a package may have.
const MOST_ENTRIES: usize = 20_000;
/// The most bytes a package unpacks to.
const MOST_BYTES: u64 = 4 * 1024 * 1024 * 1024;
/// The most bytes one file of a package unpacks to.
const MOST_FILE: u64 = 256 * 1024 * 1024;
/// The most a file may grow when it unpacks, past its first MiB: more is
/// a ZIP bomb.
const MOST_RATIO: u64 = 200;
/// The most bytes the manifest may have.
const MOST_MANIFEST: u64 = 8 * 1024 * 1024;
/// The file each installed version keeps about its install.
const INSTALL_FILE: &str = "install.json";
/// The most threads that check assets at once.
const MOST_WORKERS: usize = 4;

/// The installed packages, under `dir`.
pub(crate) struct Packs {
    dir: PathBuf,
    app: PackVersion,
    /// The publisher keys signatures are checked against (spec 112).
    trusted: &'static [gafira_ui::PackTrustedKey],
}

/// A reader of a package file.
type Archive = zip::ZipArchive<BufReader<std::fs::File>>;

/// One file of a package, from its index: where it is and its size.
#[derive(Clone, Copy)]
struct Entry {
    index: usize,
    size: u64,
}

/// The files of a package by their path in it.
type Index = BTreeMap<String, Entry>;

fn problem(key: &'static str, about: impl Into<String>) -> Vec<PackProblem> {
    vec![PackProblem::new(key, about)]
}

fn open(path: &Path) -> Result<Archive, Vec<PackProblem>> {
    let file = std::fs::File::open(path).map_err(|e| problem("packs.unreadable", e.to_string()))?;
    zip::ZipArchive::new(BufReader::new(file)).map_err(|_| problem("packs.not_a_zip", ""))
}

/// The size and change time of the file at `path`.
fn fingerprint(path: &Path) -> (u64, u128) {
    std::fs::metadata(path).map_or((0, 0), |m| {
        let changed = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        (m.len(), changed)
    })
}

/// The index of `archive`, read without unpacking anything: an encrypted,
/// linked or unsafe entry, a name used twice, and anything past the limits
/// refuse the package (§9.1).
fn index(archive: &mut Archive) -> Result<Index, Vec<PackProblem>> {
    if archive.len() > MOST_ENTRIES {
        return Err(problem("packs.too_many_files", archive.len().to_string()));
    }
    let mut out = Index::new();
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|_| problem("packs.not_a_zip", ""))?;
        let name = entry.name().map(|n| n.into_owned()).unwrap_or_default();
        if entry.encrypted() {
            return Err(problem("packs.encrypted", name));
        }
        if entry.is_dir() {
            continue;
        }
        if entry.is_symlink() || !gafira_ui::pack_safe_path(&name) {
            return Err(problem("packs.unsafe_path", name));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(problem("packs.duplicate_path", name));
        }
        let size = entry.size();
        let packed = entry.compressed_size().max(1);
        if size > MOST_FILE {
            return Err(problem("packs.file_too_big", name));
        }
        if size > 1024 * 1024 && size / packed > MOST_RATIO {
            return Err(problem("packs.zip_bomb", name));
        }
        total = total.saturating_add(size);
        if total > MOST_BYTES {
            return Err(problem("packs.too_big", ""));
        }
        out.insert(name, Entry { index, size });
    }
    Ok(out)
}

/// The bytes of one file of the package; `None` when it does not read or
/// unpacks to another size than its index says.
fn read_entry(archive: &mut Archive, entry: Entry) -> Option<Vec<u8>> {
    let file = archive.by_index(entry.index).ok()?;
    let mut bytes = Vec::with_capacity(usize::try_from(entry.size).unwrap_or(0));
    file.take(entry.size + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 == entry.size).then_some(bytes)
}

/// The bytes of the files an asset uses, for its share of the progress.
fn weight(asset: &gafira_ui::PackManifest, index: &Index) -> Vec<u64> {
    asset
        .assets
        .iter()
        .map(|a| {
            [&a.file, &a.preview, &a.definition]
                .into_iter()
                .flatten()
                .filter_map(|f| index.get(f))
                .map(|e| e.size)
                .sum::<u64>()
                .max(1)
        })
        .collect()
}

/// Whether a template file reads as a template, with the reader of the
/// templates Gafira carries (§6.4).
fn template_ok(bytes: &[u8]) -> bool {
    matches!(
        gafira_format::read_template_info(
            std::io::Cursor::new(bytes),
            gafira_format::Limits::default()
        ),
        Ok(Some(_))
    )
}

/// Who signed the package (spec 112): without `META-INF/signature.json`
/// nothing more is read; with it, every file it covers is hashed, one at a
/// time, and checked against it.
fn signer_of(
    archive: &mut Archive,
    index: &Index,
    author: &str,
    trusted: &[gafira_ui::PackTrustedKey],
) -> Result<PackSigner, Vec<PackProblem>> {
    let Some(&entry) = index.get(gafira_ui::PACK_SIGNATURE_PATH) else {
        return Ok(PackSigner::Unsigned);
    };
    if entry.size > MOST_MANIFEST {
        return Err(problem(
            "packs.bad_signature",
            gafira_ui::PACK_SIGNATURE_PATH,
        ));
    }
    let signature = read_entry(archive, entry)
        .ok_or_else(|| problem("packs.bad_signature", gafira_ui::PACK_SIGNATURE_PATH))?;
    let mut files = BTreeMap::new();
    for (name, &entry) in index {
        if gafira_ui::pack_signed_path(name) {
            let bytes = read_entry(archive, entry)
                .ok_or_else(|| problem("packs.unreadable", name.clone()))?;
            files.insert(name.clone(), gafira_ui::pack_sha256(&bytes));
        }
    }
    gafira_ui::pack_signer(Some(&signature), &files, author, trusted).map_err(|p| vec![p])
}

/// The manifest, thumbnail and signer of the package at `path`, and every
/// problem of it: its index, its manifest, then each asset's files,
/// checked by a few threads at once while `progress` hears how far they
/// are, then its signature.
fn check(
    path: &Path,
    app: &PackVersion,
    trusted: &[gafira_ui::PackTrustedKey],
    progress: &dyn Fn(PackProgress),
) -> Result<(PackManifest, Vec<u8>, PackSigner), Vec<PackProblem>> {
    let mut archive = open(path)?;
    let index = index(&mut archive)?;
    let Some(&entry) = index.get("manifest.json") else {
        return Err(problem("packs.no_manifest", "manifest.json"));
    };
    if entry.size > MOST_MANIFEST {
        return Err(problem("packs.bad_manifest", "manifest.json"));
    }
    let bytes = read_entry(&mut archive, entry)
        .ok_or_else(|| problem("packs.bad_manifest", "manifest.json"))?;
    let manifest: PackManifest =
        serde_json::from_slice(&bytes).map_err(|e| problem("packs.bad_manifest", e.to_string()))?;
    let present: BTreeSet<String> = index.keys().cloned().collect();
    let problems = gafira_ui::pack_manifest_problems(&manifest, &present, app);
    if !problems.is_empty() {
        return Err(problems);
    }
    let mut read = |name: &str| index.get(name).and_then(|&e| read_entry(&mut archive, e));
    let thumbnail = read(gafira_ui::PACK_THUMBNAIL).unwrap_or_default();
    let mut problems = gafira_ui::pack_thumbnail_problems(|_| Some(thumbnail.clone()));
    // The assets, shared out among a few threads.
    let weights = weight(&manifest, &index);
    let total: u64 = weights.iter().sum();
    let done = AtomicU64::new(0);
    let next = AtomicUsize::new(0);
    let running = AtomicUsize::new(0);
    let found = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get().saturating_sub(1))
        .clamp(1, MOST_WORKERS)
        .min(manifest.assets.len().max(1));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            running.fetch_add(1, Ordering::SeqCst);
            scope.spawn(|| {
                let mut mine = Vec::new();
                match open(path) {
                    Err(error) => mine.extend(error),
                    Ok(mut archive) => loop {
                        let at = next.fetch_add(1, Ordering::SeqCst);
                        let Some(asset) = manifest.assets.get(at) else {
                            break;
                        };
                        let mut read =
                            |name: &str| index.get(name).and_then(|&e| read_entry(&mut archive, e));
                        mine.extend(gafira_ui::pack_asset_problems(asset, &present, &mut read));
                        if asset.kind == "template"
                            && let Some(file) = &asset.file
                            && read(file).is_some_and(|bytes| !template_ok(&bytes))
                        {
                            mine.push(PackProblem::new("packs.bad_template", file.clone()));
                        }
                        done.fetch_add(weights[at], Ordering::SeqCst);
                    },
                }
                if let Ok(mut all) = found.lock() {
                    all.extend(mine);
                }
                running.fetch_sub(1, Ordering::SeqCst);
            });
        }
        // This thread tells how far they are until they end.
        while running.load(Ordering::SeqCst) > 0 {
            progress(PackProgress {
                done: done.load(Ordering::SeqCst),
                total,
                installing: false,
            });
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    });
    problems.extend(found.into_inner().unwrap_or_default());
    if !problems.is_empty() {
        return Err(problems);
    }
    let signer = signer_of(&mut archive, &index, &manifest.author.name, trusted)?;
    Ok((manifest, thumbnail, signer))
}

/// How `install.json` keeps a verified signer.
fn signer_note(signer: &PackSigner) -> serde_json::Value {
    match signer {
        PackSigner::Verified { publisher, key_id } => {
            serde_json::json!({ "publisher": publisher, "keyId": key_id })
        }
        _ => serde_json::Value::Null,
    }
}

/// The verified signer kept in `install.json`, if any.
fn signer_from(note: &serde_json::Value) -> PackSigner {
    match (note["publisher"].as_str(), note["keyId"].as_str()) {
        (Some(publisher), Some(key_id)) => PackSigner::Verified {
            publisher: publisher.to_owned(),
            key_id: key_id.to_owned(),
        },
        _ => PackSigner::Unsigned,
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn disk(error: std::io::Error) -> Vec<PackProblem> {
    problem("packs.disk", error.to_string())
}

/// The size of the files under `dir`, in bytes.
fn size_of(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map_or(0, |read| {
        read.flatten()
            .map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    size_of(&path)
                } else {
                    entry.metadata().map_or(0, |m| m.len())
                }
            })
            .sum()
    })
}

impl Packs {
    /// The store over `dir`, for a Gafira of version `app`.
    pub(crate) fn new(dir: PathBuf, app: &str) -> Self {
        let app = PackVersion::parse(app)
            .or_else(|| PackVersion::parse("0.0.0"))
            .expect("a version");
        Self {
            dir,
            app,
            trusted: gafira_ui::PACK_TRUSTED_KEYS,
        }
    }

    /// The same store, checking signatures against `trusted` instead.
    #[cfg(test)]
    fn trusting(self, trusted: &'static [gafira_ui::PackTrustedKey]) -> Self {
        Self { trusted, ..self }
    }

    /// The installed version of `id`, if any.
    fn version_of(&self, id: &str) -> Option<String> {
        self.installed()
            .into_iter()
            .find(|p| p.manifest.id == id)
            .map(|p| p.manifest.version)
    }

    /// The installed package in `dir`, if it reads.
    fn read_installed(dir: &Path) -> Option<InstalledPack> {
        let bytes = std::fs::read(dir.join("manifest.json")).ok()?;
        let manifest: PackManifest = serde_json::from_slice(&bytes).ok()?;
        let note = std::fs::read(dir.join(INSTALL_FILE))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .unwrap_or_default();
        Some(InstalledPack {
            manifest,
            dir: dir.to_path_buf(),
            installed_at: note["installedAt"].as_u64().unwrap_or(0),
            size: size_of(dir),
            signer: signer_from(&note["signer"]),
        })
    }

    /// Writes every file of the package at `path` under `staging`, one at
    /// a time and without holding it in memory.
    fn unpack(
        path: &Path,
        staging: &Path,
        progress: &dyn Fn(PackProgress),
    ) -> Result<(), Vec<PackProblem>> {
        let mut archive = open(path)?;
        let index = index(&mut archive)?;
        let total: u64 = index.values().map(|e| e.size).sum::<u64>().max(1);
        let mut done = 0u64;
        for (name, entry) in &index {
            // Every name was checked safe; it stays inside `staging`.
            let to = staging.join(name);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(disk)?;
            }
            let file = archive
                .by_index(entry.index)
                .map_err(|_| problem("packs.not_a_zip", name.clone()))?;
            let mut out = std::io::BufWriter::new(std::fs::File::create(&to).map_err(disk)?);
            let written = std::io::copy(&mut file.take(entry.size + 1), &mut out).map_err(disk)?;
            if written != entry.size {
                return Err(problem("packs.zip_bomb", name.clone()));
            }
            std::io::Write::flush(&mut out).map_err(disk)?;
            done += entry.size;
            progress(PackProgress {
                done,
                total,
                installing: true,
            });
        }
        Ok(())
    }
}

impl gafira_ui::PackStore for Packs {
    fn inspect(
        &self,
        path: &Path,
        progress: &dyn Fn(PackProgress),
    ) -> Result<PackInspection, Vec<PackProblem>> {
        let print = fingerprint(path);
        let (manifest, thumbnail, signer) = check(path, &self.app, self.trusted, progress)?;
        let installed = self.version_of(&manifest.id);
        Ok(PackInspection {
            manifest,
            thumbnail,
            size: print.0,
            installed,
            fingerprint: print,
            signer,
        })
    }

    fn install(
        &self,
        path: &Path,
        checked: Option<&PackInspection>,
        progress: &dyn Fn(PackProgress),
    ) -> Result<InstalledPack, Vec<PackProblem>> {
        // The file checked a moment ago and unchanged since is not checked
        // twice; any other is checked first.
        let (manifest, signer) = match checked.filter(|c| c.fingerprint == fingerprint(path)) {
            Some(c) => (c.manifest.clone(), c.signer.clone()),
            None => {
                let (manifest, _, signer) = check(path, &self.app, self.trusted, progress)?;
                (manifest, signer)
            }
        };
        std::fs::create_dir_all(&self.dir).map_err(disk)?;
        let staging = self.dir.join(format!(".staging-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        let written = Self::unpack(path, &staging, progress).and_then(|()| {
            let source = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let note = serde_json::json!({
                "installedAt": now(),
                "source": source,
                "signer": signer_note(&signer),
            });
            std::fs::write(staging.join(INSTALL_FILE), note.to_string()).map_err(disk)
        });
        if let Err(problems) = written {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(problems);
        }
        // Into place: the same version is swapped, then every other
        // version of the package goes.
        let home = self.dir.join(&manifest.id);
        let target = home.join(&manifest.version);
        let old = home.join(format!(".old-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&old);
        std::fs::create_dir_all(&home).map_err(disk)?;
        let had = target.exists();
        if had && let Err(error) = std::fs::rename(&target, &old) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(disk(error));
        }
        if let Err(error) = std::fs::rename(&staging, &target) {
            if had {
                let _ = std::fs::rename(&old, &target);
            }
            let _ = std::fs::remove_dir_all(&staging);
            return Err(disk(error));
        }
        let _ = std::fs::remove_dir_all(&old);
        if let Ok(read) = std::fs::read_dir(&home) {
            for entry in read.flatten() {
                if entry.path() != target {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
        Self::read_installed(&target).ok_or_else(|| problem("packs.disk", "manifest.json"))
    }

    fn uninstall(&self, id: &str) -> Result<(), PackProblem> {
        // Only a package's own folder, never a path a name could make.
        let home = self.dir.join(id);
        let ours = home.parent() == Some(self.dir.as_path())
            && self.installed().iter().any(|p| p.manifest.id == id);
        if !ours {
            return Err(PackProblem::new("packs.not_installed", id));
        }
        std::fs::remove_dir_all(&home).map_err(|e| PackProblem::new("packs.disk", e.to_string()))
    }

    fn installed(&self) -> Vec<InstalledPack> {
        let Ok(read) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out: Vec<InstalledPack> = read
            .flatten()
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .filter_map(|home| {
                std::fs::read_dir(home.path())
                    .ok()?
                    .flatten()
                    .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                    .filter_map(|version| Self::read_installed(&version.path()))
                    .max_by(|a, b| {
                        PackVersion::parse(&a.manifest.version)
                            .cmp(&PackVersion::parse(&b.manifest.version))
                    })
            })
            .collect();
        out.sort_by(|a, b| {
            a.manifest
                .name
                .en
                .to_lowercase()
                .cmp(&b.manifest.name.en.to_lowercase())
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gafira-packs-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn encoded(image: image::RgbaImage, format: image::ImageFormat) -> Vec<u8> {
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut out), format)
            .expect("encodes");
        out
    }

    fn manifest(version: &str) -> serde_json::Value {
        serde_json::json!({
            "format": "gafpack", "formatVersion": "1.1",
            "id": "com.gafira.test.christmas", "version": version,
            "packageType": "clipart", "categories": ["illustrations", "icons"],
            "name": {"es": "Navidad", "en": "Christmas"},
            "description": {"es": "Ilustraciones de Navidad.", "en": "Christmas illustrations."},
            "author": {"name": "Gafira Studio"},
            "thumbnail": "thumb.webp", "license": "CC0-1.0",
            "minGafiraVersion": "0.1.0-beta.1",
            "assets": [
                {"id": "santa", "type": "clipart", "name": "Santa", "file": "assets/santa.png"},
                {"id": "gift", "type": "icon", "name": "Regalo", "file": "assets/gift.svg"}
            ]
        })
    }

    /// A package at `path` with `manifest` and the files it names, plus
    /// `extra` entries.
    fn package(path: &Path, manifest: &serde_json::Value, extra: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).expect("creates"));
        let options = zip::write::SimpleFileOptions::default();
        let thumb = encoded(
            image::RgbaImage::from_pixel(256, 256, image::Rgba([10, 20, 30, 255])),
            image::ImageFormat::WebP,
        );
        let santa = encoded(
            image::RgbaImage::from_pixel(16, 16, image::Rgba([200, 0, 0, 0])),
            image::ImageFormat::Png,
        );
        let gift: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4"><rect width="4" height="4"/></svg>"#;
        let mut add = |name: &str, bytes: &[u8]| {
            zip.start_file(name, options).expect("entry");
            zip.write_all(bytes).expect("writes");
        };
        add("manifest.json", manifest.to_string().as_bytes());
        add("thumb.webp", &thumb);
        add("assets/santa.png", &santa);
        add("assets/gift.svg", gift);
        for (name, bytes) in extra {
            add(name, bytes);
        }
        zip.finish().expect("finishes");
    }

    const TEST_KEYS: &[gafira_ui::PackTrustedKey] = &[gafira_ui::PackTrustedKey {
        id: "test-key",
        publisher: "Gafira Studio",
        public: "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8",
    }];

    /// `from` copied to `to` with a signature of every file, by the test
    /// key, as `publisher`; `change` then replaces one file's bytes.
    fn signed_copy(from: &Path, to: &Path, publisher: &str, change: Option<(&str, &[u8])>) {
        use ed25519_dalek::Signer as _;
        let mut archive = open(from).expect("opens");
        let mut files = BTreeMap::new();
        let mut entries = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).expect("entry");
            let name = file.name().expect("name").into_owned();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).expect("reads");
            files.insert(name.clone(), gafira_ui::pack_sha256(&bytes));
            entries.push((name, bytes));
        }
        let seed: [u8; 32] = std::array::from_fn(|i| i as u8);
        let key = ed25519_dalek::SigningKey::from_bytes(&seed);
        let message = gafira_ui::pack_signed_message("test-key", publisher, &files);
        let signature: String = key
            .sign(&message)
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let note = serde_json::json!({
            "format": "gafpack-signature", "version": 1, "algorithm": "ed25519",
            "keyId": "test-key", "publisher": publisher, "files": files, "signature": signature,
        });
        let mut zip = zip::ZipWriter::new(std::fs::File::create(to).expect("creates"));
        let options = zip::write::SimpleFileOptions::default();
        for (name, bytes) in entries {
            let bytes = match change {
                Some((changed, new)) if changed == name => new.to_vec(),
                _ => bytes,
            };
            zip.start_file(name, options).expect("entry");
            zip.write_all(&bytes).expect("writes");
        }
        zip.start_file(gafira_ui::PACK_SIGNATURE_PATH, options)
            .expect("entry");
        zip.write_all(note.to_string().as_bytes()).expect("writes");
        zip.finish().expect("finishes");
    }

    // FIR-06: a package signed by a trusted key is verified when it is
    // inspected and installed, and stays verified in the installed list.
    #[test]
    fn fir_06_a_signed_package_installs_verified() {
        let dir = temp("signed");
        let store = Packs::new(dir.join("packages"), "0.1.0-beta.4").trusting(TEST_KEYS);
        let plain = dir.join("plain.gafpack");
        package(&plain, &manifest("1.0.0"), &[]);
        let file = dir.join("signed.gafpack");
        signed_copy(&plain, &file, "Gafira Studio", None);
        let seen = store.inspect(&file, &|_| {}).expect("valid");
        assert_eq!(seen.signer.verified(), Some("Gafira Studio"));
        let pack = store
            .install(&file, Some(&seen), &|_| {})
            .expect("installs");
        assert_eq!(pack.signer.verified(), Some("Gafira Studio"));
        assert_eq!(
            store.installed()[0].signer.verified(),
            Some("Gafira Studio")
        );
        // The same package against Gafira's own keys is signed by a key it
        // does not know: it installs, not verified (FIR-05).
        let shipped = Packs::new(dir.join("other"), "0.1.0-beta.4");
        assert_eq!(
            shipped.inspect(&file, &|_| {}).expect("valid").signer,
            PackSigner::Unknown {
                key_id: "test-key".to_owned()
            }
        );
        // FIR-01: without a signature nothing changes.
        assert_eq!(
            store.inspect(&plain, &|_| {}).expect("valid").signer,
            PackSigner::Unsigned
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // FIR-03, FIR-04: a file changed after signing, or an author other
    // than the signer, refuses the package and installs nothing.
    #[test]
    fn fir_03_a_changed_signed_package_is_refused() {
        let dir = temp("tampered");
        let store = Packs::new(dir.join("packages"), "0.1.0-beta.4").trusting(TEST_KEYS);
        let plain = dir.join("plain.gafpack");
        package(&plain, &manifest("1.0.0"), &[]);
        let changed = dir.join("changed.gafpack");
        let svg: &[u8] =
            br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4"><circle r="2"/></svg>"#;
        signed_copy(
            &plain,
            &changed,
            "Gafira Studio",
            Some(("assets/gift.svg", svg)),
        );
        let problems = store.inspect(&changed, &|_| {}).expect_err("refused");
        assert_eq!(problems[0].key, "packs.signature_mismatch");
        assert!(store.install(&changed, None, &|_| {}).is_err());
        assert!(store.installed().is_empty());
        let other = dir.join("other.gafpack");
        signed_copy(&plain, &other, "Other Studio", None);
        let problems = store.inspect(&other, &|_| {}).expect_err("refused");
        assert_eq!(problems[0].key, "packs.bad_signature");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // S97-08: a valid package is inspected without installing it, then
    // installed: its files are in place, it is listed, and its counts and
    // thumbnail come with the inspection.
    #[test]
    fn s97_08_a_package_is_inspected_then_installed() {
        let dir = temp("install");
        let store = Packs::new(dir.join("packages"), "0.1.0-beta.4");
        let file = dir.join("christmas.gafpack");
        package(&file, &manifest("1.0.0"), &[]);
        let seen = store.inspect(&file, &|_| {}).expect("valid");
        assert_eq!(seen.manifest.name.es, "Navidad");
        assert!(seen.thumbnail.starts_with(b"RIFF"));
        assert_eq!(seen.installed, None);
        assert!(store.installed().is_empty(), "inspecting installs nothing");
        let pack = store.install(&file, None, &|_| {}).expect("installs");
        assert!(pack.dir.join("assets/santa.png").is_file());
        assert!(pack.thumbnail().is_file());
        assert_eq!(store.installed().len(), 1);
        assert_eq!(
            store
                .inspect(&file, &|_| {})
                .expect("valid")
                .installed
                .as_deref(),
            Some("1.0.0")
        );
        // Installed again, the same version stays one package.
        // Installed again from its inspection: the progress of the install
        // reaches the whole, nothing is checked twice, and the same version
        // stays one package.
        let seen = store.inspect(&file, &|_| {}).expect("valid");
        let last = std::sync::Mutex::new(gafira_ui::PackProgress::default());
        let checked = std::sync::atomic::AtomicBool::new(false);
        store
            .install(&file, Some(&seen), &|p| {
                if !p.installing {
                    checked.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                *last.lock().expect("lock") = p;
            })
            .expect("reinstalls");
        let last = *last.lock().expect("lock");
        assert!(last.installing && last.done == last.total, "{last:?}");
        assert!(!checked.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(store.installed().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    // S97-09: a newer version replaces the older one; uninstalling removes
    // the package and only it.
    #[test]
    fn s97_09_update_and_uninstall() {
        let dir = temp("update");
        let store = Packs::new(dir.join("packages"), "0.1.0-beta.4");
        let file = dir.join("christmas.gafpack");
        package(&file, &manifest("1.0.0"), &[]);
        store.install(&file, None, &|_| {}).expect("installs");
        package(&file, &manifest("1.1.0"), &[]);
        let pack = store.install(&file, None, &|_| {}).expect("updates");
        let all = store.installed();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].manifest.version, "1.1.0");
        assert!(
            !pack.dir.with_file_name("1.0.0").exists(),
            "the old one went"
        );
        assert!(store.uninstall("../packages").is_err());
        store
            .uninstall("com.gafira.test.christmas")
            .expect("uninstalls");
        assert!(store.installed().is_empty());
        assert!(
            dir.join("packages").is_dir(),
            "the folder of packages stays"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // S97-14: a package of composite assets installs: a frame with a slot
    // and a powerclip by their masks, a collage by its slots, a template
    // as a template `.gaf`; a template that is not one is refused.
    #[test]
    fn s97_14_composite_assets_install() {
        let dir = temp("composite");
        let store = Packs::new(dir.join("packages"), "0.1.0-beta.4");
        let file = dir.join("mixed.gafpack");
        let template = crate::templates::BUNDLED
            .first()
            .map(|(_, bytes)| bytes.to_vec())
            .expect("a bundled template");
        let mut m = manifest("1.0.0");
        m["packageType"] = "mixed".into();
        m["categories"] = serde_json::json!([
            "illustrations",
            "icons",
            "frames",
            "powerclips",
            "collages",
            "templates"
        ]);
        let assets = m["assets"].as_array_mut().expect("assets");
        assets.push(serde_json::json!({"id": "slot", "type": "frame", "name": "Marco", "definition": "frames/slot.json"}));
        assets.push(serde_json::json!({"id": "heart", "type": "powerclip", "name": "Corazón", "definition": "masks/heart.json"}));
        assets.push(serde_json::json!({"id": "two", "type": "collage", "name": "Dos", "definition": "collages/two.json"}));
        assets.push(serde_json::json!({"id": "card", "type": "template", "name": "Tarjeta", "file": "templates/card.gaf"}));
        let mask: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M5 9 1 4a2 2 0 0 1 4-2 2 2 0 0 1 4 2z"/></svg>"#;
        let files: Vec<(&str, &[u8])> = vec![
            ("frames/slot.json", br#"{"schema":"gafira.frame","version":1,"mask":"slot.svg"}"#),
            ("frames/slot.svg", mask),
            ("masks/heart.json", br#"{"schema":"gafira.powerclip","version":1,"mask":"heart.svg","contentFit":"contain"}"#),
            ("masks/heart.svg", mask),
            ("collages/two.json", br#"{"schema":"gafira.collage","version":1,"slots":[{"x":0,"y":0,"w":0.5,"h":1},{"x":0.5,"y":0,"w":0.5,"h":1}]}"#),
            ("templates/card.gaf", &template),
        ];
        package(&file, &m, &files);
        let pack = store.install(&file, None, &|_| {}).expect("installs");
        let used: Vec<gafira_ui::PackUse> = pack
            .manifest
            .assets
            .iter()
            .filter_map(|a| pack.usage(a))
            .collect();
        assert!(used.iter().any(|u| matches!(
            u,
            gafira_ui::PackUse::Frame {
                fit: gafira_ui::PackContentFit::Contain,
                ..
            }
        )));
        assert!(
            used.iter().any(
                |u| matches!(u, gafira_ui::PackUse::Collage { cells, .. } if cells.len() == 2)
            )
        );
        assert!(
            used.iter()
                .any(|u| matches!(u, gafira_ui::PackUse::Template(p) if p.is_file()))
        );
        // A `.gaf` that is not a template is refused.
        let mut files = files;
        files[5] = ("templates/card.gaf", b"not a template");
        package(&file, &m, &files);
        let refused = store.inspect(&file, &|_| {}).err().unwrap_or_default();
        assert!(
            refused.iter().any(|p| p.key == "packs.bad_template"),
            "{refused:?}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // S97-10: a package with a path out of it, a missing file, a bad hash
    // or a newer minimum Gafira is refused, and nothing is written.
    #[test]
    fn s97_10_invalid_packages_install_nothing() {
        let dir = temp("refuse");
        let store = Packs::new(dir.join("packages"), "0.1.0-beta.4");
        let file = dir.join("bad.gafpack");
        let keys = |store: &Packs| -> Vec<&'static str> {
            store
                .install(&file, None, &|_| {})
                .err()
                .unwrap_or_default()
                .iter()
                .map(|p| p.key)
                .collect()
        };
        package(&file, &manifest("1.0.0"), &[("../evil.txt", b"x")]);
        assert_eq!(keys(&store), vec!["packs.unsafe_path"]);
        assert!(!dir.join("evil.txt").exists());
        let mut missing = manifest("1.0.0");
        missing["assets"][0]["file"] = "assets/none.png".into();
        package(&file, &missing, &[]);
        assert!(keys(&store).contains(&"packs.missing_file"));
        let mut hashed = manifest("1.0.0");
        hashed["assets"][1]["hash"] = "0".repeat(64).into();
        package(&file, &hashed, &[]);
        assert!(keys(&store).contains(&"packs.hash_mismatch"));
        let mut newer = manifest("1.0.0");
        newer["minGafiraVersion"] = "9.0.0".into();
        package(&file, &newer, &[]);
        assert!(keys(&store).contains(&"packs.needs_newer"));
        std::fs::write(&file, b"not a zip").expect("writes");
        assert_eq!(keys(&store), vec!["packs.not_a_zip"]);
        assert!(store.installed().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
