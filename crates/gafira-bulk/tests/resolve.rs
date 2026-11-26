//! Spec 104 §5, §7.4, §10.1: a grid as a table, and the template filled in
//! row by row without changing it.

use std::sync::Arc;

use gafira_bulk::{FieldRef, Grid, from_grid, guess_header, plan, resolve, resolve_rich, unknown};
use gafira_color::{Color, Rgba};
use gafira_doc::{
    DataSet, Document, ElementBase, Node, NodeId, NodeKind, Page, PageId, PageSize, Paragraph,
    Placement, RichText, Span, SpanStyle, TextNode,
};
use gafira_geom::{Length, Unit};

fn grid(rows: &[&[&str]]) -> Grid {
    rows.iter()
        .map(|r| r.iter().map(|c| (*c).to_owned()).collect())
        .collect()
}

fn style(font: &str) -> SpanStyle {
    SpanStyle {
        font: font.to_owned(),
        size: 20.0,
        color: Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0)),
        bold: false,
        italic: false,
    }
}

fn unnamed(i: usize) -> String {
    format!("Columna {}", gafira_bulk::column_letter(i))
}

// S104-20: the names row is the fullest of the first rows (§5.2).
#[test]
fn s104_20_guess_the_names_row() {
    let g = grid(&[
        &["Informe de afiliados", "", ""],
        &["", "", ""],
        &["Nombre", "Curso", "Fecha"],
        &["Ana", "Rust", "2026-01-02"],
    ]);
    assert_eq!(guess_header(&g), Some(2));
    assert_eq!(guess_header(&Grid::new()), None);
}

// S104-21: names, empty titles, empty rows and short rows (§5.2, §5.3).
#[test]
fn s104_21_grid_to_table() {
    let g = grid(&[
        &["Título", "", ""],
        &["Nombre", "", "Código"],
        &["Ana", "x", "00125"],
        &["", "", ""],
        &["Luis"],
        &["Eva", "y", "7", "sobra"],
    ]);
    let imported = from_grid(&g, Some(1), &unnamed);
    let set = &imported.set;
    let names: Vec<_> = set.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Nombre", "Columna B", "Código", "Columna D"]);
    let keys: Vec<_> = set.columns.iter().map(|c| c.key.as_str()).collect();
    assert_eq!(keys, ["nombre", "columna_b", "codigo", "columna_d"]);
    assert_eq!(set.rows.len(), 3);
    assert_eq!(imported.empty_rows, 1);
    assert_eq!(imported.uneven_rows, 2);
    assert_eq!(set.rows[0].values, ["Ana", "x", "00125", ""]);
    assert_eq!(set.rows[1].values, ["Luis", "", "", ""]);
    assert_eq!(set.problem(), None);
    // Without names every row is data.
    let all = from_grid(&g, None, &unnamed);
    assert_eq!(all.set.rows.len(), 5);
    assert_eq!(all.set.columns[0].name, "Columna A");
}

// S104-22: rows go first, then the template pages, in order (AC 9).
#[test]
fn s104_22_plan_order() {
    let (a, b) = (PageId::new(1), PageId::new(2));
    assert_eq!(
        plan(&[0, 1, 2], &[a, b]),
        vec![(0, a), (0, b), (1, a), (1, b), (2, a), (2, b)]
    );
}

fn lookup(f: &FieldRef) -> Option<String> {
    match f {
        FieldRef::Key(k) if k == "nombre" => Some("Ana\nPérez".to_owned()),
        FieldRef::Key(k) if k == "vacio" => Some(String::new()),
        FieldRef::Name(n) if n == "Curso" => Some("Rust".to_owned()),
        _ => None,
    }
}

// S104-23: the field takes the style of where it starts; the fixed text
// keeps its own; a line in a value is a new paragraph (§7.4, AC 5).
#[test]
fn s104_23_resolve_keeps_styles() {
    let rich = RichText {
        paragraphs: vec![Paragraph {
            spans: vec![
                Span {
                    text: "Hola, {{nom".into(),
                    style: style("Inter"),
                },
                Span {
                    text: "bre}} de {{[\"Curso\"]}}!".into(),
                    style: style("Lora"),
                },
            ],
        }],
    };
    let (out, missing) = resolve_rich(&rich, &lookup);
    assert!(missing.is_empty());
    assert_eq!(out.plain_text(), "Hola, Ana\nPérez de Rust!");
    assert_eq!(out.paragraphs.len(), 2);
    let first = &out.paragraphs[0].spans;
    assert_eq!(first[0].text, "Hola, Ana");
    assert_eq!(first[0].style.font, "Inter");
    let second = &out.paragraphs[1].spans;
    assert_eq!(second[0].text, "Pérez");
    assert_eq!(second[0].style.font, "Inter");
    assert_eq!(second[1].text, " de Rust!");
    assert_eq!(second[1].style.font, "Lora");
}

