//! Frames: a photo cut out by a shape (spec 066, FD-18, D-107).

use std::sync::Arc;

use gafira_doc::timeline::{AssetEntry, AssetRef, AssetStorage, AssetTable};
use gafira_doc::{
    ContentHash, Document, Dpi, ElementBase, Fit, FrameContent, FrameNode, Node, NodeId, NodeKind,
    Page, PageId, PageSize, Placement, ShapeGeometry, ValidationError,
};
use gafira_geom::{Length, Point, Unit};

const NODE: NodeId = NodeId::new(2);
const ASSET: AssetRef = AssetRef::new(ContentHash([7; 32]));

fn table() -> AssetTable {
    let mut assets = AssetTable::new();
    assets.insert(
        ASSET,
        AssetEntry {
            name: "photo.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    assets
}

fn document(frame: FrameNode, assets: AssetTable) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(400.0, Unit::Px),
        height: Length::new(400.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let node = Node::new(
        ElementBase::new(NODE, Placement::new(10.0, 10.0, 200.0, 100.0)),
        NodeKind::Frame(frame),
    );
    page.nodes.insert(NODE, Arc::new(node));
    page.roots = vec![NODE];
    Document::from_parts_with_assets(3, Dpi::new(96), vec![Arc::new(page)], assets)
}

fn filled() -> FrameNode {
    let mut frame = FrameNode::new(ShapeGeometry::Ellipse);
    frame.content = Some(FrameContent {
        asset: ASSET,
        natural: (400, 400),
    });
    frame
}

// S66-01: a frame round-trips, empty or with its photo.
#[test]
fn s66_01_round_trip() {
    for frame in [FrameNode::new(ShapeGeometry::Rect), filled()] {
        let doc = document(frame, table()).expect("valid");
        let json = doc.to_json().expect("json");
        assert!(json.contains(r#""type":"frame""#), "{json}");
        assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
    }
}

// S66-01: the scale is finite and above zero, the offset finite, and the
// photo is an asset of the table (P7).
#[test]
fn s66_01_validation_rejects_bad_content() {
    let refused = |frame: FrameNode, assets: AssetTable| {
        assert!(
            matches!(
                document(frame, assets),
                Err(ValidationError::InvalidFrame { node: NODE })
            ),
            "refused"
        );
    };
    let mut zero = filled();
    zero.content_scale = 0.0;
    refused(zero, table());
    let mut nan = filled();
    nan.content_offset = (f64::NAN, 0.0);
    refused(nan, table());
    refused(filled(), AssetTable::new());
    assert!(document(filled(), table()).is_ok());
}

/// Where the corners of the photo land in the box of the frame.
fn corners(frame: &FrameNode, w: f64, h: f64) -> (Point, Point) {
    let at = frame.content_transform(w, h).expect("a photo");
    let (nw, nh) = frame.content.expect("content").natural;
    (
        at * Point::new(0.0, 0.0),
        at * Point::new(f64::from(nw), f64::from(nh)),
    )
}

fn near(a: Point, x: f64, y: f64) -> bool {
    (a.x - x).abs() < 1e-9 && (a.y - y).abs() < 1e-9
}

// S66-03: «Rellenar» covers the box, the photo centred: a 400 × 400 photo
// in a 200 × 100 box is 200 × 200, from y = -50.
#[test]
fn s66_03_fill_covers() {
    let (a, b) = corners(&filled(), 200.0, 100.0);
    assert!(near(a, 0.0, -50.0) && near(b, 200.0, 150.0), "{a:?} {b:?}");
}

// S66-03: «Encajar» shows the whole photo: 100 × 100, centred.
#[test]
fn s66_03_fit_contains() {
    let mut frame = filled();
    frame.fit = Fit::Fit;
    let (a, b) = corners(&frame, 200.0, 100.0);
    assert!(near(a, 50.0, 0.0) && near(b, 150.0, 100.0), "{a:?} {b:?}");
}

// S66-03: «Estirar» fills the box exactly.
#[test]
fn s66_03_stretch_fills() {
    let mut frame = filled();
    frame.fit = Fit::Stretch;
    let (a, b) = corners(&frame, 200.0, 100.0);
    assert!(near(a, 0.0, 0.0) && near(b, 200.0, 100.0), "{a:?} {b:?}");
}

// S66-03: «Original» keeps the natural size, centred.
#[test]
fn s66_03_none_keeps_natural_size() {
    let mut frame = filled();
    frame.fit = Fit::None;
    let (a, b) = corners(&frame, 200.0, 100.0);
    assert!(
        near(a, -100.0, -150.0) && near(b, 300.0, 250.0),
        "{a:?} {b:?}"
    );
}

// S66-03: with «Rellenar», moving the photo stops where it would uncover
// the box; the other fits move freely.
#[test]
fn s66_03_fill_offset_is_clamped() {
    let mut frame = filled();
    frame.content_offset = (0.3, 0.9);
    // 200 × 200 in 200 × 100: no room across, 50 px (0.5 of H) down.
    let (a, b) = corners(&frame, 200.0, 100.0);
    assert!(near(a, 0.0, 0.0) && near(b, 200.0, 200.0), "{a:?} {b:?}");
    frame.content_scale = 2.0;
    frame.content_offset = (0.1, 0.0);
    let (a, _) = corners(&frame, 200.0, 100.0);
    assert!(near(a, -80.0, -150.0), "free within the room: {a:?}");
    frame.fit = Fit::Fit;
    frame.content_scale = 1.0;
    frame.content_offset = (0.5, 0.0);
    let (a, _) = corners(&frame, 200.0, 100.0);
    assert!(near(a, 150.0, 0.0), "free: {a:?}");
}

// S66-01: an empty frame has no photo to place.
#[test]
fn s66_01_an_empty_frame_has_no_transform() {
    assert!(
        FrameNode::new(ShapeGeometry::Rect)
            .content_transform(10.0, 10.0)
            .is_none()
    );
}

// S66-07: dragging the photo in crop mode moves it by the pointer's px as
// a share of the box, and with «Rellenar» it stops where the box would
// show a gap.
#[test]
fn s66_07_a_drag_moves_the_photo_within_its_room() {
    // 400 × 400 photo in 200 × 100 with «Rellenar»: 200 × 200, room 0.5 down.
    let frame = filled();
    assert_eq!(frame.moved(200.0, 100.0, 0.0, 20.0), (0.0, 0.2));
    assert_eq!(frame.moved(200.0, 100.0, 30.0, 500.0), (0.0, 0.5));
    let mut fit = filled();
    fit.fit = Fit::Fit;
    assert_eq!(fit.moved(200.0, 100.0, 50.0, 0.0), (0.25, 0.0), "free");
}

// S66-07: the whole photo, where it lies in the box of the frame, for the
// faded view of crop mode.
#[test]
fn s66_07_photo_bounds() {
    let bounds = filled().photo_bounds(200.0, 100.0).expect("bounds");
    assert_eq!(
        (bounds.x0, bounds.y0, bounds.x1, bounds.y1),
        (0.0, -50.0, 200.0, 150.0)
    );
    assert!(
        FrameNode::new(ShapeGeometry::Rect)
            .photo_bounds(1.0, 1.0)
            .is_none()
    );
}

/// Whether every corner of the `w` × `h` box falls on the photo.
fn covers(frame: &FrameNode, w: f64, h: f64) -> bool {
    let at = frame.content_transform(w, h).expect("a photo").inverse();
    let (nw, nh) = frame.content.expect("content").natural;
    [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)]
        .iter()
        .all(|&(x, y)| {
            let p = at * Point::new(x, y);
            p.x >= -1e-6
                && p.y >= -1e-6
                && p.x <= f64::from(nw) + 1e-6
                && p.y <= f64::from(nh) + 1e-6
        })
}

// S66-07 «Girar»: a turned photo with «Rellenar» grows just enough to keep
// covering the box, and a drag stops where a corner would be uncovered.
#[test]
fn s66_07_a_turned_photo_keeps_covering_the_box() {
    let mut frame = filled();
    for angle in [0.0, 15.0, 45.0, -30.0, 90.0, 170.0] {
        frame.content_rotation_deg = angle;
        frame.content_offset = (0.0, 0.0);
        assert!(covers(&frame, 200.0, 100.0), "{angle}°");
        frame.content_offset = frame.moved(200.0, 100.0, 300.0, -400.0);
        assert!(covers(&frame, 200.0, 100.0), "{angle}° after a drag");
    }
    // The centre of the photo stays the centre of the box at rest.
    frame.content_rotation_deg = 30.0;
    frame.content_offset = (0.0, 0.0);
    let at = frame.content_transform(200.0, 100.0).expect("photo");
    let centre = at * Point::new(200.0, 200.0);
    assert!(near(centre, 100.0, 50.0), "{centre:?}");
    // The top edge of the photo runs at 30°, clockwise.
    let edge = at * Point::new(1.0, 0.0) - at * Point::new(0.0, 0.0);
    assert!(
        (edge.y.atan2(edge.x).to_degrees() - 30.0).abs() < 1e-6,
        "{edge:?}"
    );
}
