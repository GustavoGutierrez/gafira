//! The fonts installed on the system (spec 062): the catalogue the app
//! fills in the background, what the Fuente panel lists apart from the
//! fonts of Gafira, and the files a text in one of them is drawn from.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

/// One face of a font installed on the system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemFont {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    pub path: PathBuf,
}

/// The fonts of the system, shared and filled once the scan ends; empty and
/// not ready until then. The fonts a person uploads to a brand kit (spec
/// 093) are added apart, so the scan never takes them away.
#[derive(Clone, Default)]
pub struct SystemFonts(
    Arc<RwLock<Option<Arc<Vec<SystemFont>>>>>,
    Arc<RwLock<Vec<SystemFont>>>,
);

impl SystemFonts {
    /// An empty catalogue, not ready.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The faces the scan found.
    pub fn set(&self, faces: Vec<SystemFont>) {
        if let Ok(mut slot) = self.0.write() {
            *slot = Some(Arc::new(faces));
        }
    }

    /// Whether the scan has ended.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.0.read().is_ok_and(|slot| slot.is_some())
    }

    /// Every face: those the person added, then those the scan found.
    #[must_use]
    pub fn faces(&self) -> Arc<Vec<SystemFont>> {
        let scanned = self
            .0
            .read()
            .ok()
            .and_then(|slot| slot.clone())
            .unwrap_or_default();
        let added = self.added();
        if added.is_empty() {
            return scanned;
        }
        Arc::new(added.into_iter().chain(scanned.iter().cloned()).collect())
    }

    /// Add faces the person uploaded (spec 093); a file already added is
    /// added once.
    pub fn add(&self, faces: Vec<SystemFont>) {
        if let Ok(mut added) = self.1.write() {
            for face in faces {
                if !added.iter().any(|f| f.path == face.path) {
                    added.push(face);
                }
            }
        }
    }

    /// The faces the person uploaded, in the order they came.
    #[must_use]
    pub fn added(&self) -> Vec<SystemFont> {
        self.1.read().map(|a| a.clone()).unwrap_or_default()
    }

    /// The families of the system, each once, in order, but those of
    /// `skip` (the fonts of Gafira win over the same name).
    #[must_use]
    pub fn families(&self, skip: &[&str]) -> Vec<String> {
        let mut out: Vec<String> = self
            .faces()
            .iter()
            .map(|face| face.family.clone())
            .filter(|family| !skip.contains(&family.as_str()))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The file of the face named `key` (`gafira_theme::face_key`).
    #[must_use]
    pub fn path(&self, key: &str) -> Option<PathBuf> {
        self.faces()
            .iter()
            .find(|face| gafira_theme::face_key(&face.family, face.bold, face.italic) == key)
            .map(|face| face.path.clone())
    }
}

/// The catalogue of this run, set once by `run`.
static CATALOGUE: OnceLock<SystemFonts> = OnceLock::new();

/// Keep `fonts` as the catalogue of this run.
pub(crate) fn install(fonts: SystemFonts) {
    let _ = CATALOGUE.set(fonts);
}

/// The catalogue of this run, or an empty one (a test).
pub(crate) fn catalogue() -> SystemFonts {
    CATALOGUE.get().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(family: &str, bold: bool, italic: bool) -> SystemFont {
        SystemFont {
            family: family.to_owned(),
            bold,
            italic,
            path: PathBuf::from(format!("/fonts/{family}-{bold}-{italic}.ttf")),
        }
    }

    // S62-05: the catalogue is empty until the scan ends; then it lists its
    // families once each, sorted, without those of Gafira, and finds the
    // file of a face by its name.
    #[test]
    fn s62_05_system_catalogue() {
        let fonts = SystemFonts::new();
        assert!(!fonts.ready() && fonts.families(&[]).is_empty());
        fonts.set(vec![
            face("Ubuntu", false, false),
            face("Ubuntu", true, false),
            face("Roboto", false, false),
            face("Cantarell", false, true),
        ]);
        assert!(fonts.ready());
        assert_eq!(fonts.families(&["Roboto"]), ["Cantarell", "Ubuntu"]);
        assert_eq!(
            fonts.path("Ubuntu Bold"),
            Some(PathBuf::from("/fonts/Ubuntu-true-false.ttf"))
        );
        assert_eq!(
            fonts.path("Cantarell Italic"),
            Some(PathBuf::from("/fonts/Cantarell-false-true.ttf"))
        );
        assert_eq!(fonts.path("Ubuntu Italic"), None);
        assert_eq!(fonts.faces().len(), 4);
        // Spec 093: a font uploaded to a brand kit joins the catalogue apart
        // from the scan, once per file.
        fonts.add(vec![face("Marca Sans", false, false)]);
        fonts.add(vec![face("Marca Sans", false, false)]);
        assert_eq!(fonts.added().len(), 1, "a file is added once");
        assert!(fonts.families(&[]).contains(&"Marca Sans".to_owned()));
        assert!(fonts.path("Marca Sans").is_some());
    }
}
