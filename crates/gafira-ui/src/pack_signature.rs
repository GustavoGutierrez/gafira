//! Package signatures (spec 112): who published a `.gafpack`, checked
//! with Ed25519 against the publisher keys Gafira carries. Pure, so it is
//! tested without a disk.
//!
//! `META-INF/signature.json` holds the SHA-256 of every other file of the
//! package and an Ed25519 signature over a canonical text of them, so a
//! verified package is both from its publisher and unchanged since.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::packs::PackProblem;

/// Where the signature is in a package.
pub const SIGNATURE_PATH: &str = "META-INF/signature.json";

/// The folder a signature does not cover.
const UNSIGNED_DIR: &str = "META-INF/";

/// The `format` of a signature file.
pub const SIGNATURE_FORMAT: &str = "gafpack-signature";

/// The version of the signature file this Gafira reads.
pub const SIGNATURE_VERSION: u32 = 1;

/// A publisher key Gafira trusts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustedKey {
    /// The id a signature names it by.
    pub id: &'static str,
    /// The publisher it belongs to, as shown and as the author must read.
    pub publisher: &'static str,
    /// The Ed25519 public key, 64 hexadecimal digits.
    pub public: &'static str,
}

/// The publisher keys Gafira trusts (FIR-06).
pub const TRUSTED_KEYS: &[TrustedKey] = &[TrustedKey {
    id: "gafira-2026",
    publisher: "Gafira",
    public: "92d2192dad58b85572600dd5c540e07fef79787d063f73bc84f2e57aad5aff48",
}];

/// Who signed a package, as far as Gafira can tell.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PackSigner {
    /// No signature: the author is declared, not verified (FIR-01).
    #[default]
    Unsigned,
    /// Signed with a key Gafira does not know: not verified (FIR-05).
    Unknown { key_id: String },
    /// Signed by a publisher Gafira trusts, and unchanged since (FIR-06).
    Verified { publisher: String, key_id: String },
}

impl PackSigner {
    /// The verified publisher, if any.
    #[must_use]
    pub fn verified(&self) -> Option<&str> {
        match self {
            Self::Verified { publisher, .. } => Some(publisher),
            _ => None,
        }
    }
}

/// Whether the signature covers the file at `path`: every file but those
/// in `META-INF/`.
#[must_use]
pub fn signed_path(path: &str) -> bool {
    !path.starts_with(UNSIGNED_DIR)
}

/// The text a signature signs, for `files` (path to SHA-256).
#[must_use]
pub fn signed_message(key_id: &str, publisher: &str, files: &BTreeMap<String, String>) -> Vec<u8> {
    // A `BTreeMap<String, _>` iterates by the bytes of its paths.
    let mut text =
        format!("{SIGNATURE_FORMAT}/{SIGNATURE_VERSION}\nkey:{key_id}\npublisher:{publisher}\n");
    for (path, hash) in files {
        text.push_str(hash);
        text.push_str("  ");
        text.push_str(path);
        text.push('\n');
    }
    text.into_bytes()
}

