//! Asset packages (`.gafpack`, spec 097): the manifest, the rules a
//! package follows before it is installed, and the port the app installs
//! them through. Pure, so it is tested without a window or a disk.
//!
//! A package is a ZIP with `manifest.json` and `thumb.webp` at its root.
//! The manifest says what each asset is and where Gafira shows it; nothing
//! in a package runs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The extension of a package file.
pub const PACK_EXTENSION: &str = "gafpack";

/// The format of the manifest this version reads.
pub const PACK_FORMAT: &str = "gafpack";

/// The version of the manifest this version reads.
pub const PACK_FORMAT_VERSION: &str = "1.1";

/// The package thumbnail, at the root of the ZIP.
pub const PACK_THUMBNAIL: &str = "thumb.webp";

/// The side of a package thumbnail, in px: square, within these.
pub const THUMB_SIDES: std::ops::RangeInclusive<u32> = 256..=2048;

/// The largest package thumbnail, in bytes.
pub const THUMB_MOST_BYTES: usize = 2 * 1024 * 1024;

/// The largest side of an image asset, in px.
pub const ASSET_MOST_SIDE: u32 = 8192;

/// A text in Spanish and English (§4.1.2).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Localized {
    #[serde(default)]
    pub es: String,
    #[serde(default)]
    pub en: String,
}

impl Localized {
    /// The text in `locale`'s language.
    #[must_use]
    pub fn get(&self, spanish: bool) -> &str {
        if spanish { &self.es } else { &self.en }
    }
}

/// Who made the package, as declared; not a verified identity.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackAuthor {
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// One asset of the manifest (§4.2).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackAsset {
    #[serde(default)]
    pub id: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// SHA-256 of the file, lowercase hexadecimal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl PackAsset {
    /// Where Gafira shows it: its placement, or the default of its type.
    #[must_use]
    pub fn section(&self) -> Option<PackSection> {
        let kind = AssetKind::parse(&self.kind)?;
        match &self.placement {
            None => Some(kind.default_section()),
            Some(p) => PackSection::parse(p).filter(|s| kind.allows(*s)),
        }
    }
}

/// The manifest of a package (§4.1).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManifest {
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub format_version: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub package_type: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub name: Localized,
    #[serde(default)]
    pub description: Localized,
    #[serde(default)]
    pub author: PackAuthor,
    #[serde(default)]
    pub thumbnail: String,
    #[serde(default)]
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_file: Option<String>,
    #[serde(default)]
    pub min_gafira_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_gafira_version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub assets: Vec<PackAsset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
}

/// The kind of a whole package (§4.1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackType {
    Clipart,
    Illustrations,
    Icons,
    Shapes,
    Frames,
    Powerclips,
    Collages,
    Templates,
    Mixed,
}

impl PackType {
    pub const ALL: [Self; 9] = [
        Self::Clipart,
        Self::Illustrations,
        Self::Icons,
        Self::Shapes,
        Self::Frames,
        Self::Powerclips,
        Self::Collages,
        Self::Templates,
        Self::Mixed,
    ];

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.code() == text)
    }

    /// Its value in the manifest.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Clipart => "clipart",
            Self::Illustrations => "illustrations",
            Self::Icons => "icons",
            Self::Shapes => "shapes",
            Self::Frames => "frames",
            Self::Powerclips => "powerclips",
            Self::Collages => "collages",
            Self::Templates => "templates",
            Self::Mixed => "mixed",
        }
    }

    /// The catalog key of its name.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Clipart => "packs.type_clipart",
            Self::Illustrations => "packs.type_illustrations",
            Self::Icons => "packs.type_icons",
            Self::Shapes => "packs.type_shapes",
            Self::Frames => "packs.type_frames",
            Self::Powerclips => "packs.type_powerclips",
            Self::Collages => "packs.type_collages",
            Self::Templates => "packs.type_templates",
            Self::Mixed => "packs.type_mixed",
        }
    }

    /// The section its assets are expected in; none for a mixed package.
    #[must_use]
    pub fn section(self) -> Option<PackSection> {
        Some(match self {
            Self::Clipart | Self::Illustrations => PackSection::Illustrations,
            Self::Icons => PackSection::Icons,
            Self::Shapes => PackSection::Shapes,
            Self::Frames => PackSection::Frames,
            Self::Powerclips => PackSection::Powerclips,
            Self::Collages => PackSection::Collages,
            Self::Templates => PackSection::Templates,
            Self::Mixed => return None,
        })
    }
}

/// Where an asset is shown (§5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackSection {
    Illustrations,
    Icons,
    Shapes,
    Frames,
    Powerclips,
    Collages,
    Templates,
}

impl PackSection {
    pub const ALL: [Self; 7] = [
        Self::Illustrations,
        Self::Icons,
        Self::Shapes,
        Self::Frames,
        Self::Powerclips,
        Self::Collages,
        Self::Templates,
    ];

    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.code() == text)
    }

    /// Its value in the manifest.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Illustrations => "illustrations",
            Self::Icons => "icons",
            Self::Shapes => "shapes",
            Self::Frames => "frames",
            Self::Powerclips => "powerclips",
            Self::Collages => "collages",
            Self::Templates => "templates",
        }
    }

    /// The catalog key of its name.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Illustrations => "packs.section_illustrations",
            Self::Icons => "packs.section_icons",
            Self::Shapes => "packs.section_shapes",
            Self::Frames => "packs.section_frames",
            Self::Powerclips => "packs.section_powerclips",
            Self::Collages => "packs.section_collages",
            Self::Templates => "packs.section_templates",
        }
    }

    /// The catalog key of its name for one asset.
    #[must_use]
    pub fn key_one(self) -> &'static str {
        match self {
            Self::Illustrations => "packs.section_illustrations_one",
            Self::Icons => "packs.section_icons_one",
            Self::Shapes => "packs.section_shapes_one",
            Self::Frames => "packs.section_frames_one",
            Self::Powerclips => "packs.section_powerclips_one",
            Self::Collages => "packs.section_collages_one",
            Self::Templates => "packs.section_templates_one",
        }
    }

    /// Whether this version of Gafira can show and insert its assets
    /// (phase 1 of spec 097).
    #[must_use]
    pub fn supported(self) -> bool {
        // Phase 2 of spec 097: every section takes its assets.
        true
    }
}

/// The kind of one asset (§5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Clipart,
    Illustration,
    Icon,
    Shape,
    Frame,
    Powerclip,
    Collage,
    Template,
}

