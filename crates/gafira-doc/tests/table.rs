//! Tables in the document (spec 081, FD-21).

use std::sync::Arc;

use gafira_color::{Color, Rgba};
use gafira_doc::{
    CellMerge, CellStyle, Document, Dpi, ElementBase, Node, NodeId, NodeKind, Page, PageId,
    PageSize, Placement, ShapeStroke, TableNode, ValidationError,
};
use gafira_geom::{Length, Unit};

const NODE: NodeId = NodeId::new(2);

fn black() -> Color {
    Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 1.0))
}

fn table() -> TableNode {
    let mut table = TableNode::new(3, 4, (80.0, 30.0), &CellStyle::new("Inter", 16.0, black()));
    table.cell_mut(0, 1).expect("a cell").text = "Hola\nmundo".to_owned();
    let style = &mut table.cell_mut(1, 1).expect("a cell").style;
    style.fill = Some(black());
    style.borders.top = Some(ShapeStroke::new(black(), 2.0));
    table.merges.push(merge(0, 0, 2, 1));
    table.header = true;
    table
}

fn merge(row: usize, col: usize, rows: usize, cols: usize) -> CellMerge {
    CellMerge {
        row,
        col,
        rows,
        cols,
    }
}

fn document(table: TableNode) -> Result<Document, ValidationError> {
    let size = PageSize {
        width: Length::new(400.0, Unit::Px),
        height: Length::new(400.0, Unit::Px),
    };
    let mut page = Page::new(PageId::new(1), String::new(), size);
    let node = Node::new(
        ElementBase::new(NODE, Placement::new(10.0, 10.0, 320.0, 90.0)),
        NodeKind::Table(Box::new(table)),
    );
    page.nodes.insert(NODE, Arc::new(node));
    page.roots = vec![NODE];
    Document::from_parts(3, Dpi::new(96), vec![Arc::new(page)])
}

// S81-01: a table round-trips.
#[test]
fn s81_01_table_round_trip() {
    let doc = document(table()).expect("valid");
    let json = doc.to_json().expect("json");
    assert!(json.contains(r#""type":"table""#), "{json}");
    assert_eq!(Document::from_json(json.as_bytes()).expect("loads"), doc);
}

// S81-01: each limit, inside and outside.
#[test]
fn s81_01_table_validation() {
    let style = CellStyle::new("Inter", 16.0, black());
    let of = |rows, columns| document(TableNode::new(rows, columns, (8.0, 8.0), &style)).is_ok();
    assert!(of(1, 1) && of(50, 20));
    assert!(!of(0, 1) && !of(1, 0) && !of(51, 1) && !of(1, 21));
    let ok = |change: &dyn Fn(&mut TableNode)| {
        let mut table = table();
        change(&mut table);
        let result = document(table);
        if let Err(error) = &result {
            assert_eq!(*error, ValidationError::InvalidTable { node: NODE });
        }
        result.is_ok()
    };
    assert!(ok(&|_| ()));
    assert!(!ok(&|t| t.columns[0] = 0.0));
    assert!(!ok(&|t| t.rows[0] = f64::NAN));
    assert!(!ok(&|t| drop(t.cells.pop())));
    assert!(ok(&|t| t.spacing = TableNode::MAX_SPACING));
    assert!(!ok(&|t| t.spacing = TableNode::MAX_SPACING + 1.0));
    assert!(!ok(&|t| t.padding = -1.0));
    assert!(!ok(&|t| t.cells[0].style.size = 0.0));
    assert!(!ok(
        &|t| t.cells[0].style.borders.left = Some(ShapeStroke::new(black(), -1.0))
    ));
    assert!(!ok(
        &|t| t.cells[0].style.fill = Some(Color::Srgb(Rgba::new(0.0, 0.0, 0.0, 2.0)))
    ));
    assert!(ok(&|t| t.merges.push(merge(2, 0, 1, 4))));
    assert!(!ok(&|t| t.merges.push(merge(1, 0, 1, 2))), "over another");
    assert!(!ok(&|t| t.merges.push(merge(2, 3, 1, 2))), "past the edge");
    assert!(!ok(&|t| t.merges.push(merge(2, 3, 1, 1))), "a cell alone");
    assert!(!ok(&|t| t.merges.push(merge(usize::MAX, 1, 2, 2))));
}
