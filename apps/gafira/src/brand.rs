//! The brand kits of Marca (spec 093) in the data folder: `brand/kits.json`
//! holds every kit and the one in use; `brand/<kit>/<kind>/` its files and
//! `brand/<kit>/templates/` its brand templates, each a `.gaf` marked with
//! its `template` (D-81).

use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use gafira_format::{Limits, read_template_info, read_thumbnail, set_template};
use gafira_ports::repository::{RepositoryError, TemplateMeta};
use gafira_ui::{BrandColor, BrandFiles, BrandKit, BrandPalette, BrandTextStyle};
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

impl Brand {
    /// The store over `dir`, with the fonts of every kit added to `fonts`.
    pub(crate) fn new(dir: PathBuf, fonts: gafira_ui::SystemFonts) -> Self {
        let brand = Self { dir, fonts };
        let uploaded: Vec<PathBuf> = gafira_ui::BrandStore::kits(&brand)
            .into_iter()
            .flat_map(|kit| kit.fonts)
            .collect();
        brand.register(&uploaded);
        brand
    }

    /// Add the faces of the font files `paths` to the catalogue.
    fn register(&self, paths: &[PathBuf]) {
        let faces = paths
            .iter()
            .filter_map(|path| {
                let bytes = std::fs::read(path).ok()?;
                let face = gafira_platform::face_of(&bytes, path)?;
                Some(gafira_ui::SystemFont {
                    family: face.family,
                    bold: face.bold,
                    italic: face.italic,
                    path: face.path,
                })
            })
            .collect();
        self.fonts.add(faces);
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
                    self.register(std::slice::from_ref(&path));
                }
                Some(path)
            }
            Err(error) => {
                eprintln!("brand file {}: {error}", source.display());
                None
            }
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
}

#[cfg(test)]
mod tests {
    use gafira_ui::BrandStore;

    use super::*;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gafira-brand-{}", std::process::id()));
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
}