impl AssetKind {
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "clipart" => Self::Clipart,
            "illustration" => Self::Illustration,
            "icon" => Self::Icon,
            "shape" => Self::Shape,
            "frame" => Self::Frame,
            "powerclip" => Self::Powerclip,
            "collage" => Self::Collage,
            "template" => Self::Template,
            _ => return None,
        })
    }

    /// Where it is shown when the manifest does not say.
    #[must_use]
    pub fn default_section(self) -> PackSection {
        match self {
            Self::Clipart | Self::Illustration => PackSection::Illustrations,
            Self::Icon => PackSection::Icons,
            Self::Shape => PackSection::Shapes,
            Self::Frame => PackSection::Frames,
            Self::Powerclip => PackSection::Powerclips,
            Self::Collage => PackSection::Collages,
            Self::Template => PackSection::Templates,
        }
    }

    /// Whether it may be placed in `section` (§5.1).
    #[must_use]
    pub fn allows(self, section: PackSection) -> bool {
        section == self.default_section()
            || matches!(
                (self, section),
                (Self::Clipart | Self::Illustration, PackSection::Icons)
                    | (Self::Icon, PackSection::Illustrations)
                    | (Self::Shape, PackSection::Illustrations)
            )
    }
}

/// What is wrong with a package: a catalog key and what it is about (a
/// field, an asset id or a path).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackProblem {
    pub key: &'static str,
    pub about: String,
}

impl PackProblem {
    #[must_use]
    pub fn new(key: &'static str, about: impl Into<String>) -> Self {
        Self {
            key,
            about: about.into(),
        }
    }
}

/// A version `major.minor.patch` with an optional pre-release, ordered as
/// SemVer orders them (a pre-release before its release).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    core: (u64, u64, u64),
    /// `None` (a release) sorts after any pre-release.
    release: ReleaseOrder,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum ReleaseOrder {
    Pre(Vec<PreId>),
    Release,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PreId {
    Number(u64),
    Text(String),
}

impl Version {
    /// `text` as a version; `None` when it is not `x.y.z[-pre][+build]`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut parts = core.split('.');
        let mut number = || -> Option<u64> {
            let part = parts.next()?;
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        };
        let core = (number()?, number()?, number()?);
        if parts.next().is_some() {
            return None;
        }
        let release = match pre {
            None => ReleaseOrder::Release,
            Some(pre) => {
                let ids: Option<Vec<PreId>> = pre
                    .split('.')
                    .map(|id| {
                        if id.is_empty() {
                            None
                        } else if id.bytes().all(|b| b.is_ascii_digit()) {
                            id.parse().ok().map(PreId::Number)
                        } else {
                            Some(PreId::Text(id.to_owned()))
                        }
                    })
                    .collect();
                ReleaseOrder::Pre(ids?)
            }
        };
        Some(Self { core, release })
    }
}

/// Whether `id` is a package identifier: lowercase letters, digits, `-`
/// and `_` in at least two parts joined by dots (`com.studio.pack`).
#[must_use]
pub fn valid_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    id.len() <= 128
        && parts.len() >= 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
        })
}

/// Whether `path` is a safe entry of a package: relative, with `/` only,
/// no `..`, `.`, empty parts, drive letters or control characters, and
/// not too deep.
#[must_use]
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 512
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && !path.chars().any(char::is_control)
        && path.split('/').count() <= 8
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// The problems of `manifest` for a Gafira of `app` version; empty when it
/// is valid. `present` is every file in the package (§4, §5, §9.1).
#[must_use]
pub fn manifest_problems(
    manifest: &PackManifest,
    present: &BTreeSet<String>,
    app: &Version,
) -> Vec<PackProblem> {
    let mut out = Vec::new();
    let mut problem = |key: &'static str, about: &str| out.push(PackProblem::new(key, about));
    if manifest.format != PACK_FORMAT {
        problem("packs.bad_format", &manifest.format);
    }
    if manifest.format_version != PACK_FORMAT_VERSION {
        problem("packs.bad_format_version", &manifest.format_version);
    }
    if !valid_id(&manifest.id) {
        problem("packs.bad_id", &manifest.id);
    }
    if Version::parse(&manifest.version).is_none() {
        problem("packs.bad_version", &manifest.version);
    }
    let pack_type = PackType::parse(&manifest.package_type);
    if pack_type.is_none() {
        problem("packs.bad_type", &manifest.package_type);
    }
    for (field, text) in [
        ("name.es", &manifest.name.es),
        ("name.en", &manifest.name.en),
    ] {
        let n = text.trim().chars().count();
        if !(1..=100).contains(&n) {
            problem("packs.bad_name", field);
        }
    }
    for (field, text) in [
        ("description.es", &manifest.description.es),
        ("description.en", &manifest.description.en),
    ] {
        let n = text.trim().chars().count();
        if !(10..=1000).contains(&n) {
            problem("packs.bad_description", field);
        }
    }
    if manifest.author.name.trim().is_empty() {
        problem("packs.no_author", "author.name");
    }
    if manifest.thumbnail != PACK_THUMBNAIL {
        problem("packs.bad_thumbnail", &manifest.thumbnail);
    } else if !present.contains(PACK_THUMBNAIL) {
        problem("packs.no_thumbnail", PACK_THUMBNAIL);
    }
    if manifest.license.trim().is_empty() {
        problem("packs.no_license", "license");
    } else if manifest.license.starts_with("LicenseRef-") {
        match &manifest.license_file {
            Some(file) if safe_path(file) && present.contains(file) => {}
            Some(file) => problem("packs.missing_file", file),
            None => problem("packs.no_license_file", "licenseFile"),
        }
    }
    match Version::parse(&manifest.min_gafira_version) {
        None => problem("packs.bad_min_version", &manifest.min_gafira_version),
        Some(least) if &least > app => problem("packs.needs_newer", &manifest.min_gafira_version),
        Some(_) => {}
    }
    if let Some(most) = &manifest.max_gafira_version {
        match Version::parse(most) {
            None => problem("packs.bad_max_version", most),
            Some(most_v) if &most_v < app => problem("packs.too_new_app", most),
            Some(_) => {}
        }
    }
    if manifest.assets.is_empty() {
        problem("packs.no_assets", "assets");
    }
    let mut ids = BTreeSet::new();
    let mut sections = BTreeSet::new();
    for asset in &manifest.assets {
        let about = if asset.id.is_empty() {
            "?"
        } else {
            asset.id.as_str()
        };
        if asset.id.trim().is_empty() || asset.id.len() > 128 {
            problem("packs.bad_asset_id", about);
        } else if !ids.insert(asset.id.as_str()) {
            problem("packs.duplicate_asset", about);
        }
        if asset.name.trim().is_empty() {
            problem("packs.no_asset_name", about);
        }
        let Some(kind) = AssetKind::parse(&asset.kind) else {
            problem("packs.bad_asset_type", about);
            continue;
        };
        let Some(section) = asset.section() else {
            problem("packs.bad_placement", about);
            continue;
        };
        sections.insert(section);
        // What each type is made of (§5, §6): an image or SVG; a frame one
        // of those or a descriptor; a powerclip or collage a descriptor; a
        // template a `.gaf`.
        let needs_definition = matches!(kind, AssetKind::Powerclip | AssetKind::Collage)
            || (kind == AssetKind::Frame && asset.definition.is_some());
        if needs_definition {
            match &asset.definition {
                None => problem("packs.no_definition", about),
                Some(def) if !safe_path(def) => problem("packs.unsafe_path", def),
                Some(def) if !present.contains(def) => problem("packs.missing_file", def),
                Some(def) if !def.to_ascii_lowercase().ends_with(".json") => {
                    problem("packs.bad_asset_format", def);
                }
                Some(_) => {}
            }
        } else {
            let template = kind == AssetKind::Template;
            match &asset.file {
                None => problem("packs.no_asset_file", about),
                Some(file) if !safe_path(file) => problem("packs.unsafe_path", file),
                Some(file) if !present.contains(file) => problem("packs.missing_file", file),
                Some(file) if template && !file.to_ascii_lowercase().ends_with(".gaf") => {
                    problem("packs.bad_asset_format", file);
                }
                Some(file) if !template && file_kind(file).is_none() => {
                    problem("packs.bad_asset_format", file);
                }
                Some(_) => {}
            }
        }
        if let Some(preview) = &asset.preview {
            if !safe_path(preview) {
                problem("packs.unsafe_path", preview);
            } else if !present.contains(preview) {
                problem("packs.missing_file", preview);
            }
        }
        if let Some(hash) = &asset.hash
            && (hash.len() != 64 || !hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        {
            problem("packs.bad_hash", about);
        }
    }
    // The categories are the sections of the assets, no more and no less;
    // a specific type has its own among them.
    let declared: BTreeSet<Option<PackSection>> = manifest
        .categories
        .iter()
        .map(|c| PackSection::parse(c))
        .collect();
    let actual: BTreeSet<Option<PackSection>> = sections.iter().copied().map(Some).collect();
    if manifest.categories.is_empty() || declared != actual {
        problem("packs.bad_categories", &manifest.categories.join(", "));
    }
    if let Some(expected) = pack_type.and_then(PackType::section)
        && !sections.contains(&expected)
    {
        problem("packs.type_without_assets", &manifest.package_type);
    }
    out
}

/// How the content of a frame or powerclip fills it (§6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentFit {
    Cover,
    Contain,
}

