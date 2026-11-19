//! The fonts the graph lays text out with (02 §4.6, spec 030).
//!
//! The crate reads no file (ARQ-02): the composition root hands the font
//! bytes over, by family name.

use std::sync::{Arc, Mutex};

/// Where the faces the book does not hold come from (spec 062): the
/// composition root reads the fonts of the system on demand, by face name.
pub type FontLoader = Arc<dyn Fn(&str) -> Option<Arc<[u8]>> + Send + Sync>;

/// A face name and what the loader gave for it.
type Loaded = (String, Option<Arc<[u8]>>);

/// Font files by family name, in the order they were added.
///
/// A text asks for the family of its style; the book answers that family
/// first and every other one after it, as the fallback stack of
/// `gafira_text::layout_box`. A family the book does not hold falls back to
/// the first one added.
#[derive(Clone, Default)]
pub struct FontBook {
    faces: Vec<(String, Arc<[u8]>)>,
    /// The loader of the faces the book lacks, and what it gave, once per
    /// name (spec 062).
    loader: Option<FontLoader>,
    loaded: Arc<Mutex<Vec<Loaded>>>,
}

impl std::fmt::Debug for FontBook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.faces.iter().map(|(name, _)| name.as_str()).collect();
        f.debug_struct("FontBook")
            .field("faces", &names)
            .field("loader", &self.loader.is_some())
            .finish()
    }
}

impl FontBook {
    /// The book with `loader` for the faces it does not hold (spec 062).
    #[must_use]
    pub fn with_loader(mut self, loader: FontLoader) -> Self {
        self.loader = Some(loader);
        self
    }

    /// The bytes of `family`: held, loaded before, or from the loader now.
    fn find(&self, family: &str) -> Option<Arc<[u8]>> {
        if let Some((_, bytes)) = self.faces.iter().find(|(name, _)| name == family) {
            return Some(Arc::clone(bytes));
        }
        let loader = self.loader.as_ref()?;
        let mut loaded = self
            .loaded
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, bytes)) = loaded.iter().find(|(name, _)| name == family) {
            return bytes.clone();
        }
        let bytes = loader(family);
        loaded.push((family.to_owned(), bytes.clone()));
        bytes
    }

    /// An empty book: no text draws.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The book with `bytes` added under `family`; a family added again
    /// replaces its bytes and keeps its place.
    #[must_use]
    pub fn with(mut self, family: &str, bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes = bytes.into();
        match self.faces.iter_mut().find(|(name, _)| name == family) {
            Some(face) => face.1 = bytes,
            None => self.faces.push((family.to_owned(), bytes)),
        }
        self
    }

    /// True when the book holds no font.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// True when the book holds a file under `family`.
    #[must_use]
    pub fn contains(&self, family: &str) -> bool {
        self.find(family).is_some()
    }

    /// The name of the face for `family` in `bold` and `italic` (spec 062):
    /// the face asked for when the book holds it, else the nearest one of
    /// the family, else the family name itself (the book then falls back).
    #[must_use]
    pub fn face(&self, family: &str, bold: bool, italic: bool) -> String {
        gafira_doc::nearest_face(
            |b, i| self.contains(&gafira_doc::face_key(family, b, i)),
            bold,
            italic,
        )
        .map_or_else(
            || family.to_owned(),
            |(b, i)| gafira_doc::face_key(family, b, i),
        )
    }

    /// The font stack for `family`: its file first, then the others in the
    /// order they were added. Empty when the book is.
    #[must_use]
    pub fn stack(&self, family: &str) -> Vec<Arc<[u8]>> {
        let lead = self
            .find(family)
            .or_else(|| self.faces.first().map(|f| Arc::clone(&f.1)));
        let mut stack = Vec::with_capacity(self.faces.len() + 1);
        if let Some(lead) = &lead {
            stack.push(Arc::clone(lead));
        }
        for (_, bytes) in &self.faces {
            if lead.as_ref().is_none_or(|lead| !Arc::ptr_eq(lead, bytes)) {
                stack.push(Arc::clone(bytes));
            }
        }
        stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // S62-05: a face the book lacks comes from its loader once, and leads
    // the stack; one the loader lacks falls back like an unknown family.
    #[test]
    fn s62_05_the_loader_fills_what_the_book_lacks() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        let book = FontBook::new()
            .with("Inter", vec![1u8])
            .with_loader(Arc::new(move |key: &str| {
                counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                (key == "Sistema Bold").then(|| Arc::from(vec![9u8]))
            }));
        assert!(book.contains("Sistema Bold"));
        assert_eq!(book.face("Sistema", true, false), "Sistema Bold");
        let stack = book.stack("Sistema Bold");
        assert_eq!(stack[0].as_ref(), [9u8]);
        assert_eq!(stack[1].as_ref(), [1u8]);
        assert!(!book.contains("Otra"));
        assert_eq!(book.stack("Otra")[0].as_ref(), [1u8]);
        let before = calls.load(std::sync::atomic::Ordering::SeqCst);
        let _ = book.stack("Sistema Bold");
        assert!(!book.contains("Otra"));
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            before,
            "loaded once"
        );
    }

    // S30-01: the family asked for leads the stack; an unknown one falls back
    // to the first font added.
    #[test]
    fn s30_01_the_stack_leads_with_the_family_asked_for() {
        let book = FontBook::new()
            .with("Inter", vec![1u8])
            .with("Manrope", vec![2u8]);
        let bytes = |stack: Vec<Arc<[u8]>>| stack.iter().map(|b| b.to_vec()).collect::<Vec<_>>();
        assert_eq!(bytes(book.stack("Manrope")), [vec![2u8], vec![1u8]]);
        assert_eq!(bytes(book.stack("Inter")), [vec![1u8], vec![2u8]]);
        assert_eq!(bytes(book.stack("Roboto")), [vec![1u8], vec![2u8]]);
        let book = book.with("Inter", vec![3u8]);
        assert_eq!(bytes(book.stack("Inter")), [vec![3u8], vec![2u8]]);
        assert!(FontBook::new().stack("Inter").is_empty());
        assert!(FontBook::new().is_empty());
    }
}
