use gafira_doc::Document;
use gafira_edit::History;

fn main() {
    let history = History::new(Document::default()).unwrap();
    **history.document() = Document::default();
}
