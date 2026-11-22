//! How an image is made to look (spec 085, S85-01).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    AssetEntry, AssetRef, AssetStorage, AssetTable, ColorShift, ContentHash, CropShape, Document,
    Dpi, ElementBase, Flip, ImageAdjust, ImageCrop, ImageFilter, ImageLook, ImageNode, ImageShadow,
    ImageZone, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ShadowKind,
    ValidationError, ZoneKind,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);

fn asset(n: u8) -> AssetRef {
    AssetRef::new(ContentHash([n; 32]))
}

fn document(image: ImageNode) -> Result<Document, ValidationError> {
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
    for n in [1, 5] {
        table.insert(asset(n), entry.clone());
    }
    Document::from_parts_with_assets(3, Dpi::new(96), vec![Arc::new(page)], table)
}

fn look() -> ImageLook {
    let red = Color::Srgb(Rgba::new(1.0, 0.0, 0.0, 1.0));
    ImageLook {
        adjust: ImageAdjust {
            contrast: 20.0,
            invert: true,
            colors: vec![ColorShift {
                color: red,
                hue: -20.0,
                saturation: 5.0,
            }],
            ..ImageAdjust::default()
        },
        zones: vec![ImageZone {
            kind: ZoneKind::Background,
            mask: asset(5),
            inverted: true,
            adjust: ImageAdjust {
                brightness: -30.0,
                ..ImageAdjust::default()
            },
        }],
        filter: Some(ImageFilter {
            id: "alba".into(),
            intensity: 60.0,
        }),
        shadow: Some(ImageShadow::new(ShadowKind::Drop)),
    }
}

// S85-01: it round-trips, and nothing is written for an image without it.
#[test]
fn s85_01_round_trip() {
    let image = ImageNode::new(asset(1)).looking(look());
    let doc = document(image).expect("valid");
    let json = doc.to_json().expect("json");
    assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
    assert!(!json.contains("temperature"), "only what changes: {json}");
    let plain = ImageNode::new(asset(1)).looking(ImageLook::default());
    assert_eq!(plain, ImageNode::new(asset(1)));
    assert!(
        !document(plain)
            .expect("valid")
            .to_json()
            .expect("json")
            .contains("look")
    );
}

// S85-01: each limit, inside and outside; a mask is an asset of the table.
#[test]
fn s85_01_validation() {
    let ok = |change: &dyn Fn(&mut ImageLook)| {
        let mut look = look();
        change(&mut look);
        let result = document(ImageNode::new(asset(1)).looking(look));
        if let Err(error) = &result {
            assert_eq!(*error, ValidationError::InvalidImage { node: NODE });
        }
        result.is_ok()
    };
    assert!(ok(&|_| ()));
    for v in [-100.0, 100.0] {
        assert!(ok(&|l| (
            l.adjust.temperature,
            l.adjust.blacks,
            l.adjust.vignette
        ) = (v, v, v)));
    }
    assert!(ok(&|l| l.adjust.hue = 180.0) && !ok(&|l| l.adjust.hue = 180.1));
    for wrong in [-100.5, 101.0, f64::NAN] {
        assert!(!ok(&|l| l.adjust.saturation = wrong) && !ok(&|l| l.zones[0].adjust.tint = wrong));
        assert!(
            !ok(&|l| l.adjust.sharpness = wrong)
                && !ok(&|l| l.filter.as_mut().unwrap().intensity = wrong)
        );
        assert!(!ok(&|l| l.shadow.as_mut().unwrap().opacity = wrong));
    }
    assert!(!ok(&|l| l.adjust.clarity = -1.0), "0 to 100");
    let shift = look().adjust.colors[0];
    assert!(
        ok(&|l| l.adjust.colors = vec![shift; 8]) && !ok(&|l| l.adjust.colors = vec![shift; 9])
    );
    assert!(!ok(&|l| l.adjust.colors[0].hue = 181.0));
    assert!(!ok(
        &|l| l.adjust.colors[0].color = Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0))
    ));
    let zone = look().zones[0].clone();
    assert!(ok(&|l| l.zones = vec![zone.clone(); 8]) && !ok(&|l| l.zones = vec![zone.clone(); 9]));
    assert!(!ok(&|l| l.filter.as_mut().unwrap().id = String::new()));
    assert!(!ok(&|l| l.filter.as_mut().unwrap().id = "a".repeat(65)));
    for wrong in [-0.01, 0.31, f64::NAN] {
        assert!(!ok(&|l| l.shadow.as_mut().unwrap().blur = wrong));
        assert!(!ok(&|l| l.shadow.as_mut().unwrap().distance = wrong));
        assert!(!ok(&|l| l.shadow.as_mut().unwrap().size = wrong));
    }
    assert!(!ok(&|l| l.shadow.as_mut().unwrap().angle_deg = 200.0));
    let mut lost = look();
    lost.zones[0].mask = asset(9);
    assert_eq!(
        document(ImageNode::new(asset(1)).looking(lost)),
        Err(ValidationError::DanglingImage { node: NODE })
    );
}

