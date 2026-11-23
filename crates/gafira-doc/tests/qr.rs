//! QR codes in the document (spec 080, FD-20).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::timeline::{AssetEntry, AssetRef, AssetStorage, AssetTable};
use gafira_doc::{
    ContentHash, Document, Dpi, ElementBase, Gradient, GradientKind, GradientStop, Node, NodeId,
    NodeKind, Page, PageId, PageSize, Placement, QrCoin, QrContact, QrContent, QrEcc, QrFill,
    QrLogo, QrNode, QrWifiSecurity, ValidationError,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);
const ASSET: AssetRef = AssetRef::new(ContentHash([7; 32]));

fn black() -> Color {
    Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0))
}

fn white() -> Color {
    Color::Srgb(Rgba::new(1.0, 1.0, 1.0, 1.0))
}

fn table() -> AssetTable {
    let mut assets = AssetTable::new();
    assets.insert(
        ASSET,
        AssetEntry {
            name: "logo.png".to_owned(),
            storage: AssetStorage::Embedded,
        },
    );
    assets
}

fn document(qr: QrNode, assets: AssetTable) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(400.0, Unit::Px),
        height: Length::new(400.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let node = Node::new(
        ElementBase::new(NODE, Placement::new(10.0, 10.0, 100.0, 100.0)),
        NodeKind::Qr(Box::new(qr)),
    );
    page.nodes.insert(NODE, Arc::new(node));
    page.roots = vec![NODE];
    Document::from_parts_with_assets(3, Dpi::new(96), vec![Arc::new(page)], assets)
}

fn plain() -> QrNode {
    let url = "https://example.com".to_owned();
    QrNode::new(QrContent::Url { url }, black(), white())
}

fn contents() -> Vec<QrContent> {
    let s = str::to_owned;
    vec![
        QrContent::Url {
            url: s("https://example.com"),
        },
        QrContent::Text { text: s("hola") },
        QrContent::Email {
            address: s("ana@example.com"),
            subject: s("Hola"),
            body: s(""),
        },
        QrContent::Phone {
            number: s("+34 600"),
        },
        QrContent::Sms {
            number: s("600"),
            text: s("hola"),
        },
        QrContent::Wifi {
            ssid: s("Café"),
            password: s("a;b"),
            security: QrWifiSecurity::Wpa,
            hidden: true,
        },
        QrContent::Contact(Box::new(QrContact {
            first_name: s("Ana"),
            ..QrContact::default()
        })),
        QrContent::Event {
            title: s("Feria"),
            place: s(""),
            start: s("2026-11-22 10:00"),
            end: s("2026-11-22 12:00"),
        },
        QrContent::Location {
            latitude: s("4.6"),
            longitude: s("-74.1"),
        },
        QrContent::Social {
            url: s("https://example.com/ana"),
        },
        QrContent::Crypto {
            coin: QrCoin::Litecoin,
            address: s("abc"),
            amount: s("0.5"),
        },
    ]
}

// S80-02: every kind of content round-trips, with a gradient and a logo.
#[test]
fn s80_02_qr_round_trip() {
    for content in contents() {
        let mut qr = plain();
        qr.content = content;
        qr.style.eye_ball_fill = Some(QrFill::Gradient(Gradient {
            kind: GradientKind::Linear { angle_deg: 90.0 },
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: black(),
                },
                GradientStop {
                    offset: 1.0,
                    color: white(),
                },
            ],
        }));
        qr.logo = Some(logo(0.2));
        let doc = document(qr, table()).expect("valid");
        let json = doc.to_json().expect("json");
        assert!(json.contains(r#""type":"qr""#), "{json}");
        assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
    }
}

fn logo(share: f64) -> QrLogo {
    QrLogo {
        asset: ASSET,
        natural: (200, 100),
        share,
        clear_behind: true,
    }
}

// S80-02: each limit, inside and outside.
#[test]
fn s80_02_qr_validation() {
    let ok = |change: fn(&mut QrNode)| {
        let mut qr = plain();
        change(&mut qr);
        document(qr, table()).is_ok()
    };
    assert!(ok(|_| ()));
    assert!(ok(|qr| qr.quiet_zone = 0));
    assert!(ok(|qr| qr.quiet_zone = 10));
    assert!(!ok(|qr| qr.quiet_zone = 11));
    assert!(ok(|qr| qr.logo = Some(logo(0.1))));
    assert!(ok(|qr| qr.logo = Some(logo(0.3))));
    assert!(!ok(|qr| qr.logo = Some(logo(0.09))));
    assert!(!ok(|qr| qr.logo = Some(logo(0.31))));
    assert!(!ok(|qr| qr.logo = Some(logo(f64::NAN))));
    assert!(!ok(|qr| {
        qr.logo = Some(QrLogo {
            natural: (0, 100),
            ..logo(0.2)
        });
    }));
    assert!(!ok(|qr| {
        qr.style.fill = QrFill::Solid(Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0)));
    }));
    assert!(!ok(|qr| {
        qr.style.background = Some(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.5)));
    }));
    assert!(!ok(|qr| {
        qr.style.eye_frame_fill = Some(QrFill::Gradient(Gradient {
            kind: GradientKind::Radial,
            stops: Vec::new(),
        }));
    }));
    assert!(ok(|qr| qr.content = QrContent::Text {
        text: "a".repeat(QrNode::MAX_CONTENT)
    }));
    assert!(!ok(|qr| qr.content = QrContent::Text {
        text: "a".repeat(QrNode::MAX_CONTENT + 1)
    }));
    // A code half written is kept as it is.
    assert!(ok(|qr| qr.content = QrContent::Url { url: String::new() }));
    // P7: the logo is an asset of the table.
    let mut qr = plain();
    qr.logo = Some(logo(0.2));
    assert!(matches!(
        document(qr, AssetTable::new()),
        Err(ValidationError::InvalidQr { node: NODE })
    ));
}

// S80-10: a logo takes the highest correction; without it, the one chosen.
#[test]
fn s80_10_a_logo_raises_and_restores_the_level() {
    let mut qr = plain();
    qr.ecc = QrEcc::L;
    assert_eq!(qr.effective_ecc(), QrEcc::L);
    qr.logo = Some(logo(0.2));
    assert_eq!(qr.effective_ecc(), QrEcc::H);
    qr.logo = None;
    assert_eq!(qr.effective_ecc(), QrEcc::L);
}
