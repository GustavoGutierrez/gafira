use gafira_doc::{Document, EditToken};

fn edit_shared(doc: &Document, token: &EditToken) {
    let _view = doc.edit(token);
}

fn main() {}
