//! Marca (spec 093, D-124): the brand kits a person keeps (logos, colour
//! palettes, text styles and fonts, photos, graphics and icons) and the brand
//! templates made from their designs. The kits live outside the documents,
//! like Biblioteca; the shell reaches them through [`BrandStore`].

use std::path::{Path, PathBuf};

use gafira_ports::repository::TemplateMeta;

use crate::recent::RecentThumbnail;
use crate::templates::TemplateListing;

/// The kinds of files a kit keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BrandFiles {
    Logos,
    Photos,
    Graphics,
    Icons,
    /// Font files (TTF, OTF) the person uploads for the kit's styles.
    Fonts,
}

impl BrandFiles {
    pub const ALL: [Self; 5] = [
        Self::Logos,
        Self::Photos,
        Self::Graphics,
        Self::Icons,
        Self::Fonts,
    ];

    /// The folder of the kit they are copied to, and their key in the
    /// stored file.
    #[must_use]
    pub fn folder(self) -> &'static str {
        match self {
            Self::Logos => "logos",
            Self::Photos => "photos",
            Self::Graphics => "graphics",
            Self::Icons => "icons",
            Self::Fonts => "fonts",
        }
    }
}

/// A colour of a kit: what the screen shows, `0xRRGGBB`, and, for print,
/// the CMYK the person gave it, each part in percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrandColor {
    pub rgb: u32,
    pub cmyk: Option<[u8; 4]>,
}

impl BrandColor {
    #[must_use]
    pub const fn rgb(rgb: u32) -> Self {
        Self { rgb, cmyk: None }
    }

    /// The screen colour of CMYK `[c, m, y, k]` in percent, the plain
    /// conversion without a colour profile.
    #[must_use]
    pub fn rgb_of_cmyk([c, m, y, k]: [u8; 4]) -> u32 {
        let part = |v: u8| 1.0 - f64::from(v.min(100)) / 100.0;
        let channel = |v: u8| (255.0 * part(v) * part(k)).round() as u32;
        (channel(c) << 16) | (channel(m) << 8) | channel(y)
    }
}

/// A named palette of a kit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrandPalette {
    pub name: String,
    pub colors: Vec<BrandColor>,
}

/// One text style of a kit: its name («Título», «Cuerpo»…), a family of
/// the font library, its size in px and its weight and slant.
#[derive(Clone, Debug, PartialEq)]
pub struct BrandTextStyle {
    pub name: String,
    pub font: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
}

/// A brand kit.
#[derive(Clone, Debug, PartialEq)]
pub struct BrandKit {
    /// Stable, unique among the kits; names its folder.
    pub id: String,
    pub name: String,
    pub logos: Vec<PathBuf>,
    pub photos: Vec<PathBuf>,
    pub graphics: Vec<PathBuf>,
    pub icons: Vec<PathBuf>,
    /// Uploaded font files.
    pub fonts: Vec<PathBuf>,
    pub palettes: Vec<BrandPalette>,
    /// Its text styles, largest first.
    pub styles: Vec<BrandTextStyle>,
}

impl BrandKit {
    /// A kit with only its name: everything else is added later. Its
    /// styles start as `styles` (their names in the person's language).
    #[must_use]
    pub fn new(id: String, name: String) -> Self {
        Self::with_styles(
            id,
            name,
            ["Título", "Subtítulo", "Encabezado de sección", "Cuerpo"],
        )
    }

    /// [`BrandKit::new`] with the names of its four starting styles.
    #[must_use]
    pub fn with_styles(id: String, name: String, names: [&str; 4]) -> Self {
        let style = |name: &str, font: &str, size: f64, bold: bool| BrandTextStyle {
            name: name.to_owned(),
            font: font.to_owned(),
            size,
            bold,
            italic: false,
        };
        Self {
            id,
            name,
            logos: Vec::new(),
            photos: Vec::new(),
            graphics: Vec::new(),
            icons: Vec::new(),
            fonts: Vec::new(),
            palettes: Vec::new(),
            styles: vec![
                style(names[0], "Manrope", 64.0, true),
                style(names[1], "Manrope", 40.0, true),
                style(names[2], "Inter", 28.0, true),
                style(names[3], "Inter", 18.0, false),
            ],
        }
    }

    /// The files of `kind`.
    #[must_use]
    pub fn files(&self, kind: BrandFiles) -> &Vec<PathBuf> {
        match kind {
            BrandFiles::Logos => &self.logos,
            BrandFiles::Photos => &self.photos,
            BrandFiles::Graphics => &self.graphics,
            BrandFiles::Icons => &self.icons,
            BrandFiles::Fonts => &self.fonts,
        }
    }

    pub fn files_mut(&mut self, kind: BrandFiles) -> &mut Vec<PathBuf> {
        match kind {
            BrandFiles::Logos => &mut self.logos,
            BrandFiles::Photos => &mut self.photos,
            BrandFiles::Graphics => &mut self.graphics,
            BrandFiles::Icons => &mut self.icons,
            BrandFiles::Fonts => &mut self.fonts,
        }
    }

    /// Every colour of its palettes, each once, in their order.
    #[must_use]
    pub fn colors(&self) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for color in self.palettes.iter().flat_map(|p| p.colors.iter()) {
            if !out.contains(&color.rgb) {
                out.push(color.rgb);
            }
        }
        out
    }
}

/// The tag that ties a template to its kit.
pub const BRAND_TAG: &str = "brand:";

