//! Remote photo catalogues (spec 071, D-111): the port the Fotos panel
//! searches through. The adapter (`gafira-photos`) talks to the network;
//! nothing here does.

use std::fmt;

/// A catalogue of photos the panel can search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhotoSource {
    /// Openverse (WordPress): Creative Commons and public domain images.
    Openverse,
    /// Wikimedia Commons.
    Wikimedia,
}

impl PhotoSource {
    /// Its name, as the panel shows it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Openverse => "Openverse",
            Self::Wikimedia => "Wikimedia Commons",
        }
    }
}

/// The shape of a photo, for the orientation filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhotoOrientation {
    Landscape,
    Portrait,
    Square,
}

/// What a source can filter by (spec 071 §4.3): the panel hides the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhotoCapabilities {
    /// The orientations it filters by; empty when it has no such filter.
    pub orientations: Vec<PhotoOrientation>,
}

/// One search: a source, its words, an optional orientation and a page
/// counted from 1.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhotoQuery {
    pub source: PhotoSource,
    pub text: String,
    pub orientation: Option<PhotoOrientation>,
    /// Which licences to show (owner request: free of rights by default).
    pub rights: PhotoRights,
    /// Cut-out illustrations on a transparent ground instead of photos
    /// (Ilustraciones detalladas, spec 072): rawpixel's public-domain
    /// stickers, in Openverse.
    pub cutouts: bool,
    pub page: u32,
}

/// One element of a licence, each with its own icon and explanation in the
/// panel (owner request).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LicenseTerm {
    /// CC0: no rights reserved.
    Zero,
    /// In the public domain.
    PublicDomain,
    /// BY: credit the creator.
    By,
    /// NC: non-commercial use only.
    NonCommercial,
    /// SA: adaptations under the same terms.
    ShareAlike,
    /// ND: no adaptations.
    NoDerivatives,
}

/// The licence of a photo: its short name, where it is written and its
/// elements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhotoLicense {
    pub name: String,
    pub url: String,
    pub terms: Vec<LicenseTerm>,
}

impl PhotoLicense {
    /// A licence from its short name (`CC BY-SA 3.0`, `CC0`, `Public
    /// domain`…): the elements are read from it; a name with none of them
    /// (GFDL, say) has no terms and is not marked as free to use.
    #[must_use]
    pub fn named(name: &str, url: &str) -> Self {
        let upper = name.to_uppercase();
        let words: Vec<&str> = upper
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        let has = |w: &str| words.contains(&w);
        let mut terms = Vec::new();
        if has("CC0") {
            terms.push(LicenseTerm::Zero);
        } else if upper.contains("PUBLIC DOMAIN") || has("PD") || has("PDM") {
            terms.push(LicenseTerm::PublicDomain);
        } else if has("BY") {
            terms.push(LicenseTerm::By);
            if has("NC") {
                terms.push(LicenseTerm::NonCommercial);
            }
            if has("SA") {
                terms.push(LicenseTerm::ShareAlike);
            }
            if has("ND") {
                terms.push(LicenseTerm::NoDerivatives);
            }
        }
        Self {
            name: name.trim().to_owned(),
            url: url.trim().to_owned(),
            terms,
        }
    }

    /// Whether it lets the photo be used commercially and adapted: a known
    /// licence without NC or ND (owner request: the panel marks these).
    #[must_use]
    pub fn free_to_use(&self) -> bool {
        !self.terms.is_empty()
            && !self.terms.contains(&LicenseTerm::NonCommercial)
            && !self.terms.contains(&LicenseTerm::NoDerivatives)
    }

    /// Whether it is free of rights: CC0 or the public domain, for any use
    /// and without credit (owner request).
    #[must_use]
    pub fn free_of_rights(&self) -> bool {
        self.terms
            .iter()
            .any(|t| matches!(t, LicenseTerm::Zero | LicenseTerm::PublicDomain))
    }

    /// Whether the creator must be credited.
    #[must_use]
    pub fn needs_credit(&self) -> bool {
        self.terms.contains(&LicenseTerm::By)
    }
}

/// A photo of a catalogue, with who made it and under which licence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Photo {
    pub source: PhotoSource,
    /// Its id in the source.
    pub id: String,
    pub title: String,
    pub author: String,
    /// The photo's own page at the source.
    pub page_url: String,
    pub license: PhotoLicense,
    /// A small image to browse with.
    pub thumbnail_url: String,
    /// The image to insert: the original, or a large version of it.
    pub image_url: String,
    /// Its size in px, 0 when the source does not say.
    pub width: u32,
    pub height: u32,
}

impl Photo {
    /// The credit kept with the photo once inserted: title, author, licence
    /// and source (spec 071 §6.4).
    #[must_use]
    pub fn credit(&self) -> String {
        let who = match (self.title.trim(), self.author.trim()) {
            ("", author) => author.to_owned(),
            (title, "") => title.to_owned(),
            (title, author) => format!("{title} — {author}"),
        };
        format!("{who} ({}, {})", self.license.name, self.source.name())
    }
}

/// Which licences a search shows (owner request).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PhotoRights {
    /// Free of rights: CC0 and the public domain, no credit needed, any use.
    #[default]
    Free,
    /// Commercial use and adaptation allowed, perhaps crediting the author
    /// (CC BY, CC BY-SA too).
    Commercial,
    /// Every licence, each with its marks.
    Any,
}

impl PhotoRights {
    /// Whether `license` fits.
    #[must_use]
    pub fn allows(self, license: &PhotoLicense) -> bool {
        match self {
            Self::Free => license.free_of_rights(),
            Self::Commercial => license.free_to_use(),
            Self::Any => true,
        }
    }
}

