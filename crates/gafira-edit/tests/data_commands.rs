//! Spec 104: `SetData` puts, merges and takes back the connected data.

use std::sync::Arc;
use std::time::Duration;

use gafira_doc::{DataColumn, DataRow, DataSet, Document};
use gafira_edit::{Applied, History, SetData};

fn set(value: &str) -> Arc<DataSet> {
    Arc::new(DataSet {
        columns: vec![DataColumn::new(1, "Nombre", "nombre")],
        rows: vec![Arc::new(DataRow::new(2, vec![value.to_owned()]))],
        next_id: 3,
        ..DataSet::default()
    })
}

// S104-07: one step puts the table, undo takes it away, redo brings it back.
#[test]
fn s104_07_set_data_undoes() {
    let mut history = History::new(Document::default()).unwrap();
    let applied = history
        .execute(Box::new(SetData::new(Some(set("Ana")))), Duration::ZERO)
        .unwrap();
    assert_eq!(applied, Applied::Changed);
    assert_eq!(history.document().data().unwrap().rows[0].values[0], "Ana");
    history.undo().unwrap();
    assert_eq!(history.document().data(), None);
    history.redo().unwrap();
    assert!(history.document().data().is_some());
}

// S104-08: typing a cell is one step; an invalid table is refused.
#[test]
fn s104_08_typing_merges_and_bad_tables_fail() {
    let mut history = History::new(Document::default()).unwrap();
    for (k, value) in ["A", "An", "Ana"].iter().enumerate() {
        let at = Duration::from_millis(100 * k as u64);
        history
            .execute(Box::new(SetData::new(Some(set(value)))), at)
            .unwrap();
    }
    assert_eq!(history.len(), 1);
    history.undo().unwrap();
    assert_eq!(history.document().data(), None);
    let mut bad = (*set("x")).clone();
    bad.columns[0].key = "1".into();
    let refused = history.execute(Box::new(SetData::new(Some(Arc::new(bad)))), Duration::ZERO);
    assert!(refused.is_err());
}
