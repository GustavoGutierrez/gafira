//! The background of a page (spec 069).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::timeline::{AssetEntry, AssetRef, AssetStorage, AssetTable};
use gafira_doc::{
    BackgroundFit, ContentHash, Document, Dpi, Gradient, GradientKind, GradientStop, Page,
    PageBackground, PageId, PageSize, PatternKind, ValidationError,
};
use gafira_geom::{Length, Unit};

const PHOTO: AssetRef = AssetRef::new(ContentHash([4; 32]));

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::Srgb(Rgba::new(r, g, b, 1.0))
}

fn document(
    background: Option<PageBackground>,
    assets: AssetTable,
) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(100.0, Unit::Px),
        height: Length::new(100.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    page.background = background;
    Document::from_parts_with_assets(2, Dpi::new(96), vec![Arc::new(page)], assets)
}

fn table() -> AssetTable {
    let mut assets = AssetTable::new();
    assets.insert(
        PHOTO,
        AssetEntry {
            name: "fondo.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    assets
}

// S69-01: every kind of background round-trips; a page without one writes
// no `background`, so its bytes are the ones it had before spec 069.
#[test]
fn s69_01_round_trip() {
    let gradient = Gradient {
        kind: GradientKind::Linear { angle_deg: 45.0 },
        stops: vec![
            GradientStop {
                offset: 0.0,
                color: rgb(1.0, 0.0, 0.0),
            },
            GradientStop {
                offset: 1.0,
                color: rgb(0.0, 0.0, 1.0),
            },
        ],
    };
    for background in [
        PageBackground::Solid(rgb(0.2, 0.4, 0.6)),
        PageBackground::Gradient(gradient),
        PageBackground::Pattern {
            kind: PatternKind::Checks,
            color: rgb(1.0, 0.0, 0.0),
            back: rgb(0.0, 1.0, 0.0),
        },
        PageBackground::Image {
            asset: PHOTO,
            natural: (640, 480),
            fit: BackgroundFit::Tile,
        },
    ] {
        let doc = document(Some(background), table()).expect("valid");
        let json = doc.to_json().expect("json");
        assert!(json.contains(r#""background":{"#), "{json}");
        assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
    }
    let plain = document(None, table())
        .expect("valid")
        .to_json()
        .expect("json");
    assert!(!plain.contains("background"), "{plain}");
}

// S69-01: an image missing from the asset table, a colour that is not
// finite or an image without size does not validate.
#[test]
fn s69_01_validation() {
    let refused = |background, assets| {
        assert_eq!(
            document(Some(background), assets).expect_err("refused"),
            ValidationError::InvalidBackground {
                page: PageId::new(1)
            }
        );
    };
    let image = PageBackground::Image {
        asset: PHOTO,
        natural: (10, 10),
        fit: BackgroundFit::Fill,
    };
    refused(image.clone(), AssetTable::new());
    refused(
        PageBackground::Solid(Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0))),
        table(),
    );
    refused(
        PageBackground::Image {
            asset: PHOTO,
            natural: (0, 10),
            fit: BackgroundFit::Fit,
        },
        table(),
    );
    assert!(document(Some(image), table()).is_ok());
}
