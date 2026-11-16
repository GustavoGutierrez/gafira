//! The fonts the graph lays text out with (02 §4.6, spec 030).
//!
//! The crate reads no file (ARQ-02): the composition root hands the font
//! bytes over, by family name.

use std::sync::Arc;

/// Font files by family name, in the order they were added.
///
/// A text asks for the family of its style; the book answers that family
/// first and every other one after it, as the fallback stack of
/// `gafira_text::layout_box`. A family the book does not hold falls back to
/// the first one added.
#[derive(Clone, Debug, Default)]
pub struct FontBook {
    faces: Vec<(String, Arc<[u8]>)>,
}

impl FontBook {
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

    /// The font stack for `family`: its file first, then the others in the
    /// order they were added. Empty when the book is.
    #[must_use]
    pub fn stack(&self, family: &str) -> Vec<&[u8]> {
        let first = self
            .faces
            .iter()
            .position(|(name, _)| name == family)
            .unwrap_or(0);
        let mut stack = Vec::with_capacity(self.faces.len());
        if let Some((_, bytes)) = self.faces.get(first) {
            stack.push(bytes.as_ref());
        }
        for (at, (_, bytes)) in self.faces.iter().enumerate() {
            if at != first {
                stack.push(bytes.as_ref());
            }
        }
        stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // S30-01: the family asked for leads the stack; an unknown one falls back
    // to the first font added.
    #[test]
    fn s30_01_the_stack_leads_with_the_family_asked_for() {
        let book = FontBook::new()
            .with("Inter", vec![1u8])
            .with("Manrope", vec![2u8]);
        assert_eq!(book.stack("Manrope"), [&[2u8][..], &[1u8][..]]);
        assert_eq!(book.stack("Inter"), [&[1u8][..], &[2u8][..]]);
        assert_eq!(book.stack("Roboto"), [&[1u8][..], &[2u8][..]]);
        let book = book.with("Inter", vec![3u8]);
        assert_eq!(book.stack("Inter"), [&[3u8][..], &[2u8][..]]);
        assert!(FontBook::new().stack("Inter").is_empty());
        assert!(FontBook::new().is_empty());
    }
}
