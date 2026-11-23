//! The fonts installed on the system (spec 062): where they live and, for
//! each file, its family and whether it is bold or italic. A collection
//! (`.ttc`) is left out: the text engine reads one font per file.

use std::path::{Path, PathBuf};

use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider};

/// One face of a font installed on the system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemFace {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    pub path: PathBuf,
}

/// The folders the system keeps its fonts in, and the user's own.
#[must_use]
pub fn font_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut dirs = Vec::new();
    if cfg!(target_os = "linux") {
        dirs.extend(["/usr/share/fonts", "/usr/local/share/fonts"].map(PathBuf::from));
        if let Some(home) = &home {
            dirs.push(home.join(".local/share/fonts"));
            dirs.push(home.join(".fonts"));
        }
    } else if cfg!(target_os = "macos") {
        dirs.extend(["/System/Library/Fonts", "/Library/Fonts"].map(PathBuf::from));
        if let Some(home) = &home {
            dirs.push(home.join("Library/Fonts"));
        }
    } else if cfg!(target_os = "windows") {
        if let Some(windir) = std::env::var_os("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
        }
    }
    dirs
}

/// The face of the font `bytes` read from `path`; `None` when it is not a
/// font or has no family name. Bold is a weight of 600 or more.
#[must_use]
pub fn face_of(bytes: &[u8], path: &Path) -> Option<SystemFace> {
    face_and_weight(bytes, path).map(|(face, _)| face)
}

/// The face of `bytes` and its weight.
fn face_and_weight(bytes: &[u8], path: &Path) -> Option<(SystemFace, f32)> {
    let font = FontRef::new(bytes).ok()?;
    let name = |id| {
        font.localized_strings(id)
            .english_or_first()
            .map(|s| s.chars().collect::<String>())
            .filter(|s| !s.trim().is_empty())
    };
    let family = name(StringId::TYPOGRAPHIC_FAMILY_NAME).or_else(|| name(StringId::FAMILY_NAME))?;
    let attributes = font.attributes();
    let weight = attributes.weight.value();
    let italic = !matches!(attributes.style, skrifa::attribute::Style::Normal);
    Some((
        SystemFace {
            family,
            bold: weight >= 600.0,
            italic,
            path: path.to_owned(),
        },
        weight,
    ))
}

/// Every face of the `.ttf` and `.otf` files under `dirs`, one per family,
/// weight and style: for each family, the face nearest to a regular (400)
/// and a bold (700) weight, upright and italic. Sorted by family.
#[must_use]
pub fn scan_fonts(dirs: &[PathBuf]) -> Vec<SystemFace> {
    let mut files = Vec::new();
    for dir in dirs {
        collect(dir, &mut files, 0);
    }
    // For each family and slot, the face whose weight is nearest to the
    // slot's: 400 for regular, 700 for bold.
    let mut best: std::collections::BTreeMap<(String, bool, bool), (SystemFace, f32)> =
        std::collections::BTreeMap::new();
    for path in files {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Some((face, weight)) = face_and_weight(&bytes, &path) else {
            continue;
        };
        let target = if face.bold { 700.0 } else { 400.0 };
        let key = (face.family.clone(), face.bold, face.italic);
        let closer = best
            .get(&key)
            .is_none_or(|(_, w)| (weight - target).abs() < (w - target).abs());
        if closer {
            best.insert(key, (face, weight));
        }
    }
    best.into_values().map(|(face, _)| face).collect()
}

/// The `.ttf` and `.otf` files under `dir`, folders included, down to a
/// depth that no font folder needs more than.
fn collect(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out, depth + 1);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf"))
        {
            out.push(path);
        }
    }
}

/// The faces installed on the system.
#[must_use]
pub fn system_fonts() -> Vec<SystemFace> {
    scan_fonts(&font_dirs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fonts/library")
            .join(path)
    }

    // S62-05: a font file gives its family and whether it is bold or
    // italic; something else gives nothing.
    #[test]
    fn s62_05_face_of_a_file() {
        let bold = library("opensans/OpenSans-Bold.ttf");
        let face = face_of(&std::fs::read(&bold).expect("a font"), &bold).expect("a face");
        assert_eq!(
            (face.family.as_str(), face.bold, face.italic),
            ("Open Sans", true, false)
        );
        let italic = library("lato/Lato-Italic.ttf");
        let face = face_of(&std::fs::read(&italic).expect("a font"), &italic).expect("a face");
        assert_eq!(
            (face.family.as_str(), face.bold, face.italic),
            ("Lato", false, true)
        );
        assert_eq!(face_of(b"not a font", Path::new("x.ttf")), None);
    }

    // S62-05: a scan finds the fonts under the folders, nested too, keeps
    // one face per family, weight and style, and skips other files.
    #[test]
    fn s62_05_scan_fonts() {
        let dirs = [library("opensans"), library("lato"), library("anton")];
        let faces = scan_fonts(&dirs);
        let names: Vec<(&str, bool, bool)> = faces
            .iter()
            .map(|f| (f.family.as_str(), f.bold, f.italic))
            .collect();
        assert!(names.contains(&("Open Sans", false, false)));
        assert!(names.contains(&("Open Sans", true, true)));
        assert!(names.contains(&("Lato", true, false)));
        assert!(names.contains(&("Anton", false, false)));
        assert_eq!(faces.len(), 9, "{names:?}");
        assert!(
            faces.windows(2).all(|w| w[0].family <= w[1].family),
            "sorted"
        );
        assert!(scan_fonts(&[PathBuf::from("/no/such/folder")]).is_empty());
    }
}