// S85-02: each look has pixels of its own, and an image keeps its look when
// its resolution or its background change.
#[test]
fn s85_02_keys() {
    let image = ImageNode::new(asset(1));
    assert_eq!(image.shown(), asset(1));
    let one = image.looking(look());
    let mut other = look();
    other.filter.as_mut().unwrap().intensity = 61.0;
    let keys = [
        one.shown(),
        image.looking(other).shown(),
        ImageNode::new(asset(2)).looking(look()).shown(),
        asset(1),
    ];
    for (i, a) in keys.iter().enumerate() {
        assert!(keys[i + 1..].iter().all(|b| a != b));
    }
    assert_eq!(one.shown(), image.looking(look()).shown());
    assert_eq!(one.edited(asset(3), asset(2)).look, one.look);
    assert!(one.assets().any(|a| a == asset(5)));
}

// S85-04: how far a shadow reaches, whatever the box: the image itself still
// fills it.
#[test]
fn s85_04_bleed() {
    for kind in [
        ShadowKind::Glow,
        ShadowKind::Drop,
        ShadowKind::Outline,
        ShadowKind::Curved,
        ShadowKind::Floating,
        ShadowKind::Angled,
        ShadowKind::Ground,
    ] {
        let s = ImageShadow::new(kind);
        let reach = s.reach();
        assert!(reach > 0.0 && reach <= 2.0, "{kind:?} {reach}");
        // An image of 400 × 200 with its margin, in boxes of any shape.
        let margin = (reach * 200.0).ceil();
        let padded = (400 + 2 * margin as u32, 200 + 2 * margin as u32);
        for (w, h) in [(400.0, 200.0), (100.0, 100.0)] {
            let (x, y) = s.bleed(padded, (w, h));
            assert!((x / w - margin / 400.0).abs() < 0.01, "{kind:?} {x}");
            assert!((y / h - margin / 200.0).abs() < 0.01, "{kind:?} {y}");
        }
        assert_eq!(s.bleed((0, 0), (10.0, 10.0)), (0.0, 0.0));
    }
}

fn part(shape: CropShape) -> ImageCrop {
    ImageCrop {
        x: 0.2,
        y: 0.1,
        w: 0.5,
        h: 0.25,
        shape,
    }
}

// S88-01: a crop round-trips with its shape, keeps its own key, and is not
// written when all of the image shows.
#[test]
fn s88_01_round_trip() {
    let path = CropShape::Path(vec![[0.0, 0.0], [1.0, 0.2], [0.4, 1.0]]);
    let image = ImageNode::new(asset(1));
    let mut keys = vec![image.shown()];
    for shape in [
        CropShape::Rect,
        CropShape::Rounded(0.2),
        CropShape::Ellipse,
        path,
    ] {
        let cropped = image.cropped(part(shape));
        let doc = document(cropped.clone()).expect("valid");
        let json = doc.to_json().expect("json");
        assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
        assert!(!keys.contains(&cropped.shown()) && !cropped.is_plain());
        keys.push(cropped.shown());
        assert_eq!(cropped.looking(look()).crop, cropped.crop);
        assert_eq!(cropped.edited(asset(3), asset(2)).crop, cropped.crop);
    }
    assert_eq!(image.cropped(ImageCrop::default()), image);
    assert!(
        !document(image)
            .expect("valid")
            .to_json()
            .expect("json")
            .contains("crop")
    );
}

