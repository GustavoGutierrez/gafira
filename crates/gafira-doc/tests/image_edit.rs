//! What an image was made from (spec 084, S84-01).

use std::sync::Arc;

use gafira_doc::{
    AssetEntry, AssetRef, AssetStorage, AssetTable, ContentHash, Document, Dpi, ElementBase,
    ImageEdit, ImageNode, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement,
    ValidationError,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);

fn asset(n: u8) -> AssetRef {
    AssetRef::new(ContentHash([n; 32]))
}

fn document(image: ImageNode, known: &[u8]) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(400.0, Unit::Px),
        height: Length::new(400.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let base = ElementBase::new(NODE, Placement::new(10.0, 10.0, 320.0, 90.0));
    page.nodes
        .insert(NODE, Arc::new(Node::new(base, NodeKind::Image(image))));
    page.roots = vec![NODE];
    let entry = AssetEntry {
        name: "a.png".into(),
        storage: AssetStorage::Embedded,
    };
    let mut table = AssetTable::new();
    for n in known {
        table.insert(asset(*n), entry.clone());
    }
    Document::from_parts_with_assets(3, Dpi::new(96), vec![Arc::new(page)], table)
}

// S84-01: an image remembers where it comes from, and goes back to it.
#[test]
fn s84_01_the_original_is_kept() {
    let plain = ImageNode::new(asset(1));
    assert_eq!(
        (
            plain.original(),
            plain.base(),
            plain.is_cut(),
            plain.is_enhanced()
        ),
        (asset(1), asset(1), false, false)
    );
    let better = plain.edited(asset(2), asset(2));
    assert_eq!(
        (better.original(), better.is_cut(), better.is_enhanced()),
        (asset(1), false, true)
    );
    let cut = better.edited(asset(3), asset(2));
    assert_eq!(
        cut.edit,
        Some(ImageEdit {
            original: asset(1),
            base: asset(2)
        })
    );
    assert!(cut.is_cut() && cut.is_enhanced());
    assert_eq!(
        cut.assets().collect::<Vec<_>>(),
        [asset(3), asset(1), asset(2)]
    );
    assert_eq!(
        cut.edited(asset(1), asset(1)),
        plain,
        "back to the original"
    );
}

// S84-01: it round-trips; an image without edits is written as before; every
// asset it needs is in the table.
#[test]
fn s84_01_the_document_keeps_it() {
    let cut = ImageNode::new(asset(1)).edited(asset(3), asset(2));
    let doc = document(cut, &[1, 2, 3]).expect("valid");
    let json = doc.to_json().expect("json");
    assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
    let plain = document(ImageNode::new(asset(1)), &[1])
        .expect("valid")
        .to_json()
        .expect("json");
    assert!(!plain.contains("\"edit\""), "{plain}");
    for known in [&[1, 2][..], &[1, 3], &[2, 3]] {
        assert_eq!(
            document(cut, known),
            Err(ValidationError::DanglingImage { node: NODE }),
            "{known:?}"
        );
    }
}
