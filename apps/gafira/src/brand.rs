//! The brand kits of Marca (spec 093) in the data folder: `brand/kits.json`
//! holds every kit and the one in use; `brand/<kit>/<kind>/` its files and
//! `brand/<kit>/templates/` its brand templates, each a `.gaf` marked with
//! its `template` (D-81).

use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use gafira_format::{Limits, read_template_info, read_thumbnail, set_template};
use gafira_ports::repository::{RepositoryError, TemplateMeta};
use gafira_ui::{BrandColor, BrandFiles, BrandKit, BrandPalette, BrandTextStyle, KitError};
use serde_json::{Value, json};

pub(crate) struct Brand {
    pub(crate) dir: PathBuf,
    /// The font catalogue the uploaded fonts join, so texts and exports
    /// find them as they find the system's (spec 062).
    pub(crate) fonts: gafira_ui::SystemFonts,
}

fn style_json(style: &BrandTextStyle) -> Value {
    json!({
        "name": style.name,
        "font": style.font,
        "size": style.size,
        "bold": style.bold,
        "italic": style.italic
    })
}

fn style_of(value: &Value, fallback: &BrandTextStyle) -> BrandTextStyle {
    BrandTextStyle {
        name: value["name"].as_str().unwrap_or(&fallback.name).to_owned(),
        font: value["font"].as_str().unwrap_or(&fallback.font).to_owned(),
        size: value["size"].as_f64().unwrap_or(fallback.size),
        bold: value["bold"].as_bool().unwrap_or(fallback.bold),
        italic: value["italic"].as_bool().unwrap_or(fallback.italic),
    }
}

