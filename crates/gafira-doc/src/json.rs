//! Wire entry points: bytes to document and back.

use std::fmt;
use std::sync::Arc;

use serde::Deserialize;

use crate::{Document, Dpi, Page};

/// Why a document could not be read.
#[derive(Debug)]
#[non_exhaustive]
pub enum LoadError {
    /// Syntax, shape or encoding error.
    Json(serde_json::Error),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Json(e) => write!(f, "invalid document JSON: {e}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Json(e) => Some(e),
        }
    }
}

/// The unchecked wire shape of a [`Document`].
#[derive(Deserialize)]
pub(crate) struct DocumentWire {
    schema_version: u32,
    next_id: u64,
    #[serde(default)]
    dpi: Dpi,
    pages: Vec<Arc<Page>>,
}

impl From<DocumentWire> for Document {
    fn from(w: DocumentWire) -> Self {
        Document {
            schema_version: w.schema_version,
            next_id: w.next_id,
            dpi: w.dpi,
            pages: w.pages,
        }
    }
}

impl Document {
    /// Reads a document from UTF-8 JSON bytes.
    pub fn from_json(bytes: &[u8]) -> Result<Document, LoadError> {
        serde_json::from_slice(bytes).map_err(LoadError::Json)
    }

    /// Writes the compact canonical JSON form (no trailing newline).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_color::{Color, Rgba};
    use gafira_geom::{Length, Unit};

    use crate::document::test_document;
    use crate::{
        Document, Dpi, ElementBase, ExtensionNode, GroupNode, LoadError, Node, NodeId, NodeKind,
        Page, PageId, PageSize, Placement, ShapeGeometry, ShapeNode,
    };

    fn px(w: f64, h: f64) -> PageSize {
        PageSize {
            width: Length::new(w, Unit::Px),
            height: Length::new(h, Unit::Px),
        }
    }

    fn node(id: u64, kind: NodeKind) -> Arc<Node> {
        let base = ElementBase::new(NodeId::new(id), Placement::new(0.0, 0.0, 1.0, 1.0));
        Arc::new(Node::new(base, kind))
    }

    fn doc_with(nodes: Vec<Arc<Node>>) -> Document {
        let mut p = Page::new(PageId::new(1), "P".to_string(), px(100.0, 200.0));
        for n in nodes {
            p.roots.push(n.base.id);
            p.nodes.insert(n.base.id, n);
        }
        p.roots.sort();
        test_document(3, Dpi::default(), vec![Arc::new(p)])
    }

    fn group(id: u64) -> Arc<Node> {
        node(id, NodeKind::Group(GroupNode {}))
    }

    #[test]
    fn minimal_document_has_exact_bytes() {
        let p = Page::new(PageId::new(1), "P".to_string(), px(100.0, 200.0));
        let d = test_document(3, Dpi::new(96), vec![Arc::new(p)]);
        assert_eq!(
            d.to_json().unwrap(),
            concat!(
                r#"{"schema_version":1,"next_id":3,"dpi":96,"pages":[{"id":1,"name":"P","#,
                r#""size":{"width":{"value":100.0,"unit":"px"},"height":{"value":200.0,"unit":"px"}},"#,
                r#""locked":false,"hidden":false,"roots":[],"nodes":[]}]}"#
            )
        );
    }

    #[test]
    fn round_trip_is_idempotent_and_independent_of_build_order() {
        let a = doc_with(vec![group(10), group(2), group(7)]);
        let b = doc_with(vec![group(7), group(10), group(2)]);
        let bytes = a.to_json().unwrap();
        assert_eq!(bytes, b.to_json().unwrap());
        let back = Document::from_json(bytes.as_bytes()).unwrap();
        assert_eq!(back, a);
        assert_eq!(back.to_json().unwrap(), bytes);
        let order = |d: &Document| {
            d.pages()[0]
                .nodes
                .keys()
                .map(|k| k.get())
                .collect::<Vec<_>>()
        };
        assert_eq!(order(&a), order(&b));
        assert_eq!(order(&a), [2, 7, 10]);
    }

