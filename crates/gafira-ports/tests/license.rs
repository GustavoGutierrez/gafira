//! License types and port (design P4, P6, P8; 02 §4.14).

use std::sync::Mutex;
use std::time::Duration;

use gafira_doc::msg::{MessageKey, ToMessage, is_valid_key};
use gafira_ports::common::UnixTime;
use gafira_ports::license::{
    LicenseError, LicenseKey, LicenseStatus, LicenseVerifier, NoLicensing, ReasonCode,
};

fn active() -> LicenseStatus {
    LicenseStatus::Active {
        expires_at: UnixTime(100),
        activation_id: "act-1".to_owned(),
    }
}

fn grace() -> LicenseStatus {
    LicenseStatus::OfflineGrace {
        token_expires_at: UnixTime(100),
        retry_after: Duration::from_secs(3600),
    }
}

// P8 `can_export` (D-12, O1 a); ARQ-92 needs `OfflineGrace` to export.
#[test]
fn p8_can_export_only_for_active_and_offline_grace() {
    assert!(active().can_export());
    assert!(grace().can_export());
    for status in [
        LicenseStatus::Expired,
        LicenseStatus::Unlicensed,
        LicenseStatus::Invalid {
            reason: ReasonCode::Revoked,
        },
        LicenseStatus::Invalid {
            reason: ReasonCode::TokenTampered,
        },
    ] {
        assert!(!status.can_export(), "{status:?}");
    }
}

// ARQ-94: `Debug` leaks no key byte, in any wrapper. The key uses only
// characters the type name and the redaction mark do not spell.
#[test]
fn arq_94_license_key_debug_leaks_nothing() {
    let secret = "QZXJ-7W9V-QZ4J";
    let key = LicenseKey::new(secret);
    let shown = [
        format!("{key:?}"),
        format!("{key:#?}"),
        format!("{:?}", Some(&key)),
    ];
    for text in shown {
        for ch in secret.chars().filter(|c| *c != '-') {
            assert!(!text.contains(ch), "{ch:?} leaked in {text}");
        }
    }
}

#[test]
fn p8_license_key_expose_gives_the_key() {
    assert_eq!(LicenseKey::new("k-1").expose(), "k-1");
    assert_eq!(LicenseKey::new(String::from("k-2")).expose(), "k-2");
}

// ARQ-102 (ADR 0012): one fixture per variant.
#[test]
fn arq_102_license_error_messages() {
    let cases = [
        (LicenseError::Network, "error.license.network"),
        (
            LicenseError::Rejected(ReasonCode::DeviceLimit),
            "error.license.rejected",
        ),
        (LicenseError::Storage, "error.license.storage"),
    ];
    for (err, key) in cases {
        let m = err.to_message();
        assert!(is_valid_key(m.key.0), "{:?}", m.key);
        assert_eq!(m.key, MessageKey(key));
        assert!(
            m.args.is_empty(),
            "no detail, no payload text: {:?}",
            m.args
        );
    }
}

// ARQ-20: the port is object-safe; `status` is pure and takes the time.
struct Fixed(Mutex<LicenseStatus>);

impl LicenseVerifier for Fixed {
    fn status(&self, now: UnixTime) -> LicenseStatus {
        match &*self.0.lock().unwrap() {
            LicenseStatus::Active { expires_at, .. } if now > *expires_at => LicenseStatus::Expired,
            other => other.clone(),
        }
    }
    fn activate(&self, key: &LicenseKey, _: Option<&str>) -> Result<LicenseStatus, LicenseError> {
        if key.expose().is_empty() {
            return Err(LicenseError::Rejected(ReasonCode::UnknownKey));
        }
        *self.0.lock().unwrap() = active();
        Ok(active())
    }
    fn validate(&self, now: UnixTime) -> Result<LicenseStatus, LicenseError> {
        Ok(self.status(now))
    }
    fn deactivate(&self) -> Result<(), LicenseError> {
        *self.0.lock().unwrap() = LicenseStatus::Unlicensed;
        Ok(())
    }
}

#[test]
fn arq_20_license_verifier_is_object_safe() {
    let v: Box<dyn LicenseVerifier> = Box::new(Fixed(Mutex::new(LicenseStatus::Unlicensed)));
    assert!(!v.status(UnixTime(0)).can_export());
    assert_eq!(
        v.activate(&LicenseKey::new(""), None),
        Err(LicenseError::Rejected(ReasonCode::UnknownKey))
    );
    assert!(
        v.activate(&LicenseKey::new("k"), Some("laptop"))
            .unwrap()
            .can_export()
    );
    assert_eq!(v.status(UnixTime(101)), LicenseStatus::Expired);
    v.deactivate().unwrap();
    assert_eq!(v.validate(UnixTime(0)).unwrap(), LicenseStatus::Unlicensed);
}

// Spec 010, S10-01: with licensing off nothing is required and export is open.
#[test]
fn s10_01_not_required_exports_and_asks_for_nothing() {
    assert!(LicenseStatus::NotRequired.can_export());
    assert!(!LicenseStatus::NotRequired.is_required());
    for status in [
        active(),
        grace(),
        LicenseStatus::Expired,
        LicenseStatus::Unlicensed,
        LicenseStatus::Invalid {
            reason: ReasonCode::Revoked,
        },
    ] {
        assert!(status.is_required(), "{status:?}");
    }
}

// Spec 010, S10-02: `NoLicensing` is licensing switched off (D-58).
#[test]
fn s10_02_no_licensing_is_not_required_and_refuses_every_action() {
    let off = NoLicensing;
    for now in [UnixTime(0), UnixTime(i64::MAX)] {
        assert_eq!(off.status(now), LicenseStatus::NotRequired);
    }
    assert_eq!(
        off.activate(&LicenseKey::new("k"), None),
        Err(LicenseError::Disabled)
    );
    assert_eq!(off.validate(UnixTime(1)), Err(LicenseError::Disabled));
    assert_eq!(off.deactivate(), Err(LicenseError::Disabled));
    assert_eq!(
        LicenseError::Disabled.to_message().key,
        MessageKey("error.license.disabled")
    );
}