impl PhotoQuery {
    /// The first page of `text` in `source`.
    #[must_use]
    pub fn first(source: PhotoSource, text: &str) -> Self {
        Self {
            source,
            text: text.to_owned(),
            orientation: None,
            rights: PhotoRights::default(),
            cutouts: false,
            page: 1,
        }
    }
}

/// A page of results and the next page, if there is one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PhotoPage {
    pub items: Vec<Photo>,
    pub next_page: Option<u32>,
}

/// Why a catalogue gave nothing (spec 071 §9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhotoError {
    /// No connection.
    Offline,
    /// The source asked to wait; seconds, when it said.
    RateLimited { retry_after_s: Option<u64> },
    /// The photo or the page is gone.
    Unavailable,
    /// The source does not answer now (a 5xx): another source may.
    Busy,
    /// More bytes than allowed.
    TooLarge,
    /// An answer that could not be read.
    Invalid(String),
}

impl fmt::Display for PhotoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline => f.write_str("no connection"),
            Self::RateLimited { retry_after_s } => {
                write!(f, "rate limited (retry after {retry_after_s:?} s)")
            }
            Self::Unavailable => f.write_str("not available"),
            Self::Busy => f.write_str("the source does not answer now"),
            Self::TooLarge => f.write_str("too large"),
            Self::Invalid(why) => write!(f, "invalid answer: {why}"),
        }
    }
}

impl std::error::Error for PhotoError {}

/// The catalogues the Fotos panel searches (spec 071). Every call blocks:
/// the panel calls it off the interface thread.
pub trait PhotoCatalog: Send + Sync {
    /// The sources it has, in the order the panel offers them, with what
    /// each one filters by.
    fn sources(&self) -> Vec<(PhotoSource, PhotoCapabilities)>;
    /// A page of results; empty words give the source's own selection.
    fn search(&self, query: &PhotoQuery) -> Result<PhotoPage, PhotoError>;
    /// The bytes of the small image of `photo`.
    fn thumbnail(&self, photo: &Photo) -> Result<Vec<u8>, PhotoError>;
    /// The bytes of the image to insert.
    fn image(&self, photo: &Photo) -> Result<Vec<u8>, PhotoError>;
}

/// A collection of vector illustrations (Elementos → Ilustraciones): its
/// id, name, author and licence, all of them free to use and adapt, also
/// commercially, without credit in the design.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IllustrationSet {
    pub id: String,
    pub name: String,
    pub author: String,
    pub license: PhotoLicense,
    /// In colour, or in black and white (line drawings).
    pub color: bool,
}

/// Which illustrations a search shows: all, in colour, or in black and
/// white.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IllustrationStyle {
    #[default]
    Any,
    Color,
    Mono,
}

impl IllustrationStyle {
    /// Whether a collection in colour (`color`) fits.
    #[must_use]
    pub fn allows(self, color: bool) -> bool {
        match self {
            Self::Any => true,
            Self::Color => color,
            Self::Mono => !color,
        }
    }
}

/// One illustration: its collection and its name in it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Illustration {
    pub set: String,
    pub name: String,
}

/// The vector illustrations the Ilustraciones panel searches. Every call
/// blocks: the panel calls it off the interface thread.
pub trait IllustrationCatalog: Send + Sync {
    /// The collections it searches, each free to use.
    fn sets(&self) -> Vec<IllustrationSet>;
    /// Illustrations whose names match `text`, in every collection of
    /// `style`, at most `limit`.
    fn find(
        &self,
        text: &str,
        style: IllustrationStyle,
        limit: u32,
    ) -> Result<Vec<Illustration>, PhotoError>;
    /// The first `limit` illustrations of collection `set`.
    fn browse(&self, set: &str, limit: u32) -> Result<Vec<Illustration>, PhotoError>;
    /// The SVG of `illustration`.
    fn svg(&self, illustration: &Illustration) -> Result<Vec<u8>, PhotoError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Owner request: the elements of a licence come from its name; only
    // those without NC or ND are free to use, CC0 and public domain too.
    #[test]
    fn licence_terms_from_their_names() {
        use LicenseTerm::*;
        let terms = |name: &str| PhotoLicense::named(name, "").terms;
        assert_eq!(terms("CC BY-NC-SA 2.0"), [By, NonCommercial, ShareAlike]);
        assert_eq!(terms("CC BY-SA 3.0"), [By, ShareAlike]);
        assert_eq!(terms("CC BY-ND 4.0"), [By, NoDerivatives]);
        assert_eq!(terms("CC0"), [Zero]);
        assert_eq!(terms("Public domain"), [PublicDomain]);
        assert_eq!(terms("GFDL"), []);
        let free = |name: &str| PhotoLicense::named(name, "").free_to_use();
        assert!(free("CC BY 2.0") && free("CC BY-SA 3.0") && free("CC0 1.0"));
        assert!(free("Public Domain Mark 1.0"));
        assert!(!free("CC BY-NC 2.0") && !free("CC BY-ND 2.0") && !free("GFDL"));
        assert!(PhotoLicense::named("CC BY 2.0", "").needs_credit());
        assert!(!PhotoLicense::named("CC0", "").needs_credit());
        let rights = |r: PhotoRights, name: &str| r.allows(&PhotoLicense::named(name, ""));
        assert!(rights(PhotoRights::Free, "CC0 1.0") && rights(PhotoRights::Free, "Public domain"));
        assert!(!rights(PhotoRights::Free, "CC BY 2.0"));
        assert!(rights(PhotoRights::Commercial, "CC BY 2.0"));
        assert!(!rights(PhotoRights::Commercial, "CC BY-NC 2.0"));
        assert!(rights(PhotoRights::Any, "CC BY-NC-ND 2.0"));
    }
}