/// Where the kits live. The composition root supplies it (files in the data
/// folder); the shell does no I/O of its own (02 §2.1). A failure is the
/// store's to log.
pub trait BrandStore {
    /// Every kit, in the order they were made.
    fn kits(&self) -> Vec<BrandKit>;
    /// Keep `kit`, in place of the one with its id or as a new one.
    fn save(&self, kit: &BrandKit);
    /// Remove the kit with `id`, with its files and templates.
    fn delete(&self, id: &str);
    /// Copy `source` into the folder of `kind` of the kit `id`; its path
    /// there.
    fn add_file(&self, id: &str, kind: BrandFiles, source: &Path) -> Option<PathBuf>;
    /// The kit in use, if one was chosen.
    fn active(&self) -> Option<String>;
    fn set_active(&self, id: &str);
    /// The brand templates of the kit `id`.
    fn templates(&self, id: &str) -> Vec<TemplateListing>;
    /// Copy the `.gaf` at `source` into the templates of the kit `id`,
    /// marked with `meta`; its path there.
    ///
    /// # Errors
    /// What went wrong, for the person.
    fn add_template(&self, id: &str, source: &Path, meta: &TemplateMeta)
    -> Result<PathBuf, String>;
    /// Remove the brand template at `path`.
    fn remove_template(&self, path: &Path);
    /// The thumbnail of the template at `path`.
    fn thumbnail(&self, path: &Path) -> Option<RecentThumbnail>;
}

/// An id no kit of `kits` has: `kit-<n>`.
#[must_use]
pub fn new_id(kits: &[BrandKit]) -> String {
    (1..)
        .map(|n| format!("kit-{n}"))
        .find(|id| kits.iter().all(|k| &k.id != id))
        .unwrap_or_default()
}

/// The `template` of a brand template of the kit `id`, named `name`, whose
/// first page has the size `kind` of «Crear un diseño» when it has one.
#[must_use]
pub fn template_meta(id: &str, name: &str, kind: Option<&str>) -> TemplateMeta {
    let mut tags = vec![format!("{BRAND_TAG}{id}"), format!("title:{name}")];
    if let Some(kind) = kind {
        tags.push(kind.to_owned());
    }
    TemplateMeta {
        tags,
        page_format: "custom".to_owned(),
        style: "brand".to_owned(),
        color: "#18243B".to_owned(),
    }
}

/// `0xRRGGBB` from `#RRGGBB`, `RRGGBB` or `#RGB`, any case.
#[must_use]
pub fn parse_hex(text: &str) -> Option<u32> {
    let hex = text.trim().trim_start_matches('#');
    let full = match hex.len() {
        3 => hex.chars().flat_map(|c| [c, c]).collect(),
        6 => hex.to_owned(),
        _ => return None,
    };
    u32::from_str_radix(&full, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s93_02_a_new_kit_needs_only_its_name_and_gets_a_free_id() {
        let first = BrandKit::new(new_id(&[]), "Café".to_owned());
        assert_eq!(first.id, "kit-1");
        assert!(first.logos.is_empty() && first.palettes.is_empty());
        let second = BrandKit::new(new_id(std::slice::from_ref(&first)), "Otra".to_owned());
        assert_eq!(second.id, "kit-2");
        let gap = [second.clone()];
        assert_eq!(new_id(&gap), "kit-1", "a free id is reused");
    }

    #[test]
    fn s93_03_colours_come_from_every_palette_once() {
        let mut kit = BrandKit::new("kit-1".to_owned(), "Café".to_owned());
        kit.palettes = vec![
            BrandPalette {
                name: "Principal".to_owned(),
                colors: vec![BrandColor::rgb(0x112233), BrandColor::rgb(0xFFFFFF)],
            },
            BrandPalette {
                name: "Acentos".to_owned(),
                colors: vec![
                    BrandColor::rgb(0xFFFFFF),
                    BrandColor {
                        rgb: 0xFF6B57,
                        cmyk: Some([0, 58, 66, 0]),
                    },
                ],
            },
        ];
        assert_eq!(kit.colors(), [0x112233, 0xFFFFFF, 0xFF6B57]);
    }

    #[test]
    fn s93_03_cmyk_shows_as_its_plain_screen_colour() {
        assert_eq!(BrandColor::rgb_of_cmyk([0, 0, 0, 0]), 0xFFFFFF);
        assert_eq!(BrandColor::rgb_of_cmyk([0, 0, 0, 100]), 0x000000);
        assert_eq!(BrandColor::rgb_of_cmyk([100, 0, 0, 0]), 0x00FFFF);
        assert_eq!(BrandColor::rgb_of_cmyk([0, 50, 100, 0]), 0xFF8000);
        assert_eq!(
            BrandColor::rgb_of_cmyk([0, 0, 0, 250]),
            0x000000,
            "over 100 is 100"
        );
    }

    #[test]
    fn s93_03_hex_codes_are_read_in_every_usual_form() {
        assert_eq!(parse_hex("#FF6B57"), Some(0xFF6B57));
        assert_eq!(parse_hex("ff6b57"), Some(0xFF6B57));
        assert_eq!(parse_hex(" #abc "), Some(0xAABBCC));
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("azul"), None);
    }

    #[test]
    fn s93_07_a_brand_template_names_its_kit_and_its_size() {
        let meta = template_meta("kit-2", "Promo semanal", Some("size.ig_square"));
        assert_eq!(
            meta.tags,
            ["brand:kit-2", "title:Promo semanal", "size.ig_square"]
        );
        let plain = template_meta("kit-2", "Libre", None);
        assert_eq!(plain.tags.len(), 2);
    }
}
