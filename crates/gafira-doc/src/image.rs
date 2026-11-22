//! The image element of 02 §3.1 (`ImageNode`), first slice (spec 029): the
//! asset it shows, by content hash. Cropping and adjustments join schema 1
//! later as additive fields (ADR 0006).

use serde::{Deserialize, Serialize};

use crate::timeline::AssetRef;

/// An image element: an embedded or linked asset drawn in the element's box.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageNode {
    /// The asset table entry the image shows; it must exist (P7).
    pub asset: AssetRef,
    /// What the asset was made from (spec 084). Additive to schema 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<ImageEdit>,
    /// How it is made to look (spec 085). Additive to schema 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<Box<crate::ImageLook>>,
    /// The part of it that shows (spec 088). Additive to schema 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Box<crate::ImageCrop>>,
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
        Self {
            asset,
            edit: None,
            look: None,
            crop: None,
        }
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
        Self {
            asset,
            edit,
            look: self.look.clone(),
            crop: self.crop.clone(),
        }
    }

    /// Every asset it needs.
    pub fn assets(&self) -> impl Iterator<Item = AssetRef> + '_ {
        let edit = self.edit.map(|edit| [edit.original, edit.base]);
        let masks = self.look.iter().flat_map(|look| &look.zones);
        std::iter::once(self.asset)
            .chain(edit.into_iter().flatten())
            .chain(masks.map(|zone| zone.mask))
    }

    /// The image with `look`; none when there is nothing to it.
    #[must_use]
    pub fn looking(&self, look: crate::ImageLook) -> Self {
        Self {
            asset: self.asset,
            edit: self.edit,
            look: (!look.is_plain()).then(|| Box::new(look)),
            crop: self.crop.clone(),
        }
    }

    /// The image with the part `crop` of it; all of it when that is all.
    #[must_use]
    pub fn cropped(&self, crop: crate::ImageCrop) -> Self {
        let mut image = self.clone();
        image.crop = (!crop.is_whole()).then(|| Box::new(crop));
        image
    }

    /// The part of it that shows.
    #[must_use]
    pub fn part(&self) -> crate::ImageCrop {
        self.crop.as_deref().cloned().unwrap_or_default()
    }

    /// Whether its pixels show as they are.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        self.look.is_none() && self.crop.is_none()
    }

    /// What its pixels are kept under once decoded: its asset, or, with a
    /// look or a crop, one of that asset and them.
    #[must_use]
    pub fn shown(&self) -> AssetRef {
        if self.is_plain() {
            return self.asset;
        }
        let look = self.look.as_deref().cloned().unwrap_or_default();
        look.key(self.asset, self.crop.as_deref())
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