/// `kit` as stored.
pub(crate) fn kit_json(kit: &BrandKit) -> Value {
    let paths = |list: &[PathBuf]| -> Value {
        list.iter()
            .map(|p| Value::from(p.display().to_string()))
            .collect()
    };
    let mut files = serde_json::Map::new();
    for kind in BrandFiles::ALL {
        files.insert(kind.folder().to_owned(), paths(kit.files(kind)));
    }
    json!({
        "id": kit.id,
        "name": kit.name,
        "files": files,
        "palettes": kit.palettes.iter().map(|p| json!({
            "name": p.name,
            "colors": p.colors.iter().map(|c| json!({
                "hex": format!("#{:06X}", c.rgb),
                "cmyk": c.cmyk,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "styles": kit.styles.iter().map(style_json).collect::<Vec<_>>(),
    })
}

/// The kit `value` holds, if it names an id and a name; anything else it
/// lacks takes the defaults of a new kit.
pub(crate) fn kit_of(value: &Value) -> Option<BrandKit> {
    let id = value["id"].as_str()?.to_owned();
    let name = value["name"].as_str()?.to_owned();
    let mut kit = BrandKit::new(id, name);
    for kind in BrandFiles::ALL {
        *kit.files_mut(kind) = value["files"][kind.folder()]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_str)
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default();
    }
    kit.palettes = value["palettes"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|p| BrandPalette {
                    name: p["name"].as_str().unwrap_or_default().to_owned(),
                    colors: p["colors"]
                        .as_array()
                        .map(|c| {
                            c.iter()
                                .filter_map(|c| {
                                    let rgb = gafira_ui::parse_brand_hex(c["hex"].as_str()?)?;
                                    let cmyk = c["cmyk"].as_array().and_then(|parts| {
                                        let parts: Vec<u8> = parts
                                            .iter()
                                            .filter_map(Value::as_u64)
                                            .map(|v| v.min(100) as u8)
                                            .collect();
                                        parts.try_into().ok()
                                    });
                                    Some(BrandColor { rgb, cmyk })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(styles) = value["styles"].as_array() {
        let fallback = kit.styles[0].clone();
        kit.styles = styles.iter().map(|s| style_of(s, &fallback)).collect();
    }
    Some(kit)
}

/// What a `.gafkit` says it is in its `kit.json` (spec 094).
const KIT_FORMAT: &str = "gafira-brandkit";
const KIT_VERSION: u64 = 1;
/// The most entries and bytes a `.gafkit` may unpack to: a kit is logos,
/// photos, fonts and templates, not a disk image.
const KIT_MOST_ENTRIES: usize = 4096;
const KIT_MOST_BYTES: u64 = 2 << 30;
/// The folder of a kit's brand templates, in its folder and its package.
const TEMPLATES: &str = "templates";

fn io(error: impl std::fmt::Display) -> KitError {
    KitError::Io(error.to_string())
}

/// The folder and file name of a package entry, when it is one of a kit's
/// folders and a plain file name: nothing else is ever written.
fn safe_entry(name: &str) -> Option<(&str, &str)> {
    let (folder, file) = name.split_once('/')?;
    let known = BrandFiles::ALL.iter().any(|k| k.folder() == folder) || folder == TEMPLATES;
    let plain = !file.is_empty()
        && !file.starts_with('.')
        && !file.contains(['/', '\\', ':'])
        && file != "..";
    (known && plain).then_some((folder, file))
}

/// The `kit.json` of the package `archive`, if it is a kit of a version
/// this Gafira reads.
fn package_kit<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Result<Value, KitError> {
    use std::io::Read;
    let entry = archive.by_name("kit.json").map_err(|_| KitError::NotAKit)?;
    let mut text = String::new();
    entry
        .take(16 << 20)
        .read_to_string(&mut text)
        .map_err(|_| KitError::NotAKit)?;
    let value: Value = serde_json::from_str(&text).map_err(|_| KitError::NotAKit)?;
    let known = value["format"] == KIT_FORMAT
        && value["version"].as_u64().is_some_and(|v| v <= KIT_VERSION);
    if known && value["kit"]["name"].is_string() {
        Ok(value)
    } else {
        Err(KitError::NotAKit)
    }
}

fn open_package(source: &Path) -> Result<zip::ZipArchive<std::fs::File>, KitError> {
    let file = std::fs::File::open(source).map_err(io)?;
    let archive = zip::ZipArchive::new(file).map_err(|_| KitError::NotAKit)?;
    if archive.len() > KIT_MOST_ENTRIES {
        return Err(KitError::NotAKit);
    }
    Ok(archive)
}

impl Brand {
    /// The store over `dir`, with the fonts of every kit added to `fonts`.
    pub(crate) fn new(dir: PathBuf, fonts: gafira_ui::SystemFonts) -> Self {
        let brand = Self { dir, fonts };
        brand.register();
        brand
    }

    /// Gafira's uploaded fonts are the font files in the kits' folders, one
    /// face per family, weight and style: of a family that comes as one
    /// file per weight, the regular and the bold.
    fn register(&self) {
        let files: Vec<PathBuf> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .flat_map(|kit| {
                std::fs::read_dir(kit.path().join(BrandFiles::Fonts.folder()))
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|file| file.path())
            })
            .collect();
        let families: Vec<(PathBuf, String)> = files
            .iter()
            .filter_map(|path| {
                let bytes = std::fs::read(path).ok()?;
                let face = gafira_platform::face_of(&bytes, path)?;
                Some((path.clone(), face.family))
            })
            .collect();
        let faces = gafira_platform::faces_of_files(files)
            .into_iter()
            .map(|face| gafira_ui::SystemFont {
                family: face.family,
                bold: face.bold,
                italic: face.italic,
                path: face.path,
            })
            .collect();
        self.fonts.set_added(faces, families);
    }

    fn file(&self) -> PathBuf {
        self.dir.join("kits.json")
    }

    fn read(&self) -> Value {
        std::fs::read(self.file())
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_else(|| json!({ "kits": [], "active": null }))
    }

    fn write(&self, value: &Value) {
        let written = std::fs::create_dir_all(&self.dir).and_then(|()| {
            let tmp = self.file().with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_vec_pretty(value).unwrap_or_default())?;
            std::fs::rename(tmp, self.file())
        });
        if let Err(error) = written {
            eprintln!("brand: {error}");
        }
    }

    /// The folder of the kit `id`; ids are ours (`kit-<n>`), so it stays
    /// inside the brand folder.
    fn kit_dir(&self, id: &str) -> PathBuf {
        let safe: String = id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        self.dir.join(safe)
    }

    /// A free path for `name` in `dir`.
    fn free(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        if !path.exists() {
            return path;
        }
        let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
        (2..)
            .map(|n| dir.join(format!("{stem}-{n}.{ext}")))
            .find(|p| !p.exists())
            .unwrap_or(path)
    }
}

impl gafira_ui::BrandStore for Brand {
    fn kits(&self) -> Vec<BrandKit> {
        self.read()["kits"]
            .as_array()
            .map(|list| list.iter().filter_map(kit_of).collect())
            .unwrap_or_default()
    }

    fn save(&self, kit: &BrandKit) {
        let mut value = self.read();
        let list = value["kits"].as_array_mut();
        let Some(list) = list else { return };
        let stored = kit_json(kit);
        match list.iter_mut().find(|k| k["id"] == kit.id.as_str()) {
            Some(slot) => *slot = stored,
            None => list.push(stored),
        }
        self.write(&value);
    }

    fn delete(&self, id: &str) {
        let mut value = self.read();
        if let Some(list) = value["kits"].as_array_mut() {
            list.retain(|k| k["id"] != id);
        }
        if value["active"] == id {
            value["active"] = Value::Null;
        }
        self.write(&value);
        let dir = self.kit_dir(id);
        if dir.starts_with(&self.dir)
            && dir != self.dir
            && let Err(error) = std::fs::remove_dir_all(&dir)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!("brand {}: {error}", dir.display());
        }
        self.register();
    }

    fn add_file(&self, id: &str, kind: BrandFiles, source: &Path) -> Option<PathBuf> {
        let dir = self.kit_dir(id).join(kind.folder());
        let name = source.file_name()?.to_string_lossy().into_owned();
        let copied = std::fs::create_dir_all(&dir).and_then(|()| {
            let path = Self::free(&dir, &name);
            std::fs::copy(source, &path).map(|_| path)
        });
        match copied {
            Ok(path) => {
                if kind == BrandFiles::Fonts {
                    self.register();
                }
                Some(path)
            }
            Err(error) => {
                eprintln!("brand file {}: {error}", source.display());
                None
            }
        }
    }

    fn remove_file(&self, path: &Path) {
        // Only what the store copied, inside its folder.
        if !path.starts_with(&self.dir) {
            return;
        }
        if let Err(error) = std::fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!("brand file {}: {error}", path.display());
        }
        if path.parent().and_then(Path::file_name) == Some(BrandFiles::Fonts.folder().as_ref()) {
            self.register();
        }
    }

    fn active(&self) -> Option<String> {
        self.read()["active"].as_str().map(str::to_owned)
    }

    fn set_active(&self, id: &str) {
        let mut value = self.read();
        value["active"] = Value::from(id);
        self.write(&value);
    }

    fn templates(&self, id: &str) -> Vec<gafira_ui::TemplateListing> {
        let dir = self.kit_dir(id).join("templates");
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "gaf"))
                    .collect()
            })
            .unwrap_or_default();
        paths.sort();
        paths
            .into_iter()
            .filter_map(|path| {
                let info = std::fs::File::open(&path)
                    .map_err(RepositoryError::Io)
                    .and_then(|f| read_template_info(BufReader::new(f), Limits::default()))
                    .ok()??;
                Some(gafira_ui::TemplateListing {
                    path,
                    meta: info.meta,
                })
            })
            .collect()
    }

    fn add_template(
        &self,
        id: &str,
        source: &Path,
        meta: &TemplateMeta,
    ) -> Result<PathBuf, String> {
        let dir = self.kit_dir(id).join("templates");
        let bytes = std::fs::read(source).map_err(|e| e.to_string())?;
        let mut out = Cursor::new(Vec::new());
        set_template(Cursor::new(bytes), &mut out, meta, Limits::default())
            .map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = Self::free(&dir, "template.gaf");
        std::fs::write(&path, out.into_inner()).map_err(|e| e.to_string())?;
        Ok(path)
    }

    fn remove_template(&self, path: &Path) {
        if path.starts_with(&self.dir)
            && let Err(error) = std::fs::remove_file(path)
        {
            eprintln!("brand template {}: {error}", path.display());
        }
    }

    fn thumbnail(&self, path: &Path) -> Option<gafira_ui::RecentThumbnail> {
        let thumb = std::fs::File::open(path)
            .map_err(RepositoryError::Io)
            .and_then(|f| read_thumbnail(BufReader::new(f), Limits::default()))
            .ok()??;
        Some(gafira_ui::RecentThumbnail {
            width: thumb.width,
            height: thumb.height,
            rgba: thumb.rgba,
        })
    }

    fn export_kit(&self, id: &str, dest: &Path) -> Result<(), KitError> {
        use std::io::Write;
        let mut kit = gafira_ui::BrandStore::kits(self)
            .into_iter()
            .find(|k| k.id == id)
            .ok_or_else(|| io("unknown kit"))?;
        // Written beside the destination, then moved over it: a failed
        // export leaves no half file.
        let partial = dest.with_extension("gafkit.part");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&partial).map_err(io)?);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut taken = std::collections::BTreeSet::new();
        let mut add = |zip: &mut zip::ZipWriter<std::fs::File>,
                       folder: &str,
                       path: &Path|
         -> Result<Option<String>, KitError> {
            let Ok(bytes) = std::fs::read(path) else {
                return Ok(None);
            };
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            let Some(name) = name.filter(|n| safe_entry(&format!("{folder}/{n}")).is_some()) else {
                return Ok(None);
            };
            let (stem, ext) = name.rsplit_once('.').unwrap_or((&name, ""));
            let entry = (1..)
                .map(|n| match n {
                    1 => format!("{folder}/{name}"),
                    _ => format!("{folder}/{stem}-{n}.{ext}"),
                })
                .find(|e| !taken.contains(e))
                .unwrap_or_default();
            taken.insert(entry.clone());
            zip.start_file(entry.as_str(), options).map_err(io)?;
            zip.write_all(&bytes).map_err(io)?;
            Ok(Some(entry))
        };
        for kind in BrandFiles::ALL {
            let mut kept = Vec::new();
            for path in kit.files(kind).clone() {
                if let Some(entry) = add(&mut zip, kind.folder(), &path)? {
                    kept.push(PathBuf::from(entry));
                }
            }
            *kit.files_mut(kind) = kept;
        }
        for template in gafira_ui::BrandStore::templates(self, id) {
            add(&mut zip, TEMPLATES, &template.path)?;
        }
        let manifest = json!({"format": KIT_FORMAT, "version": KIT_VERSION, "kit": kit_json(&kit)});
        zip.start_file("kit.json", options).map_err(io)?;
        zip.write_all(manifest.to_string().as_bytes()).map_err(io)?;
        zip.finish().map_err(io)?;
        std::fs::rename(&partial, dest).map_err(|e| {
            let _ = std::fs::remove_file(&partial);
            io(e)
        })
    }

    fn peek_kit(&self, source: &Path) -> Result<String, KitError> {
        let value = package_kit(&mut open_package(source)?)?;
        Ok(value["kit"]["name"].as_str().unwrap_or_default().to_owned())
    }

    fn import_kit(&self, source: &Path, replace: Option<&str>) -> Result<String, KitError> {
        use std::io::Read;
        let mut archive = open_package(source)?;
        let manifest = package_kit(&mut archive)?;
        let kits = gafira_ui::BrandStore::kits(self);
        let id = match replace {
            Some(id) if kits.iter().any(|k| k.id == id) => id.to_owned(),
            _ => gafira_ui::new_kit_id(&kits),
        };
        let mut stored = manifest["kit"].clone();
        stored["id"] = Value::from(id.as_str());
        let mut kit = kit_of(&stored).ok_or(KitError::NotAKit)?;
        // Unpacked apart first; the kit's folder changes only once all of
        // it is there.
        let staging = self.dir.join(".import");
        let _ = std::fs::remove_dir_all(&staging);
        let mut unpacked = std::collections::BTreeSet::new();
        let mut budget = KIT_MOST_BYTES;
        let result = (|| -> Result<(), KitError> {
            for index in 0..archive.len() {
                let entry = archive.by_index(index).map_err(|_| KitError::NotAKit)?;
                let name = entry.name().map(|n| n.into_owned()).unwrap_or_default();
                let Some((folder, file)) = safe_entry(&name) else {
                    continue;
                };
                if !entry.is_file() {
                    continue;
                }
                let dir = staging.join(folder);
                std::fs::create_dir_all(&dir).map_err(io)?;
                let mut bytes = Vec::new();
                entry.take(budget + 1).read_to_end(&mut bytes).map_err(io)?;
                budget = budget
                    .checked_sub(bytes.len() as u64)
                    .ok_or_else(|| io("the kit is too large"))?;
                std::fs::write(dir.join(file), bytes).map_err(io)?;
                unpacked.insert(format!("{folder}/{file}"));
            }
            Ok(())
        })();
        if let Err(error) = result {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(error);
        }
        let home = self.kit_dir(&id);
        if home.exists() {
            std::fs::remove_dir_all(&home).map_err(io)?;
        }
        std::fs::create_dir_all(&self.dir).map_err(io)?;
        if staging.exists() {
            std::fs::rename(&staging, &home).map_err(io)?;
        }
        for kind in BrandFiles::ALL {
            let files = kit
                .files(kind)
                .iter()
                .filter(|rel| unpacked.contains(&rel.to_string_lossy().into_owned()))
                .map(|rel| home.join(rel))
                .collect();
            *kit.files_mut(kind) = files;
        }
        // Its templates name the kit they now belong to.
        for template in gafira_ui::BrandStore::templates(self, &id) {
            let mut meta = template.meta.clone();
            meta.tags.retain(|t| !t.starts_with(gafira_ui::BRAND_TAG));
            meta.tags.insert(0, format!("{}{id}", gafira_ui::BRAND_TAG));
            let renamed = std::fs::read(&template.path).ok().and_then(|bytes| {
                let mut out = Cursor::new(Vec::new());
                set_template(Cursor::new(bytes), &mut out, &meta, Limits::default()).ok()?;
                Some(out.into_inner())
            });
            if let Some(bytes) = renamed {
                let _ = std::fs::write(&template.path, bytes);
            }
        }
        gafira_ui::BrandStore::save(self, &kit);
        self.register();
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use gafira_ui::BrandStore;

    use super::*;

    fn temp() -> PathBuf {
        // One folder per call: the tests run side by side in one process.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("gafira-brand-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn s93_02_kits_are_kept_and_read_back_the_same() {
        let dir = temp();
        let store = Brand::new(dir.clone(), gafira_ui::SystemFonts::new());
        assert!(store.kits().is_empty());
        let mut kit = BrandKit::new("kit-1".to_owned(), "Café".to_owned());
        kit.palettes.push(BrandPalette {
            name: "Principal".to_owned(),
            colors: vec![
                BrandColor::rgb(0x3B2A20),
                BrandColor {
                    rgb: 0xF6EEE3,
                    cmyk: Some([0, 3, 8, 4]),
                },
            ],
        });
        kit.styles[0].italic = true;
        kit.styles.remove(2);
        store.save(&kit);
        store.save(&BrandKit::new("kit-2".to_owned(), "Otra".to_owned()));
        assert_eq!(
            store.kits(),
            vec![
                kit.clone(),
                BrandKit::new("kit-2".to_owned(), "Otra".to_owned())
            ]
        );
        kit.name = "Café de barrio".to_owned();
        store.save(&kit);
        assert_eq!(store.kits()[0].name, "Café de barrio", "saved in place");
        store.set_active("kit-2");
        assert_eq!(store.active().as_deref(), Some("kit-2"));
        store.delete("kit-2");
        assert_eq!(store.kits().len(), 1);
        assert_eq!(store.active(), None, "the deleted kit is no longer in use");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn s93_03_files_are_copied_into_the_kit() {
        let dir = temp().with_extension("files");
        let _ = std::fs::remove_dir_all(&dir);
        let store = Brand::new(dir.clone(), gafira_ui::SystemFonts::new());
        std::fs::create_dir_all(&dir).expect("a folder");
        let source = dir.join("logo.svg");
        std::fs::write(&source, b"<svg/>").expect("writes");
        let first = store
            .add_file("kit-1", BrandFiles::Logos, &source)
            .expect("copied");
        let second = store
            .add_file("kit-1", BrandFiles::Logos, &source)
            .expect("copied");
        assert_eq!(first, dir.join("kit-1").join("logos").join("logo.svg"));
        assert_ne!(first, second, "a second copy does not overwrite the first");
        store.delete("kit-1");
        assert!(!dir.join("kit-1").exists(), "its files go with the kit");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn s93_03_an_uploaded_font_is_installed_and_uninstalled() {
        let dir = temp().with_extension("fonts");
        let _ = std::fs::remove_dir_all(&dir);
        let fonts = gafira_ui::SystemFonts::new();
        let store = Brand::new(dir.clone(), fonts.clone());
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fonts/manual/Inter-SemiBold.ttf");
        let copied = store
            .add_file("kit-1", BrandFiles::Fonts, &source)
            .expect("copied");
        assert!(
            fonts.added().iter().any(|f| f.path == copied),
            "an uploaded font is in Gafira's fonts"
        );
        store.remove_file(&copied);
        assert!(!copied.exists(), "its file is deleted");
        assert!(fonts.added().is_empty(), "and it leaves Gafira's fonts");
        // A path outside the brand folder is never deleted.
        store.remove_file(&source);
        assert!(source.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A store over a fresh folder named after `name`, and its fonts.
    fn store_at(name: &str) -> (Brand, PathBuf, gafira_ui::SystemFonts) {
        let dir = temp().with_extension(name);
        let _ = std::fs::remove_dir_all(&dir);
        let fonts = gafira_ui::SystemFonts::new();
        (Brand::new(dir.clone(), fonts.clone()), dir, fonts)
    }

    fn repo(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }

    /// A kit «Café» with a palette, a logo, a font and a brand template.
    fn full_kit(store: &Brand, dir: &Path) -> BrandKit {
        std::fs::create_dir_all(dir).expect("a folder");
        let logo = dir.join("logo.svg");
        std::fs::write(&logo, b"<svg xmlns='http://www.w3.org/2000/svg'/>").expect("writes");
        let mut kit = BrandKit::new("kit-1".to_owned(), "Café".to_owned());
        kit.palettes.push(BrandPalette {
            name: "Principal".to_owned(),
            colors: vec![BrandColor {
                rgb: 0xFF6B57,
                cmyk: Some([0, 58, 66, 0]),
            }],
        });
        kit.styles[0].font = "Inter".to_owned();
        kit.logos.push(
            store
                .add_file("kit-1", BrandFiles::Logos, &logo)
                .expect("logo"),
        );
        let font = repo("assets/fonts/manual/Inter-SemiBold.ttf");
        kit.fonts.push(
            store
                .add_file("kit-1", BrandFiles::Fonts, &font)
                .expect("font"),
        );
        store.save(&kit);
        let template = repo("assets/templates/documents/resume-columns.es.gaf");
        let meta = gafira_ui::brand_template_meta("kit-1", "CV", None);
        store
            .add_template("kit-1", &template, &meta)
            .expect("template");
        kit
    }

    #[test]
    fn s94_01_02_a_kit_exported_and_imported_comes_back_whole() {
        let (from, from_dir, _) = store_at("export");
        let kit = full_kit(&from, &from_dir);
        let package = from_dir.join("Café.gafkit");
        from.export_kit("kit-1", &package).expect("exported");
        assert_eq!(from.peek_kit(&package).expect("a kit"), "Café");

        let (to, to_dir, fonts) = store_at("import");
        let other = BrandKit::new("kit-1".to_owned(), "Otra".to_owned());
        to.save(&other);
        let id = to.import_kit(&package, None).expect("imported");
        assert_eq!(id, "kit-2", "a free id");
        let got = to.kits().into_iter().find(|k| k.id == id).expect("the kit");
        assert_eq!(got.name, kit.name);
        assert_eq!(got.palettes, kit.palettes);
        assert_eq!(got.styles, kit.styles);
        assert_eq!(got.logos.len(), 1);
        assert!(got.logos[0].starts_with(to_dir.join("kit-2")) && got.logos[0].is_file());
        assert_eq!(got.fonts.len(), 1);
        assert!(
            fonts.added().iter().any(|f| f.path == got.fonts[0]),
            "its fonts are installed"
        );
        let templates = to.templates(&id);
        assert_eq!(templates.len(), 1);
        assert!(
            templates[0].meta.tags.contains(&format!("brand:{id}")),
            "its templates name the new kit"
        );
        let _ = std::fs::remove_dir_all(from_dir);
        let _ = std::fs::remove_dir_all(to_dir);
    }

    #[test]
    fn s94_03_importing_over_a_kit_replaces_it() {
        let (store, dir, _) = store_at("replace");
        full_kit(&store, &dir);
        let package = dir.join("kit.gafkit");
        store.export_kit("kit-1", &package).expect("exported");
        // The kit changes, then the package goes over it.
        let mut kit = store.kits().remove(0);
        kit.palettes.clear();
        let extra = dir.join("photo.png");
        std::fs::write(&extra, b"png").expect("writes");
        let photo = store
            .add_file("kit-1", BrandFiles::Photos, &extra)
            .expect("photo");
        kit.photos.push(photo.clone());
        store.save(&kit);
        let id = store.import_kit(&package, Some("kit-1")).expect("imported");
        assert_eq!(id, "kit-1");
        let kits = store.kits();
        assert_eq!(kits.len(), 1, "replaced, not added");
        assert_eq!(kits[0].palettes.len(), 1);
        assert!(kits[0].photos.is_empty());
        assert!(!photo.exists(), "its former files are gone");
        assert!(kits[0].logos[0].is_file());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn s94_05_unsafe_entries_are_ignored_and_other_files_refused() {
        use std::io::Write;
        let (store, dir, _) = store_at("unsafe");
        std::fs::create_dir_all(&dir).expect("a folder");
        let package = dir.join("evil.gafkit");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&package).expect("creates"));
        let options = zip::write::SimpleFileOptions::default();
        let kit = json!({"format": "gafira-brandkit", "version": 1, "kit": {
            "id": "x", "name": "Evil",
            "files": {"logos": ["logos/ok.svg", "../escape.svg", "/etc/passwd"]}
        }});
        for (name, bytes) in [
            ("kit.json", kit.to_string().into_bytes()),
            ("logos/ok.svg", b"<svg/>".to_vec()),
            ("../escape.svg", b"x".to_vec()),
            ("logos/../../escape2.svg", b"x".to_vec()),
            ("secrets/key.txt", b"x".to_vec()),
        ] {
            zip.start_file(name, options).expect("entry");
            zip.write_all(&bytes).expect("writes");
        }
        zip.finish().expect("zip");
        let id = store.import_kit(&package, None).expect("imported");
        let kit = store.kits().remove(0);
        assert_eq!(kit.logos, [dir.join(&id).join("logos").join("ok.svg")]);
        assert!(
            !dir.join("escape.svg").exists() && !dir.parent().unwrap().join("escape.svg").exists()
        );
        assert!(!dir.join(&id).join("secrets").exists());

        let not_kit = dir.join("plain.gafkit");
        std::fs::write(&not_kit, b"hello").expect("writes");
        assert!(store.peek_kit(&not_kit).is_err());
        assert!(store.import_kit(&not_kit, None).is_err());
        assert_eq!(store.kits().len(), 1, "nothing added");
        let _ = std::fs::remove_dir_all(dir);
    }
}
