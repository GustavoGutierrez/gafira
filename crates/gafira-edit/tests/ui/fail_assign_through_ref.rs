use gafira_doc::Document;

fn replace(doc: &Document) {
    *doc = Document::default();
}

fn main() {
    replace(&Document::default());
}
