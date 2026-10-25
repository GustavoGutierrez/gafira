use gafira_doc::{NodeId, Page};

fn hide(page: &Page, id: NodeId) {
    if let Some(node) = page.node(id) {
        node.children.clear();
    }
}

fn main() {}
