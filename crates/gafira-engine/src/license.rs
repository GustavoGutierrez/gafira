//! `LicenseService` of 02 §5.4 line 1363 and spec 010: the licence state the
//! UI shows, the actions that change it, and `LicenseChanged` on every change
//! (02 §4.14).
//!
//! Whether the build asks for a licence at all is the injected verifier's
//! choice, never this service's (D-58): with `ports::license::NoLicensing` the
//! state is `NotRequired`, `can_export` holds and every action is refused.
//!
//! `activate`, `validate` and `deactivate` block on the network inside the
//! verifier; the engine spawns no thread (ADR 0016), so the host calls them
//! off the UI thread. A failed call never changes the state (ARQ-92), and the
//! licence never takes part in saving (D-55).

use std::sync::{Arc, Mutex, MutexGuard};

use gafira_ports::license::{LicenseKey, LicenseStatus, LicenseVerifier};
use gafira_ports::system::Clock;

use crate::engine::EngineError;
use crate::event::{EngineEvent, EventBus};

/// The licence state and its actions (02 §5.4 line 1363).
pub struct LicenseService {
    verifier: Arc<dyn LicenseVerifier>,
    clock: Arc<dyn Clock>,
    bus: EventBus,
    status: Mutex<LicenseStatus>,
}

impl LicenseService {
    /// The verifier's offline state, read but not announced until
    /// [`LicenseService::start`].
    pub(crate) fn over(
        verifier: Arc<dyn LicenseVerifier>,
        clock: Arc<dyn Clock>,
        bus: EventBus,
    ) -> Self {
        let status = verifier.status(clock.now());
        Self {
            verifier,
            clock,
            bus,
            status: Mutex::new(status),
        }
    }

    /// Read the stored licence without network and publish the first
    /// `LicenseChanged` (S10-03). The composition root calls it once at
    /// startup.
    pub fn start(&self) -> LicenseStatus {
        let mut status = self.lock();
        *status = self.verifier.status(self.clock.now());
        self.bus
            .publish(EngineEvent::LicenseChanged(status.clone()));
        status.clone()
    }

    /// The state in force.
    #[must_use]
    pub fn status(&self) -> LicenseStatus {
        self.lock().clone()
    }

    /// Whether exporting is allowed now (02 §4.14; D-12, D-55, D-58).
    #[must_use]
    pub fn can_export(&self) -> bool {
        self.lock().can_export()
    }

    /// Whether this build asks for a licence at all (D-58): the UI hides the
    /// activation screen and the licence row of Settings when it does not.
    #[must_use]
    pub fn is_required(&self) -> bool {
        self.lock().is_required()
    }

    /// Activate `key` on this device, with an optional device `label`
    /// (network). Publishes `LicenseChanged` when the state changes.
    ///
    /// # Errors
    /// [`EngineError::License`] when the call fails or the server rejects the
    /// key; the state stays as it was.
    pub fn activate(
        &self,
        key: &LicenseKey,
        label: Option<&str>,
    ) -> Result<LicenseStatus, EngineError> {
        let next = self.verifier.activate(key, label)?;
        Ok(self.adopt(next))
    }

    /// Re-validate the stored token (network). Publishes `LicenseChanged`
    /// when the state changes.
    ///
    /// # Errors
    /// [`EngineError::License`] when the call fails; the state stays as it
    /// was, so a network failure never takes a valid token away (ARQ-92).
    pub fn validate(&self) -> Result<LicenseStatus, EngineError> {
        let next = self.verifier.validate(self.clock.now())?;
        Ok(self.adopt(next))
    }

    /// Release this device (network), then read the state again offline and
    /// publish it when it changed.
    ///
    /// # Errors
    /// [`EngineError::License`] when the call fails; the state stays as it
    /// was.
    pub fn deactivate(&self) -> Result<LicenseStatus, EngineError> {
        self.verifier.deactivate()?;
        let next = self.verifier.status(self.clock.now());
        Ok(self.adopt(next))
    }

    /// Put `next` in force and publish it when it differs from the state in
    /// force. The network call ran before the lock, so a slow server never
    /// blocks a reader of the state.
    fn adopt(&self, next: LicenseStatus) -> LicenseStatus {
        let mut status = self.lock();
        if *status != next {
            *status = next.clone();
            self.bus.publish(EngineEvent::LicenseChanged(next.clone()));
        }
        next
    }

    /// The state, ignoring lock poisoning as `session.rs` does.
    fn lock(&self) -> MutexGuard<'_, LicenseStatus> {
        match self.status.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_ports::common::UnixTime;
    use gafira_ports::license::{
        LicenseError, LicenseKey, LicenseStatus, LicenseVerifier, NoLicensing, ReasonCode,
    };

    use crate::engine::EngineError;
    use crate::event::EngineEvent;
    use crate::fakes::{ManualClock, ScriptedLicense, drain, engine_with_license};

    fn active() -> LicenseStatus {
        LicenseStatus::Active {
            expires_at: UnixTime(2_000_000_000),
            activation_id: "act-1".to_owned(),
        }
    }

