use gafira_doc::{Document, PageId};

fn rename(doc: &Document, id: PageId) {
    if let Some(page) = doc.page(id) {
        page.name.push('x');
    }
}

fn main() {}
