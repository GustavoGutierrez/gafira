//! Spec 009: the persisted settings (S9-01) and their messages (S9-02).

use std::io;

use gafira_doc::msg::{MessageKey, ToMessage};
use gafira_ports::i18n::LanguagePreference;
use gafira_ports::settings::{Appearance, Settings, SettingsError};

fn read(json: &str) -> Result<Settings, serde_json::Error> {
    serde_json::from_str(json)
}

#[test]
fn s9_01_known_languages_round_trip() {
    for (text, pref) in [
        ("system", LanguagePreference::System),
        ("es", LanguagePreference::Es),
        ("en", LanguagePreference::En),
    ] {
        let json = format!(r#"{{"language":"{text}"}}"#);
        let settings = read(&json).expect("a known language reads");
        assert_eq!(settings.language, pref, "{text}");
        let written = format!(r#"{{"language":"{text}","appearance":"light"}}"#);
        assert_eq!(serde_json::to_string(&settings).expect("writes"), written);
    }
}

#[test]
fn s9_01_unknown_text_reads_as_system() {
    for text in ["ES", "fr", ""] {
        let json = format!(r#"{{"language":"{text}"}}"#);
        assert_eq!(
            read(&json).expect("unknown text is not an error").language,
            LanguagePreference::System,
            "{text:?}"
        );
    }
}

#[test]
fn s9_01_non_text_is_a_read_error() {
    for value in ["1", "null", "{}", "true"] {
        let json = format!(r#"{{"language":{value}}}"#);
        assert!(read(&json).is_err(), "{value}");
    }
}

#[test]
fn s9_01_missing_key_and_default_are_system() {
    assert_eq!(
        read("{}").expect("reads").language,
        LanguagePreference::System
    );
    assert_eq!(Settings::default().language, LanguagePreference::System);
}

#[test]
fn s9_01_with_language_changes_only_the_language() {
    let settings = Settings::default().with_language(LanguagePreference::Es);
    assert_eq!(settings.language, LanguagePreference::Es);
}

#[test]
fn s9_02_every_error_has_a_settings_key() {
    let corrupt = SettingsError::Corrupt {
        detail: "bad json".to_owned(),
    };
    let io = SettingsError::Io(io::Error::other("disk"));
    assert_eq!(
        corrupt.to_message().key,
        MessageKey("error.settings.corrupt")
    );
    assert_eq!(io.to_message().key, MessageKey("error.settings.io"));
    assert!(corrupt.to_string().contains("bad json"));
}

#[test]
fn s21_01_appearance_round_trips_and_defaults_to_light() {
    assert_eq!(Settings::default().appearance, Appearance::Light);
    let dark = Settings::default().with_appearance(Appearance::Dark);
    let json = serde_json::to_string(&dark).expect("writes");
    assert_eq!(json, r#"{"language":"system","appearance":"dark"}"#);
    assert_eq!(read(&json).expect("reads"), dark);
    for text in ["light", "sepia", ""] {
        let json = format!(r#"{{"appearance":"{text}"}}"#);
        assert_eq!(
            read(&json).expect("reads").appearance,
            Appearance::Light,
            "{text:?}"
        );
    }
    assert_eq!(
        read(r#"{"language":"es"}"#).expect("reads").appearance,
        Appearance::Light
    );
}

#[test]
fn s34_01_appearance_can_follow_the_system() {
    let system = Settings::default().with_appearance(Appearance::System);
    let json = serde_json::to_string(&system).expect("writes");
    assert!(json.contains(r#""appearance":"system""#), "{json}");
    assert_eq!(read(&json).expect("reads"), system);
}

// S44-12: rulers, guides and magnet persist, are on by default and leave
// the file as before while they are (spec 044).
#[test]
fn s44_12_view_switches_round_trip_and_default_to_on() {
    use gafira_ports::settings::ViewSettings;
    let view = Settings::default().view;
    assert!(view.rulers && view.guides && view.magnet);
    assert_eq!(read(r#"{"language":"es"}"#).expect("reads").view, view);
    let off = Settings::default().with_view(ViewSettings::new(true, false, false));
    let json = serde_json::to_string(&off).expect("writes");
    assert_eq!(
        json,
        r#"{"language":"system","appearance":"light","view":{"rulers":true,"guides":false,"magnet":false}}"#
    );
    assert_eq!(read(&json).expect("reads"), off);
    let partial = read(r#"{"view":{"magnet":false}}"#).expect("reads").view;
    assert!(
        partial.rulers && partial.guides && !partial.magnet,
        "a missing switch reads as on"
    );
}
