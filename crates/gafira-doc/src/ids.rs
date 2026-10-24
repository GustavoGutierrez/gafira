//! Typed identifiers for nodes and pages.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            /// Wraps a raw value without validation (`u64::MAX` is representable).
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// The raw value.
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(
    /// Identifier of a node, unique inside its document.
    ///
    /// Distinct from [`PageId`]; passing one where the other is expected does
    /// not compile:
    ///
    /// ```compile_fail
    /// use gafira_doc::{NodeId, PageId};
    /// fn takes_node(_: NodeId) {}
    /// takes_node(PageId::new(1));
    /// ```
    NodeId
);
id_type!(
    /// Identifier of a page, unique inside its document.
    PageId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_accessors_and_display() {
        let (a, b) = (NodeId::new(2), NodeId::new(10));
        assert!(a < b);
        assert_eq!((a.get(), b.get()), (2, 10));
        assert_eq!(a.to_string(), "2");
        assert_eq!(b.to_string(), "10");
        assert_eq!(PageId::new(7).get(), 7);
        assert_eq!(PageId::new(7).to_string(), "7");
    }

    #[test]
    fn transparent_wire_form() {
        let five = serde_json::to_string(&NodeId::new(5)).unwrap();
        assert_eq!(five, "5");
        assert_eq!(
            serde_json::from_str::<NodeId>(&five).unwrap(),
            NodeId::new(5)
        );
        let max = serde_json::to_string(&NodeId::new(u64::MAX)).unwrap();
        assert_eq!(max, "18446744073709551615");
        assert_eq!(
            serde_json::from_str::<NodeId>(&max).unwrap().get(),
            u64::MAX
        );
        let page = serde_json::to_string(&PageId::new(u64::MAX)).unwrap();
        assert_eq!(page, "18446744073709551615");
        assert_eq!(
            serde_json::from_str::<PageId>(&page).unwrap().get(),
            u64::MAX
        );
    }

    #[test]
    fn hash_and_copy_are_available() {
        let set: std::collections::BTreeSet<NodeId> =
            [NodeId::new(3), NodeId::new(1), NodeId::new(3)].into();
        assert_eq!(set.len(), 2);
        let h = |v: NodeId| {
            use std::hash::{Hash, Hasher};
            let mut s = std::hash::DefaultHasher::new();
            v.hash(&mut s);
            s.finish()
        };
        assert_eq!(h(NodeId::new(4)), h(NodeId::new(4)));
    }
}
