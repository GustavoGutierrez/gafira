//! `SettingsService` of 02 §5.4 line 1364 and spec 009: the user's settings,
//! persisted through `ports::SettingsRepository`, and the interface language
//! resolved by `ports::LocaleResolver` and announced with `LocaleChanged`
//! (02 §5.2 line 1333, ARQ-101).
//!
//! The order is always persist, then update, then publish: a save that fails
//! leaves the settings, the locale and the bus as they were (S9-06). The
//! service never touches the session, so a language change keeps the open
//! document and its history (S9-08).

use std::sync::{Arc, Mutex, MutexGuard};

use gafira_ports::i18n::{LanguagePreference, LocaleResolver, ResolvedLocale};
use gafira_ports::settings::{Settings, SettingsRepository};

use crate::engine::{EngineError, SettingsPorts};
use crate::event::{EngineEvent, EventBus};

/// What the service holds between calls: the settings in force and the
/// locale their language resolved to.
struct State {
    settings: Settings,
    resolved: ResolvedLocale,
}

/// The settings and the interface language (02 §5.4 line 1364).
pub struct SettingsService {
    repository: Arc<dyn SettingsRepository>,
    resolver: Arc<dyn LocaleResolver>,
    bus: EventBus,
    state: Mutex<State>,
}

impl SettingsService {
    /// The defaults, resolved but not announced: nothing is read or published
    /// until [`SettingsService::start`].
    pub(crate) fn over(ports: &SettingsPorts, bus: EventBus) -> Self {
        let settings = Settings::default();
        let resolved = ports.resolver.resolve(settings.language);
        Self {
            repository: Arc::clone(&ports.repository),
            resolver: Arc::clone(&ports.resolver),
            bus,
            state: Mutex::new(State { settings, resolved }),
        }
    }

    /// Load the stored settings, resolve their language and publish the first
    /// `LocaleChanged` (02 §5.2 line 1333). The composition root calls it once
    /// at startup.
    ///
    /// # Errors
    /// [`EngineError::Settings`] when the store cannot be read. The engine is
    /// still usable: the defaults are in force and their `LocaleChanged` was
    /// published, so the UI has a language; the error is for the log (S9-04).
    pub fn start(&self) -> Result<ResolvedLocale, EngineError> {
        let mut state = self.lock();
        let (settings, failure) = match self.repository.load() {
            Ok(settings) => (settings, None),
            Err(e) => (Settings::default(), Some(e)),
        };
        state.settings = settings;
        let resolved = self.announce(&mut state);
        match failure {
            None => Ok(resolved),
            Some(e) => Err(e.into()),
        }
    }

    /// The settings in force.
    #[must_use]
    pub fn get(&self) -> Settings {
        self.lock().settings
    }

    /// What `language` would resolve to now, without changing anything: the
    /// «Sistema · ahora: …» label of Ajustes → General (03 §16.1, spec 034).
    #[must_use]
    pub fn preview(&self, language: LanguagePreference) -> ResolvedLocale {
        self.resolver.resolve(language)
    }

    /// The locale the language in force resolved to.
    #[must_use]
    pub fn locale(&self) -> ResolvedLocale {
        self.lock().resolved.clone()
    }

    /// Persist `settings` and put them in force. `LocaleChanged` is published
    /// only when the language differs from the one in force (S9-07).
    ///
    /// # Errors
    /// [`EngineError::Settings`] when the save fails; nothing changes then.
    pub fn set(&self, settings: Settings) -> Result<(), EngineError> {
        let mut state = self.lock();
        self.repository.save(&settings)?;
        let language_changed = state.settings.language != settings.language;
        state.settings = settings;
        if language_changed {
            self.announce(&mut state);
        }
        Ok(())
    }

    /// Persist `language`, resolve it and publish `LocaleChanged`, even when
    /// it equals the one in force: the user picked it (RF-66).
    ///
    /// # Errors
    /// [`EngineError::Settings`] when the save fails; nothing changes then.
    pub fn set_language(
        &self,
        language: LanguagePreference,
    ) -> Result<ResolvedLocale, EngineError> {
        let mut state = self.lock();
        let next = state.settings.with_language(language);
        self.repository.save(&next)?;
        state.settings = next;
        Ok(self.announce(&mut state))
    }

    /// Resolve the language in force, keep the result and publish it. The
    /// caller holds the lock, so the events reach the bus in call order.
    fn announce(&self, state: &mut State) -> ResolvedLocale {
        let preference = state.settings.language;
        let resolved = self.resolver.resolve(preference);
        state.resolved = resolved.clone();
        self.bus.publish(EngineEvent::LocaleChanged {
            preference,
            resolved: resolved.clone(),
        });
        resolved
    }

    /// The state, ignoring lock poisoning as `session.rs` does: a panic in
    /// another thread must not lose the settings in force.
    fn lock(&self) -> MutexGuard<'_, State> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gafira_ports::i18n::{LanguagePreference, Locale, LocaleSource};
    use gafira_ports::settings::{Settings, SettingsError};

    use crate::engine::EngineError;
    use crate::event::EngineEvent;
    use crate::fakes::{MemorySettings, append_page, drain, engine_with_settings};