/// What a descriptor of a composite asset says (§6), checked.
#[derive(Clone, Debug, PartialEq)]
pub enum Composite {
    /// A frame that takes a photo, cut by the outline of the SVG `mask` (a
    /// path in the package): a frame with a slot or a powerclip.
    Frame { mask: String, fit: ContentFit },
    /// A collage: its slots as `(x, y, w, h)` shares of the page and the
    /// room between them as a share of the page's shorter side.
    Collage {
        cells: Vec<(f64, f64, f64, f64)>,
        gap: f64,
    },
}

/// The most slots a collage may have.
pub const MOST_SLOTS: usize = 24;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    #[serde(default)]
    schema: String,
    #[serde(default)]
    version: u32,
    #[serde(default)]
    mask: Option<String>,
    #[serde(default)]
    content_fit: Option<String>,
    #[serde(default)]
    slots: Option<Vec<Slot>>,
    #[serde(default)]
    gap: Option<f64>,
}

#[derive(Deserialize)]
struct Slot {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// `relative`, a path written in the descriptor at `descriptor`, as a path
/// of the package: from the descriptor's folder, and only when it stays
/// safe (§6.2).
#[must_use]
pub fn resolve(descriptor: &str, relative: &str) -> Option<String> {
    if !safe_path(relative) {
        return None;
    }
    let joined = match descriptor.rsplit_once('/') {
        Some((folder, _)) => format!("{folder}/{relative}"),
        None => relative.to_owned(),
    };
    safe_path(&joined).then_some(joined)
}

/// The descriptor of a `kind` asset at `path`, read from `bytes` and
/// checked against the files `present` in the package; the catalog key of
/// what is wrong otherwise.
pub fn composite(
    kind: AssetKind,
    bytes: &[u8],
    path: &str,
    present: &BTreeSet<String>,
) -> Result<Composite, &'static str> {
    composite_with(kind, bytes, path, |p| present.contains(p))
}

/// [`composite`] with `exists` saying whether a file is in the package.
pub fn composite_with(
    kind: AssetKind,
    bytes: &[u8],
    path: &str,
    exists: impl Fn(&str) -> bool,
) -> Result<Composite, &'static str> {
    let bad = "packs.bad_descriptor";
    let d: Descriptor = serde_json::from_slice(bytes).map_err(|_| bad)?;
    if d.version != 1 {
        return Err(bad);
    }
    match (kind, d.schema.as_str()) {
        (AssetKind::Frame, "gafira.frame") | (AssetKind::Powerclip, "gafira.powerclip") => {
            let mask = d.mask.as_deref().ok_or(bad)?;
            let mask = resolve(path, mask).ok_or("packs.unsafe_path")?;
            if !exists(&mask) {
                return Err("packs.missing_file");
            }
            if file_kind(&mask) != Some(FileKind::Svg) {
                return Err(bad);
            }
            let fit = match d.content_fit.as_deref() {
                None | Some("cover") => ContentFit::Cover,
                Some("contain") => ContentFit::Contain,
                Some(_) => return Err(bad),
            };
            Ok(Composite::Frame { mask, fit })
        }
        (AssetKind::Collage, "gafira.collage") => {
            let slots = d.slots.ok_or(bad)?;
            if slots.is_empty() || slots.len() > MOST_SLOTS {
                return Err(bad);
            }
            let unit = |v: f64| (0.0..=1.0).contains(&v);
            let cells: Vec<(f64, f64, f64, f64)> =
                slots.iter().map(|s| (s.x, s.y, s.w, s.h)).collect();
            let fits = cells.iter().all(|&(x, y, w, h)| {
                unit(x)
                    && unit(y)
                    && w > 0.0
                    && h > 0.0
                    && x + w <= 1.0 + 1e-9
                    && y + h <= 1.0 + 1e-9
            });
            let gap = d.gap.unwrap_or(0.012);
            if !fits || !(0.0..=0.2).contains(&gap) {
                return Err(bad);
            }
            Ok(Composite::Collage { cells, gap })
        }
        _ => Err(bad),
    }
}

