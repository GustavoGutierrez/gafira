//! Plain types shared by several ports.

/// How sure a sniffer is about a file format. Ordered by declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    None,
    Low,
    Medium,
    High,
}

/// The legal class of a codec (02 §4.8, D-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LegalClass {
    /// Free software with no royalty.
    RoyaltyFree,
    /// Provided by the operating system.
    OsLicensed,
    /// Bound by active patents: never selected.
    Encumbered,
}

/// A size in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(pub u64);

/// Seconds since the Unix epoch, as `MsgArg::Timestamp`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixTime(pub i64);
