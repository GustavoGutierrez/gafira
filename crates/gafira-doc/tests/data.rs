//! Spec 104: the connected data table a document carries (§11).

use std::sync::Arc;

use gafira_doc::{DataColumn, DataProblem, DataRow, DataSet, Document, ValidationError};

fn set() -> DataSet {
    DataSet {
        name: "clientes".to_owned(),
        columns: vec![
            DataColumn::new(1, "Nombre completo", "nombre_completo"),
            DataColumn::new(2, "Curso", "curso"),
        ],
        rows: vec![
            Arc::new(DataRow::new(3, vec!["Ana Pérez".into(), "Rust".into()])),
            Arc::new(DataRow::new(4, vec!["00125".into(), String::new()])),
        ],
        next_id: 5,
        ..DataSet::default()
    }
}

// S104-01: a valid table passes; each broken rule is named.
#[test]
fn s104_01_data_rules() {
    assert_eq!(set().problem(), None);
    let mut bad = set();
    bad.columns[1].key = "nombre_completo".into();
    assert_eq!(bad.problem(), Some(DataProblem::DuplicateKey));
    let mut bad = set();
    bad.columns[1].key = "1curso".into();
    assert_eq!(bad.problem(), Some(DataProblem::InvalidKey));
    let mut bad = set();
    bad.columns[1].key = "cur so".into();
    assert_eq!(bad.problem(), Some(DataProblem::InvalidKey));
    let mut bad = set();
    bad.rows[1] = Arc::new(DataRow::new(4, vec!["x".into()]));
    assert_eq!(bad.problem(), Some(DataProblem::RowWidth));
    let mut bad = set();
    bad.rows[1] = Arc::new(DataRow::new(1, vec!["x".into(), "y".into()]));
    assert_eq!(bad.problem(), Some(DataProblem::DuplicateId));
    let mut bad = set();
    bad.next_id = 4;
    assert_eq!(bad.problem(), Some(DataProblem::IdNotBelowNext));
}

// S104-02: the key rule of §7.3 rule 6.
#[test]
fn s104_02_simple_keys() {
    for good in ["a", "_x", "nombre", "Curso_2", "A9"] {
        assert!(DataSet::is_key(good), "{good}");
    }
    for bad in ["", "9a", "ñandú", "a-b", "a b", "{{a}}"] {
        assert!(!DataSet::is_key(bad), "{bad}");
    }
}

// S104-03: the table travels inside the document and comes back the same,
// codes with leading zeros and empty cells included (§11, AC 7 and 8).
#[test]
fn s104_03_round_trip() {
    let doc = Document::default()
        .with_data(Some(Arc::new(set())))
        .unwrap();
    let json = doc.to_json().unwrap();
    assert!(json.contains("\"connected_data\""), "{json}");
    let back = Document::from_json(json.as_bytes()).unwrap();
    assert_eq!(back, doc);
    let data = back.data().unwrap();
    assert_eq!(data.rows[1].values[0], "00125");
    assert_eq!(data.rows[1].values[1], "");
}

// S104-04: a document without data writes no field and reads as before.
#[test]
fn s104_04_absent_by_default() {
    let doc = Document::default();
    let json = doc.to_json().unwrap();
    assert!(!json.contains("connected_data"), "{json}");
    assert_eq!(Document::from_json(json.as_bytes()).unwrap().data(), None);
}

// S104-05: a broken table makes the document invalid.
#[test]
fn s104_05_broken_table_is_refused() {
    let mut bad = set();
    bad.columns[0].key = String::new();
    assert_eq!(
        Document::default().with_data(Some(Arc::new(bad))),
        Err(ValidationError::InvalidData {
            problem: DataProblem::InvalidKey
        })
    );
}

// S104-06: lookups by key and by original name.
#[test]
fn s104_06_lookups() {
    let set = set();
    assert_eq!(set.column_by_key("curso").map(|c| c.id), Some(2));
    assert_eq!(set.column_by_name("Nombre completo").map(|c| c.id), Some(1));
    assert_eq!(set.value(0, 2), Some("Rust"));
    assert_eq!(set.value(1, 1), Some("00125"));
    assert_eq!(set.value(2, 1), None);
}
