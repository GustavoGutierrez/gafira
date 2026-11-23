//! Project identities: random RFC 9562 version-4 UUIDs (D-63).

use gafira_doc::ProjectId;
use gafira_ports::system::IdSource;

/// Mints version-4 UUIDs from the OS random source.
#[derive(Clone, Copy, Debug, Default)]
pub struct RandomIds;

/// Sets the version (4) and the variant (RFC 9562) bits on 16 random bytes.
fn version_4(mut bytes: [u8; 16]) -> ProjectId {
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    ProjectId::from_u128(u128::from_be_bytes(bytes))
}

impl IdSource for RandomIds {
    fn project_id(&self) -> ProjectId {
        let mut bytes = [0_u8; 16];
        // The OS source does not fail on the three targets (ARQ-03); if it
        // ever did, the version bits still make the id non-nil, and the
        // clock-free fallback keeps ids distinct within the process.
        if getrandom::fill(&mut bytes).is_err() {
            bytes = fallback();
        }
        version_4(bytes)
    }
}

fn fallback() -> [u8; 16] {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut bytes = [0_u8; 16];
    bytes[8..].copy_from_slice(&n.to_be_bytes());
    bytes[..8].copy_from_slice(&u64::from(std::process::id()).to_be_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s18_03_ids_are_version_4_distinct_and_never_nil() {
        let ids = RandomIds;
        let a = ids.project_id();
        let b = ids.project_id();
        assert_ne!(a, b);
        for id in [a, b, version_4([0; 16])] {
            let text = id.to_string();
            assert!(!id.is_nil());
            assert_eq!(&text[14..15], "4", "{text}");
            assert!(matches!(&text[19..20], "8" | "9" | "a" | "b"), "{text}");
        }
    }
}
