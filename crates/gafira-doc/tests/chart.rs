//! Charts in the document (spec 083, FD-22, D-114).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    ChartCombine, ChartKind, ChartLabels, ChartNode, ChartText, Document, Dpi, ElementBase,
    MAX_CHART_COLORS, Node, NodeId, NodeKind, Page, PageId, PageSize, Placement, ValidationError,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);

fn black() -> Color {
    Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0))
}

fn chart() -> ChartNode {
    ChartNode {
        kind: ChartKind::PercentArea,
        columns: 2,
        cells: ["Mes", "Ventas", "Enero", "12,5", "Febrero", ""]
            .map(str::to_owned)
            .to_vec(),
        labels: Some(0),
        values: vec![1],
        combine: ChartCombine::Mean,
        colors: vec![black()],
        by_label: true,
        text: ChartText {
            font: "Inter".to_owned(),
            size: 16.0,
            color: black(),
            bold: true,
            italic: false,
        },
        title: "Ventas".to_owned(),
        legend: true,
        names: true,
        numbers: ChartLabels::Percent,
        axis: true,
        grid: true,
        smooth: true,
        markers: true,
        round: 0.5,
        gap: 0.3,
        hole: 0.6,
        weight: 3.0,
    }
}

fn document(chart: ChartNode) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(400.0, Unit::Px),
        height: Length::new(400.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let node = Node::new(
        ElementBase::new(NODE, Placement::new(10.0, 10.0, 320.0, 90.0)),
        NodeKind::Chart(Box::new(chart)),
    );
    page.nodes.insert(NODE, Arc::new(node));
    page.roots = vec![NODE];
    Document::from_parts(3, Dpi::new(96), vec![Arc::new(page)])
}

// S83-01: a chart round-trips.
#[test]
fn s83_01_chart_round_trip() {
    let doc = document(chart()).expect("valid");
    let json = doc.to_json().expect("json");
    assert!(
        json.contains(r#""type":"chart""#) && json.contains("percentArea"),
        "{json}"
    );
    assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
}

// S83-01: each limit, inside and outside.
#[test]
fn s83_01_chart_validation() {
    let ok = |change: &dyn Fn(&mut ChartNode)| {
        let mut chart = chart();
        change(&mut chart);
        let result = document(chart);
        if let Err(error) = &result {
            assert_eq!(*error, ValidationError::InvalidChart { node: NODE });
        }
        result.is_ok()
    };
    let grid = |rows: usize, columns: usize| {
        ok(&move |c| {
            c.columns = columns;
            c.cells = vec![String::new(); rows * columns];
            (c.labels, c.values) = (None, Vec::new());
        })
    };
    assert!(ok(&|_| ()));
    assert!(grid(1, 1) && grid(201, 20));
    assert!(!grid(0, 1) && !grid(1, 0) && !grid(202, 1) && !grid(1, 21));
    assert!(!ok(&|c| c.cells.push(String::new())), "not whole rows");
    assert!(!ok(&|c| c.cells[0] = "a".repeat(1025)) && ok(&|c| c.cells[0] = "a".repeat(1024)));
    assert!(!ok(&|c| c.title = "a".repeat(1025)));
    assert!(!ok(&|c| c.labels = Some(2)) && !ok(&|c| c.values = vec![2]));
    assert!(!ok(&|c| c.values = vec![1, 1]), "a column once");
    assert!(!ok(&|c| c.colors.clear()));
    assert!(ok(&|c| c.colors = vec![black(); MAX_CHART_COLORS]));
    assert!(!ok(&|c| c.colors = vec![black(); MAX_CHART_COLORS + 1]));
    assert!(!ok(
        &|c| c.colors[0] = Color::Srgb(Rgba::new(f32::NAN, 0.0, 0.0, 1.0))
    ));
    assert!(!ok(
        &|c| c.text.color = Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 2.0))
    ));
    assert!(!ok(&|c| c.text.size = 0.0) && !ok(&|c| c.text.size = f64::INFINITY));
    for wrong in [-0.1, 1.1, f64::NAN] {
        assert!(!ok(&|c| c.round = wrong) && !ok(&|c| c.gap = wrong) && !ok(&|c| c.hole = wrong));
    }
    assert!(ok(
        &|c| (c.gap, c.hole) = (ChartNode::MAX_GAP, ChartNode::MAX_HOLE)
    ));
    assert!(!ok(&|c| c.weight = 0.0) && !ok(&|c| c.weight = 201.0));
}

// S83-01: what is outside the data says nothing.
#[test]
fn s83_01_cells() {
    let c = chart();
    assert_eq!(
        (c.rows(), c.cell(1, 1), c.cell(1, 2), c.cell(9, 0)),
        (3, "12,5", "", "")
    );
}
