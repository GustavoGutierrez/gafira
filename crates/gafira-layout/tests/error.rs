//! ARQ-110: an error names the input at fault.

use gafira_doc::NodeId;
use gafira_layout::LayoutError;

#[test]
fn arq_110_every_error_says_what_failed_and_names_its_input() {
    let field = "bleed";
    let named = [
        LayoutError::NonFinite { field },
        LayoutError::InvalidBox { field },
        LayoutError::OutOfRange { field },
    ];
    for e in named {
        assert!(e.to_string().contains(field), "{e:?}");
    }
    let id = NodeId::new(7);
    for e in [LayoutError::UnknownNode(id), LayoutError::Cycle(id)] {
        assert!(e.to_string().contains('7'), "{e:?}");
    }
    assert!(!LayoutError::NoArea.to_string().is_empty());
}
