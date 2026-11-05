//! The registry convention (02 §4).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use gafira_doc::contract::Describe;
use gafira_doc::msg;
use gafira_doc::msg::{Message, ToMessage};

/// Why a [`Registry`] refused an item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// An item with this id is already registered.
    DuplicateId { id: &'static str },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId { id } => write!(f, "duplicate id `{id}`"),
        }
    }
}

impl std::error::Error for RegistryError {}

impl ToMessage for RegistryError {
    fn to_message(&self) -> Message {
        match self {
            Self::DuplicateId { id } => {
                msg!("error.registry.duplicate_id", id = Text((*id).into()))
            }
        }
    }
}

/// Items by stable id, in registration order. Built by value at the
/// composition root: no global registries (ARQ-31).
pub struct Registry<T: ?Sized> {
    items: Vec<Arc<T>>,
    by_id: BTreeMap<&'static str, usize>,
}

impl<T: ?Sized> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> Registry<T> {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            by_id: BTreeMap::new(),
        }
    }

    /// Number of registered items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when nothing is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<T>> {
        self.items.iter()
    }
}

impl<T: ?Sized + Describe> Registry<T> {
    /// Adds `item` under its descriptor id.
    ///
    /// # Errors
    /// [`RegistryError::DuplicateId`] when the id is taken; the registry is
    /// left unchanged.
    pub fn register(&mut self, item: Arc<T>) -> Result<(), RegistryError> {
        let id = item.descriptor().id;
        if self.by_id.contains_key(id) {
            return Err(RegistryError::DuplicateId { id });
        }
        self.by_id.insert(id, self.items.len());
        self.items.push(item);
        Ok(())
    }

    /// The item registered under `id`.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Arc<T>> {
        self.by_id.get(id).map(|&at| &self.items[at])
    }
}
