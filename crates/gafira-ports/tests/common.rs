//! Plain types shared by several ports (design P4, `common`).

use gafira_ports::common::{Bytes, Confidence, LegalClass, UnixTime};

// FD-31 (technique): confidence orders by declaration, `None` lowest.
#[test]
fn fd_31_confidence_orders_by_declaration() {
    let order = [
        Confidence::None,
        Confidence::Low,
        Confidence::Medium,
        Confidence::High,
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(order.iter().max(), Some(&Confidence::High));
}

// V-5 (type): the three legal classes of 05 FV-02, and no more.
#[test]
fn v_5_legal_class_has_three_classes() {
    let name = |c: LegalClass| match c {
        LegalClass::RoyaltyFree => "royalty_free",
        LegalClass::OsLicensed => "os_licensed",
        LegalClass::Encumbered => "encumbered",
    };
    let all = [
        LegalClass::RoyaltyFree,
        LegalClass::OsLicensed,
        LegalClass::Encumbered,
    ];
    let names: Vec<_> = all.into_iter().map(name).collect();
    assert_eq!(names, ["royalty_free", "os_licensed", "encumbered"]);
}

// P4: byte counts and seconds since the Unix epoch are plain wrappers.
#[test]
fn p4_bytes_and_unix_time_wrap_plain_numbers() {
    assert_eq!(Bytes(1 << 30).0, 1_073_741_824);
    assert!(UnixTime(-1) < UnixTime(0));
}