    #[test]
    fn numbers_keep_their_bits_through_a_document() {
        let mut n = Node::new(
            ElementBase::new(NodeId::new(2), Placement::new(-0.0, 0.0, f64::MAX, 1.0)),
            NodeKind::Group(GroupNode {}),
        );
        n.base.opacity = 1e-45;
        n.base.placement.rotation_deg = 5e-324;
        let mut shape = ShapeNode::new(ShapeGeometry::Rect);
        shape.fill = Some(Color::Srgb(Rgba::new(f32::MAX, 1e-45, -0.0, 1.0)));
        let s = node(3, NodeKind::Shape(shape));
        let d = doc_with(vec![Arc::new(n), s]);
        let back = Document::from_json(d.to_json().unwrap().as_bytes()).unwrap();
        let (a, b) = (&d.pages()[0], &back.pages()[0]);
        let (a, b) = (
            &a.nodes[&NodeId::new(2)].base,
            &b.nodes[&NodeId::new(2)].base,
        );
        assert_eq!(a.opacity.to_bits(), b.opacity.to_bits());
        assert!(b.placement.x.is_sign_negative());
        assert_eq!(a.placement.w.to_bits(), b.placement.w.to_bits());
        assert_eq!(
            a.placement.rotation_deg.to_bits(),
            b.placement.rotation_deg.to_bits()
        );
        let fill = |d: &Document| match &d.pages()[0].nodes[&NodeId::new(3)].kind {
            NodeKind::Shape(ShapeNode {
                fill: Some(Color::Srgb(c)),
                ..
            }) => [c.r, c.g, c.b, c.a].map(f32::to_bits),
            _ => panic!("not an sRGB shape"),
        };
        assert_eq!(fill(&back), fill(&d));
        assert!(f32::from_bits(fill(&back)[2]).is_sign_negative());
        assert_eq!(back, d);
    }

    #[test]
    fn non_finite_numbers_serialize_as_null_and_do_not_load() {
        let with = |f: &dyn Fn(&mut Node)| {
            let mut n = Arc::try_unwrap(group(2)).unwrap();
            f(&mut n);
            doc_with(vec![Arc::new(n)])
        };
        let mut docs = vec![
            (with(&|n| n.base.opacity = f32::NAN), r#""opacity":null"#),
            (with(&|n| n.base.placement.x = f64::INFINITY), r#""x":null"#),
            (
                with(&|n| n.base.placement.rotation_deg = f64::NEG_INFINITY),
                r#""rotation_deg":null"#,
            ),
            (
                with(&|n| {
                    let mut s = ShapeNode::new(ShapeGeometry::Rect);
                    s.fill = Some(Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0)));
                    n.kind = NodeKind::Shape(s);
                }),
                r#""space":"srgb","r":null,"g":0.0"#,
            ),
        ];
        let mut page = Page::new(PageId::new(1), String::new(), px(f64::NAN, 1.0));
        page.roots.clear();
        docs.push((
            test_document(2, Dpi::default(), vec![Arc::new(page)]),
            r#""width":{"value":null,"unit":"px"}"#,
        ));
        for (d, field) in docs {
            let json = d.to_json().unwrap();
            assert!(json.contains(field), "{field} not in {json}");
            assert!(matches!(
                Document::from_json(json.as_bytes()),
                Err(LoadError::Json(_))
            ));
        }
    }