/// The kind of file an asset is, by its extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Png,
    Webp,
    Svg,
}

/// The kind of the file at `path`; `None` for any other format.
#[must_use]
pub fn file_kind(path: &str) -> Option<FileKind> {
    let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match ext.as_str() {
        "png" => FileKind::Png,
        "webp" => FileKind::Webp,
        "svg" => FileKind::Svg,
        _ => return None,
    })
}

/// The problem of an SVG file, if any (§9.2): it must parse, and it may
/// not hold scripts, entities, `foreignObject` or references outside
/// itself (a link that is not `#…` or a `data:` image).
#[must_use]
pub fn svg_problem(bytes: &[u8]) -> Option<&'static str> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Some("packs.svg_unreadable");
    };
    let lower = text.to_ascii_lowercase();
    if lower.contains("<!doctype") || lower.contains("<!entity") {
        return Some("packs.svg_active");
    }
    let Ok(doc) = roxmltree::Document::parse(text) else {
        return Some("packs.svg_unreadable");
    };
    if doc.root_element().tag_name().name() != "svg" {
        return Some("packs.svg_unreadable");
    }
    for node in doc.descendants().filter(roxmltree::Node::is_element) {
        let tag = node.tag_name().name().to_ascii_lowercase();
        if matches!(
            tag.as_str(),
            "script" | "foreignobject" | "iframe" | "embed" | "object" | "audio" | "video"
        ) {
            return Some("packs.svg_active");
        }
        for attr in node.attributes() {
            let name = attr.name().to_ascii_lowercase();
            let value = attr.value().trim().to_ascii_lowercase();
            if name.starts_with("on") {
                return Some("packs.svg_active");
            }
            if name == "href" && !value.starts_with('#') && !value.starts_with("data:image/") {
                return Some("packs.svg_external");
            }
            if value.contains("url(") && !external_free(&value) {
                return Some("packs.svg_external");
            }
        }
        if tag == "style"
            && let Some(css) = node.text()
            && (css.contains("@import") || !external_free(&css.to_ascii_lowercase()))
        {
            return Some("packs.svg_external");
        }
    }
    None
}

/// Whether every `url(…)` in `text` points inside the document (`#…`).
fn external_free(text: &str) -> bool {
    text.split("url(").skip(1).all(|rest| {
        let target = rest.trim_start_matches(['\'', '"', ' ']);
        target.starts_with('#')
    })
}

/// The size and whether a raster image has any see-through pixel; the
/// problem when it does not decode within the limits (§9.2).
pub fn raster_check(bytes: &[u8], kind: FileKind) -> Result<(u32, u32, bool), &'static str> {
    let format = match kind {
        FileKind::Png => image::ImageFormat::Png,
        FileKind::Webp => image::ImageFormat::WebP,
        FileKind::Svg => return Err("packs.bad_asset_format"),
    };
    let reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let (w, h) = reader
        .into_dimensions()
        .map_err(|_| "packs.image_unreadable")?;
    if w == 0 || h == 0 || w > ASSET_MOST_SIDE || h > ASSET_MOST_SIDE {
        return Err("packs.image_too_big");
    }
    let image =
        image::load_from_memory_with_format(bytes, format).map_err(|_| "packs.image_unreadable")?;
    // The alpha read where it is, without a copy of the pixels.
    let clear = match &image {
        image::DynamicImage::ImageRgba8(rgba) => rgba.pixels().any(|p| p.0[3] < 255),
        other if other.color().has_alpha() => other.to_rgba8().pixels().any(|p| p.0[3] < 255),
        _ => false,
    };
    Ok((w, h, clear))
}

/// The problem of the package thumbnail, if any (§4.1.3): a still WebP,
/// square, between 256 and 2048 px a side, at most 2 MiB.
#[must_use]
pub fn thumbnail_problem(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() > THUMB_MOST_BYTES {
        return Some("packs.thumb_too_big");
    }
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Some("packs.thumb_not_webp");
    }
    if webp_animated(bytes) {
        return Some("packs.thumb_animated");
    }
    let reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::WebP);
    let Ok((w, h)) = reader.into_dimensions() else {
        return Some("packs.thumb_not_webp");
    };
    if w != h || !THUMB_SIDES.contains(&w) {
        return Some("packs.thumb_bad_size");
    }
    if image::load_from_memory_with_format(bytes, image::ImageFormat::WebP).is_err() {
        return Some("packs.thumb_not_webp");
    }
    None
}

/// Whether a WebP file is animated: an extended header with the animation
/// flag, or an `ANIM` chunk.
fn webp_animated(bytes: &[u8]) -> bool {
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let tag = &bytes[at..at + 4];
        let size = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]])
            as usize;
        if tag == b"ANIM" || tag == b"ANMF" {
            return true;
        }
        if tag == b"VP8X" && bytes.get(at + 8).is_some_and(|flags| flags & 0x02 != 0) {
            return true;
        }
        at = at.saturating_add(8 + size + (size & 1));
    }
    false
}