    fn licence_events(events: &[EngineEvent]) -> Vec<LicenseStatus> {
        events
            .iter()
            .filter_map(|event| match event {
                EngineEvent::LicenseChanged(status) => Some(status.clone()),
                _ => None,
            })
            .collect()
    }

    fn scripted(
        status: LicenseStatus,
        outcomes: Vec<Result<LicenseStatus, LicenseError>>,
    ) -> Arc<ScriptedLicense> {
        Arc::new(ScriptedLicense::new(status, outcomes))
    }

    #[test]
    fn s10_03_start_reads_offline_with_the_clock_and_publishes_once() {
        let verifier = scripted(active(), vec![]);
        let engine = engine_with_license(Arc::clone(&verifier) as Arc<dyn LicenseVerifier>);

        assert_eq!(engine.license().start(), active());

        assert_eq!(engine.license().status(), active());
        assert!(engine.license().can_export());
        assert!(engine.license().is_required());
        assert!(verifier.nows().iter().all(|now| *now == ManualClock::NOW));
        let events = drain(&engine);
        assert_eq!(
            events.len(),
            1,
            "start publishes one event and nothing else"
        );
        assert_eq!(licence_events(&events), [active()]);
    }

    #[test]
    fn s10_04_activate_adopts_the_new_state_and_publishes_it() {
        let verifier = scripted(LicenseStatus::Unlicensed, vec![Ok(active())]);
        let engine = engine_with_license(verifier);
        engine.license().start();
        drain(&engine);
        assert!(!engine.license().can_export(), "unlicensed exports nothing");

        let status = engine
            .license()
            .activate(&LicenseKey::new("KEY"), Some("laptop"))
            .expect("the key activates");

        assert_eq!(status, active());
        assert_eq!(engine.license().status(), active());
        assert!(engine.license().can_export());
        assert_eq!(licence_events(&drain(&engine)), [active()]);
    }

    #[test]
    fn s10_04_a_validate_that_keeps_the_state_publishes_nothing() {
        let verifier = scripted(active(), vec![Ok(active())]);
        let engine = engine_with_license(Arc::clone(&verifier) as Arc<dyn LicenseVerifier>);
        engine.license().start();
        drain(&engine);

        assert_eq!(engine.license().validate().expect("validates"), active());

        assert_eq!(verifier.nows().last(), Some(&ManualClock::NOW));
        assert!(drain(&engine).is_empty(), "same state, no event");
    }

    #[test]
    fn s10_05_a_failed_call_keeps_the_state_and_publishes_nothing() {
        let verifier = scripted(
            active(),
            vec![
                Err(LicenseError::Network),
                Err(LicenseError::Rejected(ReasonCode::UnknownKey)),
                Err(LicenseError::Network),
            ],
        );
        let engine = engine_with_license(verifier);
        engine.license().start();
        drain(&engine);

        let validated = engine.license().validate();
        let activated = engine.license().activate(&LicenseKey::new("OTHER"), None);
        let deactivated = engine.license().deactivate();

        assert!(matches!(
            validated,
            Err(EngineError::License(LicenseError::Network))
        ));
        assert!(matches!(
            activated,
            Err(EngineError::License(LicenseError::Rejected(
                ReasonCode::UnknownKey
            )))
        ));
        assert!(matches!(
            deactivated,
            Err(EngineError::License(LicenseError::Network))
        ));
        assert_eq!(engine.license().status(), active());
        assert!(engine.license().can_export(), "ARQ-92");
        assert!(drain(&engine).is_empty(), "nothing is published");
    }

    #[test]
    fn s10_06_deactivate_reads_the_state_again_and_publishes_it() {
        let verifier = scripted(active(), vec![Ok(LicenseStatus::Unlicensed)]);
        let engine = engine_with_license(verifier);
        engine.license().start();
        drain(&engine);

        let status = engine.license().deactivate().expect("released");

        assert_eq!(status, LicenseStatus::Unlicensed);
        assert_eq!(engine.license().status(), LicenseStatus::Unlicensed);
        assert!(!engine.license().can_export());
        assert_eq!(licence_events(&drain(&engine)), [LicenseStatus::Unlicensed]);
    }

    #[test]
    fn s10_07_with_licensing_off_nothing_is_asked() {
        let engine = engine_with_license(Arc::new(NoLicensing));

        assert_eq!(engine.license().start(), LicenseStatus::NotRequired);
        drain(&engine);

        assert!(engine.license().can_export());
        assert!(!engine.license().is_required());
        assert!(matches!(
            engine.license().activate(&LicenseKey::new("KEY"), None),
            Err(EngineError::License(LicenseError::Disabled))
        ));
        assert!(matches!(
            engine.license().validate(),
            Err(EngineError::License(LicenseError::Disabled))
        ));
        assert_eq!(engine.license().status(), LicenseStatus::NotRequired);
        assert!(
            drain(&engine).is_empty(),
            "a refused action publishes nothing"
        );
    }
}
