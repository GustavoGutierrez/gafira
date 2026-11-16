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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContentHash;

    #[test]
    fn s29_01_the_wire_form_names_the_asset_by_hash() {
        let node = ImageNode {
            asset: AssetRef::new(ContentHash([0xab; 32])),
        };
        let json = serde_json::to_string(&node).expect("writes");
        assert_eq!(json, format!(r#"{{"asset":"{}"}}"#, "ab".repeat(32)));
        assert_eq!(
            serde_json::from_str::<ImageNode>(&json).expect("reads"),
            node
        );
    }
}
