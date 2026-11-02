//! The identity of a registered engine component (02 §4.17, ARQ-98).

use crate::msg::MessageKey;

/// Implemented by every registrable component: effect, transition, codec...
pub trait Describe {
    /// The stable id, display name and version of this component.
    fn descriptor(&self) -> &Descriptor;
}

/// Id, display name and version of a component.
///
/// Capabilities are not here: each trait exposes its own typed form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Descriptor {
    /// Stable id, the key of the registry.
    pub id: &'static str,
    /// Catalog key of the display name (ARQ-98).
    pub name: MessageKey,
    /// Version of the component's behaviour.
    pub version: u32,
}

impl Descriptor {
    /// Plain data: nothing is validated.
    #[must_use]
    pub const fn new(id: &'static str, name: MessageKey, version: u32) -> Self {
        Self { id, name, version }
    }
}
