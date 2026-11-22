//! The image element of 02 §3.1 (`ImageNode`), first slice (spec 029): the
//! asset it shows, by content hash. Cropping and adjustments join schema 1
//! later as additive fields (ADR 0006).

use serde::{Deserialize, Serialize};

use crate::timeline::AssetRef;

/// An image element: an embedded or linked asset drawn in the element's box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageNode {
    /// The asset table entry the image shows; it must exist (P7).
    pub asset: AssetRef,
    /// What the asset was made from (spec 084). Additive to schema 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<ImageEdit>,
}

/// What an image was before it was changed (spec 084): nothing is lost, so
/// it can always go back. Both are entries of the asset table (P7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageEdit {
    /// The image as it was brought in.
    pub original: AssetRef,
    /// The whole image its pixels are taken from, before any was cut out:
    /// the original, or the original with a better resolution.
    pub base: AssetRef,
}

impl ImageNode {
    /// An image as it was brought in.
    #[must_use]
    pub const fn new(asset: AssetRef) -> Self {
        Self { asset, edit: None }
    }

    /// The image as it was brought in.
    #[must_use]
    pub fn original(&self) -> AssetRef {
        self.edit.map_or(self.asset, |edit| edit.original)
    }

    /// The whole image its pixels are taken from.
    #[must_use]
    pub fn base(&self) -> AssetRef {
        self.edit.map_or(self.asset, |edit| edit.base)
    }

    /// Whether some pixels were cut out of it.
    #[must_use]
    pub fn is_cut(&self) -> bool {
        self.edit.is_some_and(|edit| edit.base != self.asset)
    }

    /// Whether its resolution was made better.
    #[must_use]
    pub fn is_enhanced(&self) -> bool {
        self.edit.is_some_and(|edit| edit.base != edit.original)
    }

    /// The image showing `asset`, made from `base`; nothing to go back to
    /// when both are the original.
    #[must_use]
    pub fn edited(&self, asset: AssetRef, base: AssetRef) -> Self {
        let original = self.original();
        let edit = (asset != original || base != original).then_some(ImageEdit { original, base });
        Self { asset, edit }
    }

    /// Every asset it needs.
    pub fn assets(&self) -> impl Iterator<Item = AssetRef> {
        let edit = self.edit.map(|edit| [edit.original, edit.base]);
        std::iter::once(self.asset).chain(edit.into_iter().flatten())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContentHash;

    #[test]
    fn s29_01_the_wire_form_names_the_asset_by_hash() {
        let node = ImageNode::new(AssetRef::new(ContentHash([0xab; 32])));
        let json = serde_json::to_string(&node).expect("writes");
        assert_eq!(json, format!(r#"{{"asset":"{}"}}"#, "ab".repeat(32)));
        assert_eq!(
            serde_json::from_str::<ImageNode>(&json).expect("reads"),
            node
        );
    }
}
