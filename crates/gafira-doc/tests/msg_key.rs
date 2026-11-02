//! The message key grammar (02 §14.2, ARQ-107), through the public contract.

use gafira_doc::msg::is_valid_key;

// ARQ-107
#[test]
fn arq_107_is_valid_key_follows_the_grammar() {
    let accepted = [
        "edit.set_property",
        "error.format.too_new",
        "a.b2",
        "a_1.b_2.c_3",
    ];
    for k in accepted {
        assert!(is_valid_key(k), "should accept {k:?}");
    }
    let rejected = [
        "",
        "a",
        ".a",
        "a.",
        "a..b",
        "a.b.c.d",
        "A.b",
        "a.B",
        "a.2b",
        "2a.b",
        "a b.c",
        "a.é",
        "a.bB",
        "aB.c",
        "a.bé",
        "é.b",
        "a.b\u{7f}",
        "a.b.",
        "a.b..c",
        "_a.b",
        "a-b.c",
        "a.b.c.d.e",
        ".",
        "..",
        "a.b.2",
    ];
    for k in rejected {
        assert!(!is_valid_key(k), "should reject {k:?}");
    }
}

// ARQ-107: the character class is exactly [a-z0-9_]; bytes next to each range
// (`/` `:` `` ` `` `{` `@` `[` `^` `-` ...) must not pass, in any position.
#[test]
fn arq_107_only_the_class_bytes_are_accepted_in_every_position() {
    for b in 0u8..=255 {
        let Some(c) = char::from_u32(u32::from(b)) else {
            continue;
        };
        let in_class = b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_';
        let start_class = b.is_ascii_lowercase();
        // Start of the first segment, of a later segment, and continuation.
        let first = format!("{c}.b");
        let later = format!("a.{c}");
        let cont_mid = format!("a{c}.b");
        let cont_end = format!("a.b{c}");
        assert_eq!(is_valid_key(&first), start_class, "first {first:?}");
        assert_eq!(is_valid_key(&later), start_class, "later {later:?}");
        assert_eq!(is_valid_key(&cont_mid), in_class, "mid {cont_mid:?}");
        assert_eq!(is_valid_key(&cont_end), in_class, "end {cont_end:?}");
    }
}
