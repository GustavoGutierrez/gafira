//! `SettingsRepository` as a JSON file in the data folder (spec 009, 02 §14.4).

use std::fs;
use std::io;
use std::path::PathBuf;

use gafira_ports::settings::{Settings, SettingsError, SettingsRepository};

/// The settings in `<data_dir>/settings.json`, written atomically.
pub struct JsonSettings {
    path: PathBuf,
}

impl JsonSettings {
    #[must_use]
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }
}

impl SettingsRepository for JsonSettings {
    /// A missing file reads as the defaults; it is not an error (spec 009).
    fn load(&self) -> Result<Settings, SettingsError> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| SettingsError::Corrupt {
                detail: e.to_string(),
            }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(e) => Err(SettingsError::Io(e)),
        }
    }

    fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        let text = serde_json::to_vec_pretty(settings)
            .map_err(|e| SettingsError::Io(io::Error::other(e)))?;
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder).map_err(SettingsError::Io)?;
        }
        let mut tmp = self.path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        fs::write(&tmp, text).map_err(SettingsError::Io)?;
        fs::rename(&tmp, &self.path).map_err(SettingsError::Io)
    }
}

#[cfg(test)]
mod tests {
    use gafira_ports::i18n::LanguagePreference;

    use super::*;

    #[test]
    fn s18_05_missing_reads_default_saved_reads_back_and_garbage_is_corrupt() {
        let dir = std::env::temp_dir().join(format!("gafira-settings-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let repo = JsonSettings::at(dir.join("nested/settings.json"));
        assert_eq!(repo.load().expect("missing is fine"), Settings::default());

        let settings = Settings::default().with_language(LanguagePreference::Es);
        repo.save(&settings).expect("saves");
        assert_eq!(repo.load().expect("reads"), settings);

        fs::write(dir.join("nested/settings.json"), b"{nope").expect("garbage");
        assert!(matches!(repo.load(), Err(SettingsError::Corrupt { .. })));
        fs::remove_dir_all(dir).expect("clean up");
    }
}