    #[test]
    fn extension_fields_survive_and_stay_separate_from_the_document_version() {
        let ext = ExtensionNode {
            type_id: "com.example.sticker".to_string(),
            schema_version: 2,
            payload: serde_json::json!({"b": [1, 2.5, null], "a": "é"}),
        };
        let d = doc_with(vec![node(2, NodeKind::Extension(ext))]);
        let json = d.to_json().unwrap();
        assert!(json.contains(r#""schema_version":2,"payload":{"a":"é","b":[1,2.5,null]}"#));
        let back = Document::from_json(json.as_bytes()).unwrap();
        assert_eq!(back, d);
        assert_eq!(back.schema_version(), 1);
    }

    #[test]
    fn unreadable_bytes_are_json_errors_with_a_source() {
        let good = doc_with(vec![group(2)]).to_json().unwrap();
        let mut cases: Vec<Vec<u8>> = vec![Vec::new(), format!("{good}x").into_bytes()];
        cases.push(vec![0xff, 0xfe, b'{']);
        cases.push([&[0xef, 0xbb, 0xbf][..], good.as_bytes()].concat());
        cases.push(good.replace(r#""P""#, r#""\ud800""#).into_bytes());
        cases.extend((0..good.len()).map(|n| good.as_bytes()[..n].to_vec()));
        for c in cases {
            let err = Document::from_json(&c).unwrap_err();
            assert!(matches!(err, LoadError::Json(_)));
            assert!(std::error::Error::source(&err).is_some());
            assert!(err.to_string().starts_with("invalid document JSON"));
        }
        assert!(Document::from_json(good.as_bytes()).is_ok());
    }

    #[test]
    fn defaults_are_applied_and_rewritten_explicitly() {
        let json = r#"{"schema_version":1,"next_id":9,"pages":[{"id":1,"size":{"width":{"value":1,"unit":"px"},"height":{"value":1,"unit":"px"}},"roots":[2],"nodes":[{"base":{"id":2,"placement":{"x":0,"y":0,"w":1,"h":1}},"kind":{"type":"group"}}]}]}"#;
        let d = Document::from_json(json.as_bytes()).unwrap();
        assert_eq!(d.dpi(), Dpi::default());
        let out = d.to_json().unwrap();
        assert!(
            out.contains(r#""dpi":96"#)
                && out.contains(r#""opacity":1.0,"blend":"normal","visible":true"#)
        );
        assert!(out.contains(r#""name":null"#) && out.contains(r#""children":[]"#));
        assert_eq!(Document::from_json(out.as_bytes()).unwrap(), d);
    }

    #[test]
    fn full_golden_equals_the_built_document() {
        let mut group = Node::new(
            ElementBase::new(NodeId::new(2), Placement::new(1.0, 2.0, 30.0, 40.0)),
            NodeKind::Group(GroupNode {}),
        );
        group.children.push(NodeId::new(5));
        group.base.name = Some("Group".to_string());
        let mut ellipse = ShapeNode::new(ShapeGeometry::Ellipse);
        ellipse.fill = Some(Color::Srgb(Rgba::new(1.0, 0.5, 0.25, 1.0)));
        let mut shaded = Node::new(
            ElementBase::new(NodeId::new(3), Placement::new(-0.0, 0.0, 5.5, 6.5)),
            NodeKind::Shape(ellipse),
        );
        shaded.base.opacity = 0.5;
        shaded.base.visible = false;
        shaded.base.placement.rotation_deg = 45.0;
        let ext = ExtensionNode {
            type_id: "com.example.sticker".to_string(),
            schema_version: 2,
            payload: serde_json::json!({"b": [1, 2.5, null], "a": "é"}),
        };
        let mut page = Page::new(PageId::new(1), "Cover".to_string(), px(100.0, 200.0));
        let nodes = [
            Arc::new(group),
            Arc::new(shaded),
            node(4, NodeKind::Extension(ext)),
            node(5, NodeKind::Shape(ShapeNode::new(ShapeGeometry::Rect))),
        ];
        page.roots = vec![NodeId::new(2), NodeId::new(3), NodeId::new(4)];
        for n in nodes {
            page.nodes.insert(n.base.id, n);
        }
        let a4 = PageSize {
            width: Length::new(21.0, Unit::Cm),
            height: Length::new(29.7, Unit::Cm),
        };
        let mut second = Page::new(PageId::new(10), String::new(), a4);
        second.hidden = true;
        let d = test_document(20, Dpi::new(300), vec![Arc::new(page), Arc::new(second)]);
        let golden = include_str!("../tests/golden/full.json");
        assert_eq!(d.to_json().unwrap(), golden);
        assert_eq!(Document::from_json(golden.as_bytes()).unwrap(), d);
    }

    #[test]
    fn dpi_survives_the_wire_conversion() {
        let d = test_document(3, Dpi::new(300), Vec::new());
        let json = d.to_json().unwrap();
        assert!(json.contains(r#""dpi":300"#), "{json}");
        let back = Document::from_json(json.as_bytes()).unwrap();
        assert_eq!(back.dpi().get(), 300);
        assert_eq!(back, d);
    }

    #[test]
    fn schema_version_is_read_as_written_by_the_wire_conversion() {
        let json = r#"{"schema_version":2,"next_id":3,"dpi":96,"pages":[]}"#;
        let d = Document::from_json(json.as_bytes()).unwrap();
        assert_eq!(d.schema_version(), 2);
        assert_eq!(d.to_json().unwrap(), json);
    }
}