/// SHA-256 of `bytes`, lowercase hexadecimal.
#[must_use]
pub fn sha256(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let digest = sha2::Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// The problems of the package thumbnail read by `read` (§4.1.3).
pub fn thumbnail_problems(mut read: impl FnMut(&str) -> Option<Vec<u8>>) -> Vec<PackProblem> {
    match read(PACK_THUMBNAIL) {
        None => vec![PackProblem::new("packs.no_thumbnail", PACK_THUMBNAIL)],
        Some(bytes) => thumbnail_problem(&bytes)
            .map(|key| PackProblem::new(key, PACK_THUMBNAIL))
            .into_iter()
            .collect(),
    }
}

/// The problems of the files of one asset (§9.2): its descriptor and the
/// mask it names, its file and its preview decode or parse safely, and a
/// declared hash matches. `read` gives the bytes of a file of the package,
/// one at a time, so a large package never sits in memory whole.
pub fn asset_problems(
    asset: &PackAsset,
    present: &BTreeSet<String>,
    mut read: impl FnMut(&str) -> Option<Vec<u8>>,
) -> Vec<PackProblem> {
    let mut out = Vec::new();
    // A descriptor, and the mask it names.
    if let (Some(def), Some(kind)) = (&asset.definition, AssetKind::parse(&asset.kind)) {
        let Some(bytes) = read(def) else {
            out.push(PackProblem::new("packs.missing_file", def.clone()));
            return out;
        };
        match composite(kind, &bytes, def, present) {
            Err(key) => out.push(PackProblem::new(key, def.clone())),
            Ok(Composite::Frame { mask, .. }) => match read(&mask).as_deref().map(svg_problem) {
                None => out.push(PackProblem::new("packs.missing_file", mask)),
                Some(Some(key)) => out.push(PackProblem::new(key, mask)),
                Some(None) => {}
            },
            Ok(Composite::Collage { .. }) => {}
        }
    }
    for (path, is_preview) in asset
        .file
        .iter()
        .map(|f| (f, false))
        .chain(asset.preview.iter().map(|p| (p, true)))
    {
        let Some(kind) = file_kind(path) else {
            if is_preview {
                out.push(PackProblem::new("packs.bad_asset_format", path.clone()));
            }
            continue;
        };
        let Some(bytes) = read(path) else {
            out.push(PackProblem::new("packs.missing_file", path.clone()));
            continue;
        };
        if !is_preview
            && let Some(hash) = &asset.hash
            && sha256(&bytes) != *hash
        {
            out.push(PackProblem::new("packs.hash_mismatch", asset.id.clone()));
        }
        let problem = match kind {
            FileKind::Svg => svg_problem(&bytes),
            FileKind::Png | FileKind::Webp => raster_check(&bytes, kind).err(),
        };
        if let Some(key) = problem {
            out.push(PackProblem::new(key, path.clone()));
        }
    }
    out
}

/// The problems of every file of a package: the thumbnail, then each
/// asset's (see [`asset_problems`]).
pub fn file_problems(
    manifest: &PackManifest,
    present: &BTreeSet<String>,
    mut read: impl FnMut(&str) -> Option<Vec<u8>>,
) -> Vec<PackProblem> {
    let mut out = thumbnail_problems(&mut read);
    for asset in &manifest.assets {
        out.extend(asset_problems(asset, present, &mut read));
    }
    out
}

/// How many assets of `manifest` go to each section, counted from its
/// assets, never from a number it states (§13.2).
#[must_use]
pub fn counts(manifest: &PackManifest) -> BTreeMap<PackSection, usize> {
    let mut out = BTreeMap::new();
    for section in manifest.assets.iter().filter_map(PackAsset::section) {
        *out.entry(section).or_insert(0) += 1;
    }
    out
}

/// A package looked at before it is installed (§8.2).
#[derive(Clone, Debug)]
pub struct PackInspection {
    pub manifest: PackManifest,
    /// The bytes of `thumb.webp`.
    pub thumbnail: Vec<u8>,
    /// The size of the package file, in bytes.
    pub size: u64,
    /// The version of this package already installed, if any.
    pub installed: Option<String>,
    /// The size and change time of the package file when it was checked:
    /// installing the same, unchanged file needs no second check.
    pub fingerprint: (u64, u128),
    /// Who signed it (spec 112).
    pub signer: crate::pack_signature::PackSigner,
}

/// How far a long task on a package has come.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PackProgress {
    /// What is done and the whole, in bytes.
    pub done: u64,
    pub total: u64,
    /// `true` while installing, `false` while checking.
    pub installing: bool,
}

impl PackProgress {
    /// The share done, in `[0, 1]`.
    #[must_use]
    pub fn share(self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.done as f64 / self.total as f64).clamp(0.0, 1.0) as f32
        }
    }
}

/// An installed package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledPack {
    pub manifest: PackManifest,
    /// The folder its files are in.
    pub dir: PathBuf,
    /// When it was installed, seconds since the Unix epoch.
    pub installed_at: u64,
    /// The size of its files, in bytes.
    pub size: u64,
    /// Who signed it, as checked when it was installed (spec 112).
    pub signer: crate::pack_signature::PackSigner,
}

/// What an installed asset becomes when it is used (§5, §6).
#[derive(Clone, Debug, PartialEq)]
pub enum PackUse {
    /// An image or SVG file, placed as it is.
    File(PathBuf),
    /// A frame that takes a photo, of the outline of the SVG `mask`.
    Frame { mask: PathBuf, fit: ContentFit },
    /// A collage of frames in these cells, `gap` apart.
    Collage {
        cells: Vec<(f64, f64, f64, f64)>,
        gap: f64,
    },
    /// A template, opened as a new design.
    Template(PathBuf),
}

impl InstalledPack {
    /// What `asset` becomes when it is used; `None` when its files are no
    /// longer there or do not read.
    #[must_use]
    pub fn usage(&self, asset: &PackAsset) -> Option<PackUse> {
        let kind = AssetKind::parse(&asset.kind)?;
        if let Some(def) = &asset.definition {
            let bytes = std::fs::read(self.dir.join(def)).ok()?;
            let found = composite_with(kind, &bytes, def, |p| self.dir.join(p).is_file()).ok()?;
            return Some(match found {
                Composite::Frame { mask, fit } => PackUse::Frame {
                    mask: self.dir.join(mask),
                    fit,
                },
                Composite::Collage { cells, gap } => PackUse::Collage { cells, gap },
            });
        }
        let file = self.dir.join(asset.file.as_ref()?);
        Some(if kind == AssetKind::Template {
            PackUse::Template(file)
        } else {
            PackUse::File(file)
        })
    }

    /// The file of `asset` on disk.
    #[must_use]
    pub fn file(&self, asset: &PackAsset) -> Option<PathBuf> {
        asset.file.as_ref().map(|f| self.dir.join(f))
    }

    /// The picture that shows `asset` in a list: its preview, else itself.
    #[must_use]
    pub fn preview(&self, asset: &PackAsset) -> Option<PathBuf> {
        asset
            .preview
            .as_ref()
            .or(asset.file.as_ref())
            .map(|f| self.dir.join(f))
    }

    /// Its thumbnail on disk.
    #[must_use]
    pub fn thumbnail(&self) -> PathBuf {
        self.dir.join(PACK_THUMBNAIL)
    }
}

/// Where installed packages are kept and how they are installed (spec 097).
///
/// Checking and installing run on a thread of their own and report how far
/// they are through `progress`, so the store is `Send + Sync`.
pub trait PackStore: Send + Sync {
    /// The package at `path`, read and checked, without installing it.
    ///
    /// # Errors
    /// What makes it invalid.
    fn inspect(
        &self,
        path: &Path,
        progress: &dyn Fn(PackProgress),
    ) -> Result<PackInspection, Vec<PackProblem>>;

