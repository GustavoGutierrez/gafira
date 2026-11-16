//! Spec 009: the persisted settings (S9-01) and their messages (S9-02).

use std::io;

use gafira_doc::msg::{MessageKey, ToMessage};
use gafira_ports::i18n::LanguagePreference;
use gafira_ports::settings::{Settings, SettingsError};

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
        assert_eq!(serde_json::to_string(&settings).expect("writes"), json);
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
