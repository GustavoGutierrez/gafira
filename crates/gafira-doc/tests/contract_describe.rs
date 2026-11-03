//! `Describe` and `Descriptor` through the public contract (ARQ-98).

use gafira_doc::contract::{Describe, Descriptor};
use gafira_doc::msg::MessageKey;

struct Blur;

impl Describe for Blur {
    fn descriptor(&self) -> &Descriptor {
        const D: Descriptor = Descriptor::new("blur.gaussian", MessageKey("effect.blur"), 3);
        &D
    }
}

// ARQ-98
#[test]
fn arq_98_a_descriptor_keeps_id_label_key_and_version_behind_describe() {
    let component: &dyn Describe = &Blur;
    let d = component.descriptor();
    assert_eq!(d.id, "blur.gaussian");
    assert_eq!(d.name, MessageKey("effect.blur"));
    assert_eq!(d.version, 3);
    assert_ne!(
        *d,
        Descriptor::new("blur.gaussian", MessageKey("effect.blur"), 4)
    );
}
