//! Gafira interface language (02 §2.1, §14; D-08, D-64).
//!
//! A layer-4 adapter that implements the language ports of
//! `gafira_ports::i18n`:
//!
//! - [`D08LocaleResolver`] (`LocaleResolver`): the D-08 rule on the operating
//!   system's tags and the D-64 format region.
//! - [`CatalogTranslator`] (`Translator`): messages by key with parameters and
//!   CLDR plurals from the Spanish and English catalogs, compiled in by the
//!   build script from `assets/i18n/*.toml`; numbers and quantities with ICU4X;
//!   the fallback chain of 02 §14.6.
//! - `PseudoTranslator` (feature `pseudo`): pseudo-localization for tests.
//!
//! No global state, no disk access at run time, no egui, `gafira-engine`,
//! `gafira-ui` or `gafira-platform` (ARQ-107).
#![forbid(unsafe_code)]

mod catalog;
mod format;
#[cfg(any(test, feature = "pseudo"))]
mod pseudo;
mod resolver;
mod translator;

#[cfg(any(test, feature = "pseudo"))]
pub use pseudo::PseudoTranslator;
pub use resolver::D08LocaleResolver;
pub use translator::CatalogTranslator;