    /// Installs the package at `path`, all or nothing: an older version of
    /// it is replaced only once the new one is in place. `checked` is the
    /// inspection of the same file, if any: when the file has not changed
    /// since, it is not checked again.
    ///
    /// # Errors
    /// What makes it invalid, or what failed on disk.
    fn install(
        &self,
        path: &Path,
        checked: Option<&PackInspection>,
        progress: &dyn Fn(PackProgress),
    ) -> Result<InstalledPack, Vec<PackProblem>>;

    /// Removes the installed package `id` and its files.
    ///
    /// # Errors
    /// What failed on disk.
    fn uninstall(&self, id: &str) -> Result<(), PackProblem>;

    /// The installed packages, by name.
    fn installed(&self) -> Vec<InstalledPack>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> Version {
        Version::parse("0.1.0-beta.4").expect("version")
    }

    /// A valid manifest of two illustrations and an icon, with the files
    /// it names.
    pub(crate) fn valid() -> (PackManifest, BTreeSet<String>) {
        let asset = |id: &str, kind: &str, file: &str| PackAsset {
            id: id.to_owned(),
            kind: kind.to_owned(),
            name: id.to_owned(),
            file: Some(file.to_owned()),
            ..PackAsset::default()
        };
        let manifest = PackManifest {
            format: "gafpack".to_owned(),
            format_version: "1.1".to_owned(),
            id: "com.gafira.test.pack".to_owned(),
            version: "1.0.0".to_owned(),
            package_type: "mixed".to_owned(),
            categories: vec!["illustrations".to_owned(), "icons".to_owned()],
            name: Localized {
                es: "Paquete de prueba".to_owned(),
                en: "Test pack".to_owned(),
            },
            description: Localized {
                es: "Unas ilustraciones y un icono de prueba.".to_owned(),
                en: "Some test illustrations and an icon.".to_owned(),
            },
            author: PackAuthor {
                name: "Gafira Studio".to_owned(),
                ..PackAuthor::default()
            },
            thumbnail: "thumb.webp".to_owned(),
            license: "CC0-1.0".to_owned(),
            min_gafira_version: "0.1.0-beta.1".to_owned(),
            assets: vec![
                asset("santa", "clipart", "assets/santa.png"),
                asset("tree", "illustration", "assets/tree.svg"),
                asset("gift", "icon", "assets/gift.svg"),
            ],
            ..PackManifest::default()
        };
        let present = [
            "thumb.webp",
            "assets/santa.png",
            "assets/tree.svg",
            "assets/gift.svg",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        (manifest, present)
    }

    fn keys(problems: &[PackProblem]) -> Vec<&'static str> {
        problems.iter().map(|p| p.key).collect()
    }

    fn descriptor(json: &str) -> Vec<u8> {
        json.as_bytes().to_vec()
    }

    // S97-12: a descriptor path resolves inside the package, from the
    // folder of the descriptor; it never leaves the package.
    #[test]
    fn s97_12_descriptor_paths_resolve_beside_it() {
        assert_eq!(
            resolve("assets/masks/heart.json", "heart.svg").as_deref(),
            Some("assets/masks/heart.svg")
        );
        assert_eq!(
            resolve("frame.json", "masks/a.svg").as_deref(),
            Some("masks/a.svg")
        );
        assert_eq!(resolve("assets/masks/heart.json", "../x.svg"), None);
        assert_eq!(resolve("assets/heart.json", "/etc/x.svg"), None);
    }

