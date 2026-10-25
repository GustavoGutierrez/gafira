//! Wire form of a page arena: an array of nodes in ascending id order.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serializer};

use crate::{Node, NodeId};

pub(crate) fn serialize<S: Serializer>(
    nodes: &BTreeMap<NodeId, Arc<Node>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(nodes.values())
}

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<NodeId, Arc<Node>>, D::Error> {
    let mut arena = BTreeMap::new();
    for node in Vec::<Arc<Node>>::deserialize(deserializer)? {
        let id = node.base.id;
        if arena.insert(id, node).is_some() {
            return Err(D::Error::custom(format!("duplicate node id {}", id.get())));
        }
    }
    Ok(arena)
}