/// The bytes written as hexadecimal digits, if they are exactly `N`.
fn from_hex<const N: usize>(hex: &str) -> Option<[u8; N]> {
    if hex.len() != 2 * N || !hex.is_ascii() {
        return None;
    }
    let mut out = [0u8; N];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

/// The Ed25519 public key written as 64 hexadecimal digits.
fn public_key(hex: &str) -> Option<ed25519_dalek::VerifyingKey> {
    ed25519_dalek::VerifyingKey::from_bytes(&from_hex::<32>(hex)?).ok()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignatureFile {
    #[serde(default)]
    format: String,
    #[serde(default)]
    version: u32,
    #[serde(default)]
    algorithm: String,
    #[serde(default)]
    key_id: String,
    #[serde(default)]
    publisher: String,
    #[serde(default)]
    files: BTreeMap<String, String>,
    #[serde(default)]
    signature: String,
}

/// Who signed the package whose signature file is `signature` (`None`
/// when it has none), `files` being the SHA-256 of each of its files the
/// signature covers and `author` the author its manifest declares,
/// against the keys `trusted` (FIR-01 to FIR-05).
///
/// # Errors
/// A signature that does not hold: the package is refused.
pub fn signer(
    signature: Option<&[u8]>,
    files: &BTreeMap<String, String>,
    author: &str,
    trusted: &[TrustedKey],
) -> Result<PackSigner, PackProblem> {
    let Some(bytes) = signature else {
        return Ok(PackSigner::Unsigned);
    };
    let bad = |about: &str| PackProblem::new("packs.bad_signature", about);
    let file: SignatureFile = serde_json::from_slice(bytes).map_err(|e| bad(&e.to_string()))?;
    if file.format != SIGNATURE_FORMAT {
        return Err(bad("format"));
    }
    if file.version != SIGNATURE_VERSION {
        return Err(bad("version"));
    }
    if file.algorithm != "ed25519" {
        return Err(bad("algorithm"));
    }
    // FIR-03: exactly the package's files, each with its hash.
    if let Some(path) = file
        .files
        .iter()
        .find(|(path, hash)| files.get(*path) != Some(*hash))
        .map(|(path, _)| path)
        .or_else(|| files.keys().find(|path| !file.files.contains_key(*path)))
    {
        return Err(PackProblem::new("packs.signature_mismatch", path.clone()));
    }
    let Some(key) = trusted.iter().find(|k| k.id == file.key_id) else {
        return Ok(PackSigner::Unknown {
            key_id: file.key_id,
        });
    };
    let public = public_key(key.public).ok_or_else(|| bad("keyId"))?;
    let raw = from_hex::<64>(&file.signature).ok_or_else(|| bad("signature"))?;
    let message = signed_message(&file.key_id, &file.publisher, &file.files);
    public
        .verify_strict(&message, &ed25519_dalek::Signature::from_bytes(&raw))
        .map_err(|_| bad("signature"))?;
    if file.publisher != key.publisher {
        return Err(bad("publisher"));
    }
    if author.trim() != key.publisher {
        return Err(PackProblem::new("packs.signature_author", author));
    }
    Ok(PackSigner::Verified {
        publisher: key.publisher.to_owned(),
        key_id: file.key_id,
    })
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer as _, SigningKey};

    use super::*;

    const TEST_SEED: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];
    const TEST_KEY: TrustedKey = TrustedKey {
        id: "test-key",
        publisher: "Test Studio",
        public: "03a107bff3ce10be1d70dd18e74bc09967e4d6309ba50d5f1ddc8664125531b8",
    };

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn files() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("manifest.json".to_owned(), "a".repeat(64)),
            ("assets/b.png".to_owned(), "b".repeat(64)),
        ])
    }

    /// A signature file for `files`, signed by `seed` as `key_id` of
    /// `publisher`.
    fn signed(
        seed: &[u8; 32],
        key_id: &str,
        publisher: &str,
        files: &BTreeMap<String, String>,
    ) -> serde_json::Value {
        let key = SigningKey::from_bytes(seed);
        let signature = key.sign(&signed_message(key_id, publisher, files));
        serde_json::json!({
            "format": "gafpack-signature", "version": 1, "algorithm": "ed25519",
            "keyId": key_id, "publisher": publisher, "files": files,
            "signature": hex(&signature.to_bytes()),
        })
    }

    fn check(
        value: &serde_json::Value,
        files: &BTreeMap<String, String>,
        author: &str,
    ) -> Result<PackSigner, PackProblem> {
        signer(
            Some(value.to_string().as_bytes()),
            files,
            author,
            &[TEST_KEY],
        )
    }

    fn key_of(result: Result<PackSigner, PackProblem>) -> &'static str {
        match result {
            Err(problem) => problem.key,
            Ok(_) => "ok",
        }
    }

    // FIR-01: a package without a signature is unsigned.
    #[test]
    fn fir_01_no_signature_is_unsigned() {
        assert_eq!(
            signer(None, &files(), "Anyone", &[TEST_KEY]),
            Ok(PackSigner::Unsigned)
        );
        assert_eq!(PackSigner::Unsigned.verified(), None);
    }

    // FIR-02: a signature that does not read or does not verify refuses
    // the package.
    #[test]
    fn fir_02_a_bad_signature_is_refused() {
        let good = signed(&TEST_SEED, "test-key", "Test Studio", &files());
        assert_eq!(
            key_of(signer(
                Some(b"{not json"),
                &files(),
                "Test Studio",
                &[TEST_KEY]
            )),
            "packs.bad_signature"
        );
        for (field, value) in [
            ("format", serde_json::json!("other")),
            ("version", serde_json::json!(2)),
            ("algorithm", serde_json::json!("rsa")),
            ("signature", serde_json::json!("00")),
            ("signature", serde_json::json!("zz".repeat(64))),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert_eq!(
                key_of(check(&bad, &files(), "Test Studio")),
                "packs.bad_signature",
                "{field}"
            );
        }
        // Signed by another key under the trusted id.
        let forged = signed(&[7; 32], "test-key", "Test Studio", &files());
        assert_eq!(
            key_of(check(&forged, &files(), "Test Studio")),
            "packs.bad_signature"
        );
    }

    // FIR-03: the files the signature lists are exactly the package's.
    #[test]
    fn fir_03_files_must_match_the_package() {
        let good = signed(&TEST_SEED, "test-key", "Test Studio", &files());
        let mut changed = files();
        changed.insert("assets/b.png".to_owned(), "c".repeat(64));
        let mut more = files();
        more.insert("assets/extra.png".to_owned(), "d".repeat(64));
        let mut fewer = files();
        fewer.remove("assets/b.png");
        for package in [changed, more, fewer] {
            assert_eq!(
                key_of(check(&good, &package, "Test Studio")),
                "packs.signature_mismatch"
            );
        }
    }

    // FIR-04: the signed publisher is the key's, and the author is it.
    #[test]
    fn fir_04_publisher_and_author_must_match_the_key() {
        let other_publisher = signed(&TEST_SEED, "test-key", "Someone Else", &files());
        assert_eq!(
            key_of(check(&other_publisher, &files(), "Someone Else")),
            "packs.bad_signature"
        );
        let good = signed(&TEST_SEED, "test-key", "Test Studio", &files());
        assert_eq!(
            key_of(check(&good, &files(), "Another Author")),
            "packs.signature_author"
        );
    }

    // FIR-05: a key Gafira does not know leaves the package unverified.
    #[test]
    fn fir_05_an_unknown_key_is_not_verified() {
        let unknown = signed(&[9; 32], "studio-x", "Studio X", &files());
        assert_eq!(
            check(&unknown, &files(), "Studio X"),
            Ok(PackSigner::Unknown {
                key_id: "studio-x".to_owned()
            })
        );
    }

    // FIR-06: a trusted signature over the same files verifies.
    #[test]
    fn fir_06_a_trusted_signature_verifies() {
        let good = signed(&TEST_SEED, "test-key", "Test Studio", &files());
        let found = check(&good, &files(), " Test Studio ").expect("verifies");
        assert_eq!(found.verified(), Some("Test Studio"));
        assert_eq!(
            found,
            PackSigner::Verified {
                publisher: "Test Studio".to_owned(),
                key_id: "test-key".to_owned()
            }
        );
        assert!(signed_path("assets/b.png") && !signed_path("META-INF/signature.json"));
    }

    // FIR-07: a signature made by gafpack.py (Python `cryptography`) over
    // the same canonical text verifies here.
    #[test]
    fn fir_07_python_vector_verifies() {
        let message = signed_message("test-key", "Test Studio", &files());
        let expected = format!(
            "gafpack-signature/1\nkey:test-key\npublisher:Test Studio\n{}  assets/b.png\n{}  manifest.json\n",
            "b".repeat(64),
            "a".repeat(64)
        );
        assert_eq!(String::from_utf8(message).expect("utf-8"), expected);
        let python = serde_json::json!({
            "format": "gafpack-signature", "version": 1, "algorithm": "ed25519",
            "keyId": "test-key", "publisher": "Test Studio", "files": files(),
            "signature": "313df2da372a071eae870bd515805dde69ccab59cb1b345b361fae5faa8e124feab3c18ce914ac701ef877f04da8e86792c3d18b52d476217ba2a3c0bc7d0f09",
        });
        assert_eq!(
            check(&python, &files(), "Test Studio")
                .expect("verifies")
                .verified(),
            Some("Test Studio")
        );
    }

    // The key Gafira ships is a valid Ed25519 public key.
    #[test]
    fn fir_06_the_shipped_keys_are_valid() {
        for key in TRUSTED_KEYS {
            assert!(public_key(key.public).is_some(), "{}", key.id);
        }
    }
}