    // S97-12: frame and powerclip descriptors name a safe SVG mask in the
    // package and fit its content by cover or contain; a collage has
    // between one and 24 slots inside the page.
    #[test]
    fn s97_12_composite_descriptors_are_checked() {
        let present: BTreeSet<String> = ["assets/m/heart.svg".to_owned()].into();
        let clip = descriptor(
            r#"{"schema":"gafira.powerclip","version":1,"mask":"heart.svg","contentFit":"contain"}"#,
        );
        assert_eq!(
            composite(AssetKind::Powerclip, &clip, "assets/m/heart.json", &present),
            Ok(Composite::Frame {
                mask: "assets/m/heart.svg".to_owned(),
                fit: ContentFit::Contain,
            })
        );
        let frame = descriptor(r#"{"schema":"gafira.frame","version":1,"mask":"heart.svg"}"#);
        assert_eq!(
            composite(AssetKind::Frame, &frame, "assets/m/f.json", &present),
            Ok(Composite::Frame {
                mask: "assets/m/heart.svg".to_owned(),
                fit: ContentFit::Cover,
            })
        );
        let wrong_schema =
            descriptor(r#"{"schema":"gafira.frame","version":1,"mask":"heart.svg"}"#);
        assert_eq!(
            composite(
                AssetKind::Powerclip,
                &wrong_schema,
                "assets/m/h.json",
                &present
            ),
            Err("packs.bad_descriptor")
        );
        let missing = descriptor(r#"{"schema":"gafira.powerclip","version":1,"mask":"none.svg"}"#);
        assert_eq!(
            composite(AssetKind::Powerclip, &missing, "assets/m/h.json", &present),
            Err("packs.missing_file")
        );
        let fit = descriptor(
            r#"{"schema":"gafira.powerclip","version":1,"mask":"heart.svg","contentFit":"stretch"}"#,
        );
        assert_eq!(
            composite(AssetKind::Powerclip, &fit, "assets/m/h.json", &present),
            Err("packs.bad_descriptor")
        );
        let collage = descriptor(
            r#"{"schema":"gafira.collage","version":1,"gap":0.02,
                "slots":[{"x":0,"y":0,"w":0.5,"h":1},{"x":0.5,"y":0,"w":0.5,"h":1}]}"#,
        );
        assert_eq!(
            composite(AssetKind::Collage, &collage, "c.json", &present),
            Ok(Composite::Collage {
                cells: vec![(0.0, 0.0, 0.5, 1.0), (0.5, 0.0, 0.5, 1.0)],
                gap: 0.02,
            })
        );
        for bad in [
            r#"{"schema":"gafira.collage","version":1,"slots":[]}"#,
            r#"{"schema":"gafira.collage","version":1,"slots":[{"x":0.6,"y":0,"w":0.5,"h":1}]}"#,
            r#"{"schema":"gafira.collage","version":1,"slots":[{"x":0,"y":0,"w":0,"h":1}]}"#,
            r#"{"schema":"gafira.collage","version":2,"slots":[{"x":0,"y":0,"w":1,"h":1}]}"#,
            "not json",
        ] {
            assert_eq!(
                composite(AssetKind::Collage, &descriptor(bad), "c.json", &present),
                Err("packs.bad_descriptor"),
                "{bad}"
            );
        }
    }

    // S97-13: each composite type has what it needs: a frame a file or a
    // definition, a powerclip and a collage a definition, a template a
    // `.gaf` file; they go to Marcos, Collages and Plantillas.
    #[test]
    fn s97_13_composite_assets_in_the_manifest() {
        let (mut m, mut present) = valid();
        let add =
            |m: &mut PackManifest, id: &str, kind: &str, file: Option<&str>, def: Option<&str>| {
                m.assets.push(PackAsset {
                    id: id.to_owned(),
                    kind: kind.to_owned(),
                    name: id.to_owned(),
                    file: file.map(str::to_owned),
                    definition: def.map(str::to_owned),
                    ..PackAsset::default()
                });
            };
        add(&mut m, "deco", "frame", Some("assets/deco.svg"), None);
        add(&mut m, "photo", "frame", None, Some("assets/photo.json"));
        add(&mut m, "clip", "powerclip", None, Some("assets/clip.json"));
        add(&mut m, "two", "collage", None, Some("assets/two.json"));
        add(&mut m, "card", "template", Some("templates/card.gaf"), None);
        for file in [
            "assets/deco.svg",
            "assets/photo.json",
            "assets/clip.json",
            "assets/two.json",
            "templates/card.gaf",
        ] {
            present.insert(file.to_owned());
        }
        m.categories = [
            "illustrations",
            "icons",
            "frames",
            "powerclips",
            "collages",
            "templates",
        ]
        .map(str::to_owned)
        .to_vec();
        assert_eq!(manifest_problems(&m, &present, &app()), Vec::new());
        let counts = counts(&m);
        assert_eq!(counts.get(&PackSection::Frames), Some(&2));
        assert_eq!(counts.get(&PackSection::Templates), Some(&1));
        // What each type lacks.
        let mut bad = m.clone();
        present.insert("templates/card.json".to_owned());
        bad.assets[5].definition = None; // the powerclip
        bad.assets[7].file = Some("templates/card.json".to_owned()); // the template
        bad.assets[3].file = None; // the decorative frame, now with nothing
        let got = keys(&manifest_problems(&bad, &present, &app()));
        assert!(got.contains(&"packs.no_definition"), "{got:?}");
        assert!(got.contains(&"packs.bad_asset_format"), "{got:?}");
        assert!(got.contains(&"packs.no_asset_file"), "{got:?}");
    }

    // S97-01: a complete manifest is valid; its counts come from its
    // assets, by section.
    #[test]
    fn s97_01_a_complete_manifest_is_valid() {
        let (manifest, present) = valid();
        assert_eq!(manifest_problems(&manifest, &present, &app()), Vec::new());
        let counts = counts(&manifest);
        assert_eq!(counts.get(&PackSection::Illustrations), Some(&2));
        assert_eq!(counts.get(&PackSection::Icons), Some(&1));
    }

    // S97-01: the manifest reads the JSON of the spec, camelCase keys and
    // bilingual objects included.
    #[test]
    fn s97_01_the_manifest_reads_its_json() {
        let json = r#"{"format":"gafpack","formatVersion":"1.1","id":"com.a.b","version":"1.0.0",
            "packageType":"clipart","categories":["illustrations"],
            "name":{"es":"Uno","en":"One"},"description":{"es":"Una descripción","en":"A description"},
            "author":{"name":"A"},"thumbnail":"thumb.webp","license":"CC0-1.0",
            "minGafiraVersion":"0.1.0","x-extra":true,
            "assets":[{"id":"a","type":"clipart","name":"A","file":"a.png","placement":"illustrations"}]}"#;
        let manifest: PackManifest = serde_json::from_str(json).expect("reads");
        assert_eq!(manifest.format_version, "1.1");
        assert_eq!(manifest.name.get(true), "Uno");
        assert_eq!(manifest.name.get(false), "One");
        assert_eq!(
            manifest.assets[0].section(),
            Some(PackSection::Illustrations)
        );
    }

    // S97-02: what the spec requires is required: format, version, type,
    // both languages, author, thumbnail, licence text, assets.
    #[test]
    fn s97_02_missing_identity_is_refused() {
        let (mut m, present) = valid();
        m.package_type = "stickers".to_owned();
        m.name.en = "  ".to_owned();
        m.description.es = "Corta".to_owned();
        m.author.name.clear();
        m.thumbnail = "thumb.png".to_owned();
        m.license = "LicenseRef-Mine".to_owned();
        m.format_version = "1.0".to_owned();
        let got = keys(&manifest_problems(&m, &present, &app()));
        for key in [
            "packs.bad_type",
            "packs.bad_name",
            "packs.bad_description",
            "packs.no_author",
            "packs.bad_thumbnail",
            "packs.no_license_file",
            "packs.bad_format_version",
        ] {
            assert!(got.contains(&key), "{key} in {got:?}");
        }
        let (mut no_assets, present) = valid();
        no_assets.assets.clear();
        assert!(
            keys(&manifest_problems(&no_assets, &present, &app())).contains(&"packs.no_assets")
        );
        let (m, mut without_thumb) = valid();
        without_thumb.remove("thumb.webp");
        assert!(
            keys(&manifest_problems(&m, &without_thumb, &app())).contains(&"packs.no_thumbnail")
        );
    }

    // S97-02: the categories are exactly the sections of the assets, and a
    // specific package type has its own section among them.
    #[test]
    fn s97_02_categories_follow_the_assets() {
        let (mut m, present) = valid();
        m.categories.push("frames".to_owned());
        assert!(keys(&manifest_problems(&m, &present, &app())).contains(&"packs.bad_categories"));
        let (mut m, present) = valid();
        m.categories = vec!["illustrations".to_owned()];
        assert!(keys(&manifest_problems(&m, &present, &app())).contains(&"packs.bad_categories"));
        let (mut m, present) = valid();
        m.package_type = "shapes".to_owned();
        assert!(
            keys(&manifest_problems(&m, &present, &app())).contains(&"packs.type_without_assets")
        );
    }

    // S97-03: each asset has a known type, an allowed placement, a unique
    // id, and a safe file that is in the package; a composite type this
    // version cannot insert is refused, never shown in a wrong place.
    #[test]
    fn s97_03_assets_are_checked() {
        let (mut m, present) = valid();
        m.assets[1].id = "santa".to_owned();
        m.assets[0].placement = Some("templates".to_owned());
        m.assets[2].file = Some("../gift.svg".to_owned());
        let got = keys(&manifest_problems(&m, &present, &app()));
        assert!(got.contains(&"packs.duplicate_asset"), "{got:?}");
        assert!(got.contains(&"packs.bad_placement"), "{got:?}");
        assert!(got.contains(&"packs.unsafe_path"), "{got:?}");
        let (mut m, present) = valid();
        m.assets[2].file = Some("assets/missing.svg".to_owned());
        assert!(keys(&manifest_problems(&m, &present, &app())).contains(&"packs.missing_file"));
        // A powerclip without its descriptor is refused.
        let (mut m, present) = valid();
        m.assets[0].kind = "powerclip".to_owned();
        m.assets[0].placement = None;
        m.categories.push("powerclips".to_owned());
        assert!(keys(&manifest_problems(&m, &present, &app())).contains(&"packs.no_definition"));
    }

    // S97-04: a package for a newer Gafira is not installed.
    #[test]
    fn s97_04_versions_are_compared_as_semver() {
        let v = |t: &str| Version::parse(t).expect(t);
        assert!(v("0.1.0-beta.4") < v("0.1.0"));
        assert!(v("0.1.0-beta.4") > v("0.1.0-beta.1"));
        assert!(v("0.1.0-beta.10") > v("0.1.0-beta.9"));
        assert!(v("1.2.3") > v("1.2.0"));
        assert!(Version::parse("1.2").is_none());
        assert!(Version::parse("1.x.0").is_none());
        let (mut m, present) = valid();
        m.min_gafira_version = "1.0.0".to_owned();
        assert!(keys(&manifest_problems(&m, &present, &app())).contains(&"packs.needs_newer"));
    }

    // S97-05: a path that leaves the package, or is absolute or ambiguous,
    // is not safe.
    #[test]
    fn s97_05_unsafe_paths() {
        for bad in [
            "../x.png",
            "/etc/passwd",
            "a/../../b",
            "C:/x.png",
            "a\\b.png",
            "a//b.png",
            "./a.png",
            "",
            "a/b/c/d/e/f/g/h/i.png",
        ] {
            assert!(!safe_path(bad), "{bad}");
        }
        for good in ["thumb.webp", "assets/icons/gift.svg", "previews/a b.webp"] {
            assert!(safe_path(good), "{good}");
        }
        assert!(valid_id("com.gafira.christmas-clipart"));
        assert!(!valid_id("Christmas"));
        assert!(!valid_id("com..x"));
    }

    // S97-06: an SVG with a script, an event, an entity, a foreignObject
    // or a link outside itself is refused; a plain one passes.
    #[test]
    fn s97_06_active_svg_is_refused() {
        let ok = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><defs><linearGradient id="g"/></defs><rect width="10" height="10" fill="url(#g)"/><use href="#g"/></svg>"##;
        assert_eq!(svg_problem(ok), None);
        let bad: [&[u8]; 6] = [
            br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg"><rect onload="x()"/></svg>"#,
            br#"<!DOCTYPE svg [<!ENTITY x SYSTEM "file:///etc/passwd">]><svg xmlns="http://www.w3.org/2000/svg">&x;</svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg"><foreignObject/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><image xlink:href="https://x.test/a.png"/></svg>"#,
            br#"<svg xmlns="http://www.w3.org/2000/svg"><rect fill="url(https://x.test/p)"/></svg>"#,
        ];
        for svg in bad {
            assert!(
                svg_problem(svg).is_some(),
                "{}",
                String::from_utf8_lossy(svg)
            );
        }
        assert_eq!(svg_problem(b"not xml"), Some("packs.svg_unreadable"));
    }

    fn webp(w: u32, h: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut out),
                image::ImageFormat::WebP,
            )
            .expect("webp");
        out
    }

    fn png(clear: bool) -> Vec<u8> {
        let alpha = if clear { 0 } else { 255 };
        let mut image = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 255]));
        image.put_pixel(0, 0, image::Rgba([0, 0, 0, alpha]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("png");
        out
    }

    // S97-07: the thumbnail is a still, square WebP within the sides; a
    // PNG, a wrong size or an animation is refused.
    #[test]
    fn s97_07_the_thumbnail_is_a_square_still_webp() {
        assert_eq!(thumbnail_problem(&webp(256, 256)), None);
        assert_eq!(
            thumbnail_problem(&webp(300, 256)),
            Some("packs.thumb_bad_size")
        );
        assert_eq!(
            thumbnail_problem(&webp(128, 128)),
            Some("packs.thumb_bad_size")
        );
        assert_eq!(thumbnail_problem(&png(false)), Some("packs.thumb_not_webp"));
        let mut animated = webp(256, 256);
        // A VP8X header with the animation flag after the RIFF header.
        let mut chunk = b"VP8X".to_vec();
        chunk.extend_from_slice(&10u32.to_le_bytes());
        chunk.extend_from_slice(&[0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        animated.splice(12..12, chunk);
        assert_eq!(thumbnail_problem(&animated), Some("packs.thumb_animated"));
    }

    // S97-07: an image asset decodes and says whether it has see-through
    // pixels; a declared hash must match its bytes.
    #[test]
    fn s97_07_files_decode_and_match_their_hash() {
        assert_eq!(raster_check(&png(true), FileKind::Png), Ok((8, 8, true)));
        assert_eq!(raster_check(&png(false), FileKind::Png), Ok((8, 8, false)));
        assert_eq!(
            raster_check(b"nope", FileKind::Png),
            Err("packs.image_unreadable")
        );
        let (mut manifest, _) = valid();
        let santa = png(true);
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4"><rect width="4" height="4"/></svg>"#.to_vec();
        manifest.assets[0].hash = Some(sha256(&santa));
        let thumb = webp(256, 256);
        let files = |name: &str| -> Option<Vec<u8>> {
            match name {
                "thumb.webp" => Some(thumb.clone()),
                "assets/santa.png" => Some(santa.clone()),
                "assets/tree.svg" | "assets/gift.svg" => Some(svg.clone()),
                _ => None,
            }
        };
        let present = BTreeSet::new();
        assert_eq!(file_problems(&manifest, &present, files), Vec::new());
        manifest.assets[0].hash = Some("0".repeat(64));
        let got = keys(&file_problems(&manifest, &present, files));
        assert_eq!(got, vec!["packs.hash_mismatch"]);
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
