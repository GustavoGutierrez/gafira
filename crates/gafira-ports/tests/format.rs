//! `format` import side (design P4, P8; 02 §4.5; FD-31, ARQ-102).

use gafira_doc::ContentHash;
use gafira_doc::contract::Cancelled;
use gafira_doc::msg::{MessageKey, MsgArg, ToMessage, is_valid_key};
use gafira_ports::common::{Confidence, LegalClass};
use gafira_ports::format::{ByteSignature, FormatDescriptor, FormatKind, ImportError};
use gafira_ports::store::StoreError;

const PNG: &[u8] = &[0x89, b'P', b'N', b'G'];

const fn sig(offset: usize, bytes: &'static [u8]) -> ByteSignature {
    ByteSignature { offset, bytes }
}

fn descriptor(signatures: &'static [ByteSignature]) -> FormatDescriptor {
    FormatDescriptor {
        id: "demo",
        extensions: &["demo"],
        mime: &["image/demo"],
        signatures,
        kind: FormatKind::Raster,
        legal: LegalClass::RoyaltyFree,
    }
}

// FD-31, P8 `ByteSignature::matches`: one row per rule.
#[test]
fn p8_signature_matches_table() {
    let ftyp = sig(4, b"ftyp");
    let cases: Vec<(&str, ByteSignature, &[u8], bool)> = vec![
        ("exact at 0", sig(0, PNG), PNG, true),
        (
            "trailing bytes ignored",
            sig(0, PNG),
            &[0x89, b'P', b'N', b'G', 1, 2],
            true,
        ),
        ("at offset", ftyp, b"\0\0\0\x18ftypmp42", true),
        (
            "last byte differs",
            sig(0, PNG),
            &[0x89, b'P', b'N', b'H'],
            false,
        ),
        (
            "first byte differs",
            sig(0, PNG),
            &[0x88, b'P', b'N', b'G'],
            false,
        ),
        (
            "head one byte short",
            sig(0, PNG),
            &[0x89, b'P', b'N'],
            false,
        ),
        ("head empty", sig(0, PNG), &[], false),
        ("head ends before offset", ftyp, b"\0\0", false),
        ("head ends inside bytes", ftyp, b"\0\0\0\0fty", false),
        ("offset overflows", sig(usize::MAX, PNG), PNG, false),
        (
            "offset + len overflows",
            sig(usize::MAX - 1, PNG),
            PNG,
            false,
        ),
        ("empty bytes, empty head", sig(0, b""), &[], false),
        ("empty bytes, some head", sig(0, b""), PNG, false),
        ("empty bytes, offset past head", sig(99, b""), PNG, false),
    ];
    for (name, signature, head, want) in cases {
        assert_eq!(signature.matches(head), want, "{name}");
    }
}

// FD-31, P8 `sniff_signatures`.
#[test]
fn p8_sniff_signatures_high_on_any_match_else_none() {
    static SIGS: [ByteSignature; 2] = [sig(0, PNG), sig(4, b"ftyp")];
    let d = descriptor(&SIGS);
    assert_eq!(d.sniff_signatures(PNG), Confidence::High, "first signature");
    assert_eq!(
        d.sniff_signatures(b"\0\0\0\0ftyp"),
        Confidence::High,
        "second signature"
    );
    assert_eq!(
        d.sniff_signatures(b"GIF89a"),
        Confidence::None,
        "no signature matches"
    );
    assert_eq!(
        descriptor(&[]).sniff_signatures(PNG),
        Confidence::None,
        "no signatures"
    );
}

// ARQ-102 (ADR 0012, P6): one fixture per variant.
#[test]
fn arq_102_import_error_messages() {
    let hash = ContentHash([0xAB; 32]);
    let own = [
        (ImportError::Unsupported, "error.import.unsupported"),
        (
            ImportError::Corrupt {
                detail: "secret detail".into(),
            },
            "error.import.corrupt",
        ),
        (
            ImportError::Io(std::io::Error::other("secret detail")),
            "error.import.io",
        ),
    ];
    for (error, key) in own {
        let m = error.to_message();
        assert!(is_valid_key(m.key.0), "{:?}", m.key);
        assert_eq!(m.key, MessageKey(key));
        assert!(
            m.args.is_empty(),
            "{key}: library text never becomes an argument"
        );
    }
    // Wrapped errors delegate their message.
    let store = ImportError::Store(StoreError::NotFound { hash }).to_message();
    assert_eq!(store, StoreError::NotFound { hash }.to_message());
    assert_eq!(store.args, vec![("hash", MsgArg::Text("ab".repeat(32)))]);
    let cancelled = ImportError::Cancelled(Cancelled).to_message();
    assert_eq!(cancelled, Cancelled.to_message());
    assert_eq!(cancelled.key, MessageKey("error.job.cancelled"));
}