    fn locale_events(events: &[EngineEvent]) -> Vec<(LanguagePreference, Locale)> {
        events
            .iter()
            .filter_map(|event| match event {
                EngineEvent::LocaleChanged {
                    preference,
                    resolved,
                } => Some((*preference, resolved.locale)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn s9_03_start_loads_resolves_and_publishes_once() {
        let store = Arc::new(MemorySettings::holding(
            Settings::default().with_language(LanguagePreference::Es),
        ));
        let engine = engine_with_settings(&store);

        let resolved = engine.settings().start().expect("the settings load");

        assert_eq!(resolved.locale, Locale::Es);
        assert_eq!(resolved.source, LocaleSource::Manual);
        assert_eq!(engine.settings().get().language, LanguagePreference::Es);
        assert_eq!(engine.settings().locale(), resolved);
        let events = drain(&engine);
        assert_eq!(
            events.len(),
            1,
            "start publishes one event and nothing else"
        );
        assert_eq!(
            locale_events(&events),
            [(LanguagePreference::Es, Locale::Es)]
        );
    }

    #[test]
    fn s9_04_a_store_that_cannot_be_read_leaves_the_defaults_announced() {
        let store = Arc::new(MemorySettings::holding(
            Settings::default().with_language(LanguagePreference::Es),
        ));
        store.corrupt();
        let engine = engine_with_settings(&store);

        let error = engine.settings().start().expect_err("the store is corrupt");

        assert!(matches!(
            error,
            EngineError::Settings(SettingsError::Corrupt { .. })
        ));
        assert_eq!(engine.settings().get(), Settings::default());
        assert_eq!(engine.settings().locale().source, LocaleSource::Fallback);
        assert_eq!(
            locale_events(&drain(&engine)),
            [(LanguagePreference::System, Locale::En)]
        );
    }

    #[test]
    fn s9_05_set_language_persists_then_publishes() {
        let store = Arc::new(MemorySettings::default());
        let engine = engine_with_settings(&store);
        engine.settings().start().expect("the settings load");
        drain(&engine);

        let resolved = engine
            .settings()
            .set_language(LanguagePreference::Es)
            .expect("the save succeeds");

        assert_eq!(resolved.locale, Locale::Es);
        assert_eq!(
            store.stored().map(|s| s.language),
            Some(LanguagePreference::Es)
        );
        assert_eq!(engine.settings().get().language, LanguagePreference::Es);
        assert_eq!(engine.settings().locale(), resolved);
        assert_eq!(
            locale_events(&drain(&engine)),
            [(LanguagePreference::Es, Locale::Es)]
        );
    }

    #[test]
    fn s9_05_set_language_to_the_same_language_still_publishes() {
        let store = Arc::new(MemorySettings::default());
        let engine = engine_with_settings(&store);
        engine.settings().start().expect("the settings load");
        drain(&engine);

        engine
            .settings()
            .set_language(LanguagePreference::System)
            .expect("the save succeeds");

        assert_eq!(store.saves(), 1);
        assert_eq!(
            locale_events(&drain(&engine)),
            [(LanguagePreference::System, Locale::En)]
        );
    }

    #[test]
    fn s9_06_a_failed_save_changes_nothing() {
        let store = Arc::new(MemorySettings::default());
        let engine = engine_with_settings(&store);
        engine.settings().start().expect("the settings load");
        drain(&engine);
        let before = engine.settings().locale();
        store.fail_saves();

        let by_language = engine.settings().set_language(LanguagePreference::Es);
        let by_set = engine
            .settings()
            .set(Settings::default().with_language(LanguagePreference::En));

        assert!(matches!(
            by_language,
            Err(EngineError::Settings(SettingsError::Io(_)))
        ));
        assert!(matches!(
            by_set,
            Err(EngineError::Settings(SettingsError::Io(_)))
        ));
        assert_eq!(engine.settings().get(), Settings::default());
        assert_eq!(engine.settings().locale(), before);
        assert_eq!(store.stored(), None);
        assert!(drain(&engine).is_empty(), "nothing is published");
    }

    #[test]
    fn s9_07_set_publishes_only_when_the_language_changes() {
        let store = Arc::new(MemorySettings::default());
        let engine = engine_with_settings(&store);
        engine.settings().start().expect("the settings load");
        drain(&engine);

        engine
            .settings()
            .set(Settings::default())
            .expect("the save succeeds");
        assert_eq!(store.saves(), 1, "the same settings are still saved");
        assert!(drain(&engine).is_empty(), "same language, no event");

        engine
            .settings()
            .set(Settings::default().with_language(LanguagePreference::En))
            .expect("the save succeeds");
        assert_eq!(engine.settings().locale().source, LocaleSource::Manual);
        assert_eq!(
            locale_events(&drain(&engine)),
            [(LanguagePreference::En, Locale::En)]
        );
    }

    #[test]
    fn s9_08_a_language_change_keeps_the_session() {
        let store = Arc::new(MemorySettings::default());
        let mut engine = engine_with_settings(&store);
        append_page(&mut engine);
        let pages = engine.edit().document().expect("a session").pages().len();
        assert!(engine.projects().is_modified());
        drain(&engine);

        engine
            .settings()
            .set_language(LanguagePreference::Es)
            .expect("the save succeeds");

        assert!(engine.projects().is_modified(), "still modified");
        assert_eq!(
            engine.edit().document().expect("a session").pages().len(),
            pages
        );
        assert!(engine.edit().undo().is_ok(), "the history is intact");
        assert_eq!(
            engine.edit().document().expect("a session").pages().len(),
            pages - 1
        );
    }

    #[test]
    fn s34_02_preview_resolves_without_changing_or_publishing() {
        let store = Arc::new(MemorySettings::default());
        let engine = engine_with_settings(&store);
        engine.settings().start().expect("the settings load");
        drain(&engine);
        let before = engine.settings().locale();

        assert_eq!(
            engine.settings().preview(LanguagePreference::Es).locale,
            Locale::Es
        );
        assert_eq!(
            engine.settings().preview(LanguagePreference::System).locale,
            Locale::En
        );

        assert_eq!(engine.settings().locale(), before);
        assert_eq!(store.saves(), 0);
        assert!(drain(&engine).is_empty(), "a preview publishes nothing");
    }
}