// S104-24: unknown fields stay written and are told; an empty value is
// empty, never the example text (§6, §7.3 rule 5).
#[test]
fn s104_24_unknown_and_empty() {
    let rich = RichText::plain("{{otro}}-{{vacio}}-\\{{nombre}}", &style("Inter"));
    let (out, missing) = resolve_rich(&rich, &lookup);
    assert_eq!(out.plain_text(), "{{otro}}--{{nombre}}");
    assert_eq!(missing, vec![FieldRef::Key("otro".into())]);
    // Values are never read as fields again.
    let sneaky = |_: &FieldRef| Some("{{nombre}}".to_owned());
    let (out, _) = resolve_rich(&RichText::plain("{{x}}", &style("Inter")), &sneaky);
    assert_eq!(out.plain_text(), "{{nombre}}");
}

fn text_node(id: u64, words: &str) -> Node {
    let base = ElementBase::new(NodeId::new(id), Placement::new(10.0, 10.0, 200.0, 40.0));
    Node {
        base,
        kind: NodeKind::Text(TextNode::plain(words, &style("Inter"))),
        children: Vec::new(),
    }
}

fn template() -> (Document, Arc<DataSet>) {
    let size = PageSize {
        width: Length::new(400.0, Unit::Px),
        height: Length::new(300.0, Unit::Px),
    };
    let mut first = Page::new(PageId::new(1), "Frente".into(), size);
    first.roots = vec![NodeId::new(2), NodeId::new(3)];
    first
        .nodes
        .insert(NodeId::new(2), Arc::new(text_node(2, "Hola, {{nombre}}")));
    first
        .nodes
        .insert(NodeId::new(3), Arc::new(text_node(3, "Fijo")));
    let mut back = Page::new(PageId::new(4), "Reverso".into(), size);
    back.roots = vec![NodeId::new(5)];
    back.nodes.insert(
        NodeId::new(5),
        Arc::new(text_node(5, "{{curso}} {{falta}}")),
    );
    let doc = Document::from_parts(
        6,
        gafira_doc::Dpi::default(),
        vec![Arc::new(first), Arc::new(back)],
    )
    .unwrap();
    let g = grid(&[&["Nombre", "Curso"], &["Ana", "Rust"], &["Luis", "Go"]]);
    let set = Arc::new(from_grid(&g, Some(0), &unnamed).set);
    let doc = doc.with_data(Some(Arc::clone(&set))).unwrap();
    (doc, set)
}

// S104-25: N rows × P pages, with new ids, the template untouched and no
// table in the result (§10.1, §10.2 req 1, 5 and 6).
#[test]
fn s104_25_resolve_the_document() {
    let (doc, set) = template();
    let before = doc.clone();
    let pages = [PageId::new(1), PageId::new(4)];
    let out = resolve(&doc, &set, &[0, 1], &pages).unwrap();
    assert_eq!(doc, before);
    let result = &out.document;
    assert_eq!(result.pages().len(), 4);
    assert_eq!(result.data(), None);
    let words = |page: usize, node: usize| {
        let p = &result.pages()[page];
        match &p.nodes[&p.roots[node]].kind {
            NodeKind::Text(t) => t.content.plain_text(),
            _ => String::new(),
        }
    };
    assert_eq!(words(0, 0), "Hola, Ana");
    assert_eq!(words(0, 1), "Fijo");
    assert_eq!(words(1, 0), "Rust {{falta}}");
    assert_eq!(words(2, 0), "Hola, Luis");
    assert_eq!(words(3, 0), "Go {{falta}}");
    assert_eq!(result.pages()[2].name, "Frente #2");
    assert_eq!(out.rows, vec![0, 0, 1, 1]);
    assert!(result.validate().is_ok());
    // The unknown field is found once, before any row.
    let missing = unknown(&doc, &set, &pages);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].2, FieldRef::Key("falta".into()));
}

// S104-26: a cancelled run stops between rows; the filled elements are
// listed for the boxes that follow their words.
#[test]
fn s104_26_cancel_between_rows() {
    let (doc, set) = template();
    let pages = [PageId::new(1), PageId::new(4)];
    let mut seen = Vec::new();
    let mut go_on = |done: usize| {
        seen.push(done);
        done < 1
    };
    let out = gafira_bulk::resolve_with(&doc, &set, &[0, 1], &pages, &mut go_on).unwrap();
    assert!(out.is_none());
    assert_eq!(seen, [0, 1]);
    let all = resolve(&doc, &set, &[0], &pages).unwrap();
    assert_eq!(all.filled.len(), 2);
}