// S88-01: each limit, inside and outside.
#[test]
fn s88_01_validation() {
    let ok = |change: &dyn Fn(&mut ImageCrop)| {
        let mut crop = part(CropShape::Rect);
        change(&mut crop);
        let mut image = ImageNode::new(asset(1));
        image.crop = Some(Box::new(crop));
        document(image).is_ok()
    };
    assert!(ok(&|_| ()) && ok(&|c| (c.x, c.w) = (0.0, 1.0)) && ok(&|c| c.w = ImageCrop::LEAST));
    // Sums of shares are not exact.
    assert!(ok(&|c| (c.x, c.w) = (0.1 + 0.2, 0.7)));
    for wrong in [-0.01, f64::NAN, f64::INFINITY] {
        assert!(!ok(&|c| c.x = wrong) && !ok(&|c| c.y = wrong));
        assert!(!ok(&|c| c.w = wrong) && !ok(&|c| c.h = wrong));
        assert!(!ok(&|c| c.shape = CropShape::Rounded(wrong)));
        let points = vec![[0.0, 0.0], [wrong, 0.0], [1.0, 1.0]];
        assert!(!ok(&|c| c.shape = CropShape::Path(points.clone())));
    }
    assert!(!ok(&|c| c.w = 0.0) && !ok(&|c| c.w = 0.9) && !ok(&|c| c.h = 0.95));
    assert!(
        ok(&|c| c.shape = CropShape::Rounded(0.5)) && !ok(&|c| c.shape = CropShape::Rounded(0.51))
    );
    let points = |n: usize| CropShape::Path(vec![[0.5, 0.5]; n]);
    assert!(!ok(&|c| c.shape = points(2)) && ok(&|c| c.shape = points(3)));
    assert!(ok(&|c| c.shape = points(512)) && !ok(&|c| c.shape = points(513)));
}

// S88-03: what is seen stays where it is, turned, skewed or mirrored; and
// the whole box comes back.
#[test]
fn s88_03_what_is_seen_stays() {
    for (degrees, skew, x, y) in [
        (0.0, 0.0, false, false),
        (30.0, 0.0, true, false),
        (-100.0, 12.0, false, true),
    ] {
        let mut whole = Placement::new(40.0, -10.0, 300.0, 200.0);
        (whole.rotation_deg, whole.skew) = (degrees, skew);
        whole.flip = Flip { x, y };
        let crop = part(CropShape::Ellipse);
        let seen = crop.part_box(&whole);
        assert!((seen.w - 150.0).abs() < 1e-9 && (seen.h - 50.0).abs() < 1e-9);
        assert_eq!(
            (seen.rotation_deg, seen.skew, seen.flip),
            (degrees, skew, whole.flip)
        );
        for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.3, 0.8)] {
            let in_part = seen.to_transform() * gafira_geom::Point::new(u * seen.w, v * seen.h);
            let (u, v) = (crop.x + u * crop.w, crop.y + v * crop.h);
            let in_whole = whole.to_transform() * gafira_geom::Point::new(u * whole.w, v * whole.h);
            assert!(
                (in_part - in_whole).hypot() < 1e-9,
                "{degrees} {in_part:?} {in_whole:?}"
            );
        }
        let back = crop.whole_box(&seen);
        for (a, b) in [
            (back.x, whole.x),
            (back.y, whole.y),
            (back.w, whole.w),
            (back.h, whole.h),
        ] {
            assert!((a - b).abs() < 1e-9);
        }
    }
}
