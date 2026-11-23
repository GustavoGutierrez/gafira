//! «Crear un diseño» (spec 076): the sizes a design can start from, by
//! category and platform, with what each one looks like in its card, a
//! search, and the label of its size in the unit it is known by.

use gafira_doc::{Guide, GuideAxis, GuideSet, Page, PageId, PageSize};
use gafira_geom::{Length, Unit};
use gafira_ports::i18n::Locale;

/// The categories of the side list, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Category {
    ForYou,
    Social,
    Presentations,
    Video,
    Print,
    Documents,
    Photos,
    Web,
    Custom,
}

impl Category {
    pub(crate) const ALL: [Self; 9] = [
        Self::ForYou,
        Self::Social,
        Self::Presentations,
        Self::Video,
        Self::Print,
        Self::Documents,
        Self::Photos,
        Self::Web,
        Self::Custom,
    ];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::ForYou => "create.for_you",
            Self::Social => "create.social",
            Self::Presentations => "create.presentations",
            Self::Video => "create.video",
            Self::Print => "create.print",
            Self::Documents => "create.documents",
            Self::Photos => "create.photos",
            Self::Web => "create.web",
            Self::Custom => "create.custom",
        }
    }

    /// The colour of its icon (0xRRGGBB).
    pub(crate) fn colour(self) -> u32 {
        match self {
            Self::ForYou => 0x3FBFA9,
            Self::Social => 0xFF6B57,
            Self::Presentations => 0xF4A21B,
            Self::Video => 0x7B61FF,
            Self::Print => 0x2563EB,
            Self::Documents => 0x17695A,
            Self::Photos => 0xE0457B,
            Self::Web => 0x0097B2,
            Self::Custom => 0x18243B,
        }
    }
}

/// How the card of a size is drawn (spec 076, `design_art`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Look {
    /// A sheet of paper with its corner folded: documents and print.
    Sheet,
    /// A business card: two cards, one over the other.
    Card,
    /// A poster: shapes over a coloured sheet.
    Poster,
    /// An invitation: a scalloped frame.
    Invitation,
    /// A folded brochure: three panels.
    Brochure,
    /// A postcard: half colour, half text.
    Postcard,
    /// A label or sticker.
    Label,
    /// A flyer: a picture block and lines.
    Flyer,
    /// An infographic: a header and a rising chart.
    Infographic,
    /// A certificate: a white sheet with a seal.
    Certificate,
    /// A slide with a chart.
    Slide,
    /// A phone: stories and vertical video.
    Phone,
    /// A social post.
    Post,
    /// A video frame with its play button.
    Video,
    /// A browser window.
    Web,
    /// An email.
    Email,
    /// A photo.
    Photo,
}

/// One size: its name's key, the words that find it, its size and unit,
/// its look, its category and its group within it (a platform, a kind of
/// print), if any.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DesignSize {
    pub(crate) key: &'static str,
    pub(crate) words: &'static str,
    pub(crate) w: f64,
    pub(crate) h: f64,
    pub(crate) unit: Unit,
    pub(crate) look: Look,
    pub(crate) category: Category,
    pub(crate) group: Option<&'static str>,
}

const fn size(
    key: &'static str,
    words: &'static str,
    (w, h, unit): (f64, f64, Unit),
    look: Look,
    category: Category,
    group: Option<&'static str>,
) -> DesignSize {
    DesignSize {
        key,
        words,
        w,
        h,
        unit,
        look,
        category,
        group,
    }
}

use Category as C;
use Look as L;
use Unit::{In, Mm, Px};

/// Every size, by category.
pub(crate) const SIZES: [DesignSize; 56] = [
    // Social media, by platform (names only, no brand marks).
    size(
        "size.ig_square",
        "instagram post cuadrado square",
        (1080.0, 1080.0, Px),
        L::Post,
        C::Social,
        Some("instagram"),
    ),
    size(
        "size.ig_portrait",
        "instagram post vertical 4:5 portrait",
        (1080.0, 1350.0, Px),
        L::Post,
        C::Social,
        Some("instagram"),
    ),
    size(
        "size.ig_story",
        "instagram historia story reel",
        (1080.0, 1920.0, Px),
        L::Phone,
        C::Social,
        Some("instagram"),
    ),
    size(
        "size.fb_post",
        "facebook post publicación",
        (1200.0, 630.0, Px),
        L::Post,
        C::Social,
        Some("facebook"),
    ),
    size(
        "size.fb_cover",
        "facebook portada cover",
        (1640.0, 624.0, Px),
        L::Post,
        C::Social,
        Some("facebook"),
    ),
    size(
        "size.fb_story",
        "facebook historia story",
        (1080.0, 1920.0, Px),
        L::Phone,
        C::Social,
        Some("facebook"),
    ),
    size(
        "size.li_post",
        "linkedin post publicación",
        (1200.0, 627.0, Px),
        L::Post,
        C::Social,
        Some("linkedin"),
    ),
    size(
        "size.li_banner",
        "linkedin banner portada",
        (1584.0, 396.0, Px),
        L::Post,
        C::Social,
        Some("linkedin"),
    ),
    size(
        "size.pin",
        "pinterest pin",
        (1000.0, 1500.0, Px),
        L::Post,
        C::Social,
        Some("pinterest"),
    ),
    size(
        "size.x_post",
        "x twitter post publicación",
        (1600.0, 900.0, Px),
        L::Post,
        C::Social,
        Some("x"),
    ),
    size(
        "size.x_header",
        "x twitter encabezado header",
        (1500.0, 500.0, Px),
        L::Post,
        C::Social,
        Some("x"),
    ),
    size(
        "size.yt_thumb",
        "youtube miniatura thumbnail",
        (1280.0, 720.0, Px),
        L::Video,
        C::Social,
        Some("youtube"),
    ),
    size(
        "size.yt_banner",
        "youtube banner canal channel",
        (2560.0, 1440.0, Px),
        L::Post,
        C::Social,
        Some("youtube"),
    ),
    size(
        "size.yt_shorts",
        "youtube shorts cortos vertical",
        (1080.0, 1920.0, Px),
        L::Phone,
        C::Social,
        Some("youtube"),
    ),
    size(
        "size.tiktok",
        "tiktok video vertical",
        (1080.0, 1920.0, Px),
        L::Phone,
        C::Social,
        Some("tiktok"),
    ),
    size(
        "size.wa_status",
        "whatsapp estado status",
        (1080.0, 1920.0, Px),
        L::Phone,
        C::Social,
        Some("whatsapp"),
    ),
    // Presentations.
    size(
        "size.pres_16_9",
        "presentación presentation 16:9 diapositiva slide",
        (1920.0, 1080.0, Px),
        L::Slide,
        C::Presentations,
        None,
    ),
    size(
        "size.pres_4_3",
        "presentación presentation 4:3 diapositiva slide",
        (1024.0, 768.0, Px),
        L::Slide,
        C::Presentations,
        None,
    ),
    size(
        "size.pres_mobile",
        "presentación móvil mobile vertical",
        (1080.0, 1920.0, Px),
        L::Phone,
        C::Presentations,
        None,
    ),
    // Video.
    size(
        "size.video_h",
        "video horizontal 16:9 full hd",
        (1920.0, 1080.0, Px),
        L::Video,
        C::Video,
        None,
    ),
    size(
        "size.video_v",
        "video vertical móvil mobile 9:16",
        (1080.0, 1920.0, Px),
        L::Phone,
        C::Video,
        None,
    ),
    size(
        "size.video_sq",
        "video cuadrado square",
        (1080.0, 1080.0, Px),
        L::Video,
        C::Video,
        None,
    ),
    size(
        "size.video_4k",
        "video 4k uhd",
        (3840.0, 2160.0, Px),
        L::Video,
        C::Video,
        None,
    ),
    // Print, by kind.
    size(
        "size.business_card",
        "tarjeta de presentación visita business card",
        (85.0, 55.0, Mm),
        L::Card,
        C::Print,
        Some("cards"),
    ),
    size(
        "size.poster_a2",
        "afiche póster poster cartel a2",
        (420.0, 594.0, Mm),
        L::Poster,
        C::Print,
        Some("marketing"),
    ),
    size(
        "size.invitation",
        "invitación invitation",
        (105.0, 148.0, Mm),
        L::Invitation,
        C::Print,
        Some("cards"),
    ),
    size(
        "size.trifold",
        "folleto tríptico brochure trifold",
        (11.0, 8.5, In),
        L::Brochure,
        C::Print,
        Some("marketing"),
    ),
    size(
        "size.flyer_a4",
        "volante flyer a4",
        (210.0, 297.0, Mm),
        L::Flyer,
        C::Print,
        Some("marketing"),
    ),
    size(
        "size.infographic",
        "infografía infographic dl",
        (99.0, 210.0, Mm),
        L::Infographic,
        C::Print,
        Some("marketing"),
    ),
    size(
        "size.postcard",
        "postal postcard a6",
        (148.0, 105.0, Mm),
        L::Postcard,
        C::Print,
        Some("cards"),
    ),
    size(
        "size.label",
        "etiqueta label",
        (100.0, 60.0, Mm),
        L::Label,
        C::Print,
        Some("labels"),
    ),
    size(
        "size.sticker",
        "sticker pegatina adhesivo",
        (3.0, 3.0, In),
        L::Label,
        C::Print,
        Some("labels"),
    ),
    size(
        "size.gift_certificate",
        "certificado de regalo gift certificate",
        (8.0, 3.5, In),
        L::Certificate,
        C::Print,
        Some("gifts"),
    ),
    size(
        "size.folding_card",
        "tarjeta plegable folding card felicitación",
        (5.0, 7.0, In),
        L::Invitation,
        C::Print,
        Some("gifts"),
    ),
    size(
        "size.letterhead",
        "membrete letterhead a4",
        (210.0, 297.0, Mm),
        L::Sheet,
        C::Print,
        Some("documents"),
    ),
    size(
        "size.letter_v",
        "carta letter vertical us documento",
        (8.5, 11.0, In),
        L::Sheet,
        C::Print,
        Some("documents"),
    ),
    size(
        "size.letter_h",
        "carta letter horizontal us",
        (11.0, 8.5, In),
        L::Sheet,
        C::Print,
        Some("documents"),
    ),
    size(
        "size.a4_v",
        "a4 vertical documento",
        (210.0, 297.0, Mm),
        L::Sheet,
        C::Print,
        Some("documents"),
    ),
    size(
        "size.a4_h",
        "a4 horizontal documento",
        (297.0, 210.0, Mm),
        L::Sheet,
        C::Print,
        Some("documents"),
    ),
    size(
        "size.a3_v",
        "a3 vertical",
        (297.0, 420.0, Mm),
        L::Sheet,
        C::Print,
        Some("documents"),
    ),
    size(
        "size.a3_h",
        "a3 horizontal",
        (420.0, 297.0, Mm),
        L::Sheet,
        C::Print,
        Some("documents"),
    ),
    size(
        "size.a5_v",
        "a5 vertical",
        (148.0, 210.0, Mm),
        L::Sheet,
        C::Print,
        Some("documents"),
    ),
    size(
        "size.poster",
        "póster poster cartel grande",
        (18.0, 24.0, In),
        L::Poster,
        C::Print,
        Some("marketing"),
    ),
    // Documents.
    size(
        "size.doc_a4",
        "documento document a4",
        (210.0, 297.0, Mm),
        L::Sheet,
        C::Documents,
        None,
    ),
    size(
        "size.doc_letter",
        "documento document carta letter",
        (8.5, 11.0, In),
        L::Sheet,
        C::Documents,
        None,
    ),
    size(
        "size.resume",
        "currículum curriculum resume cv",
        (210.0, 297.0, Mm),
        L::Sheet,
        C::Documents,
        None,
    ),
    size(
        "size.report",
        "informe report",
        (8.5, 11.0, In),
        L::Sheet,
        C::Documents,
        None,
    ),
    // Photos.
    size(
        "size.photo_4_3",
        "foto photo 4:3",
        (1600.0, 1200.0, Px),
        L::Photo,
        C::Photos,
        None,
    ),
    size(
        "size.photo_3_2",
        "foto photo 3:2",
        (1800.0, 1200.0, Px),
        L::Photo,
        C::Photos,
        None,
    ),
    size(
        "size.photo_sq",
        "foto photo cuadrada square collage",
        (2048.0, 2048.0, Px),
        L::Photo,
        C::Photos,
        None,
    ),
    size(
        "size.photo_v",
        "foto photo vertical retrato portrait",
        (1200.0, 1800.0, Px),
        L::Photo,
        C::Photos,
        None,
    ),
    // Web and email.
    size(
        "size.web_banner",
        "banner web anuncio ad",
        (1200.0, 300.0, Px),
        L::Web,
        C::Web,
        None,
    ),
    size(
        "size.email_header",
        "correo email encabezado header newsletter",
        (600.0, 200.0, Px),
        L::Email,
        C::Web,
        None,
    ),
    size(
        "size.email",
        "correo email newsletter boletín",
        (600.0, 900.0, Px),
        L::Email,
        C::Web,
        None,
    ),
    size(
        "size.website",
        "sitio web website página page",
        (1440.0, 900.0, Px),
        L::Web,
        C::Web,
        None,
    ),
    size(
        "size.blog_header",
        "blog encabezado header",
        (1200.0, 630.0, Px),
        L::Web,
        C::Web,
        None,
    ),
];

/// The groups of a category, with their chip's key: the platforms of
/// «Redes sociales» and the kinds of «Imprimir».
pub(crate) fn groups(category: Category) -> &'static [(&'static str, &'static str)] {
    match category {
        Category::Social => &PLATFORMS,
        Category::Print => &PRINT_KINDS,
        _ => &[],
    }
}

const PRINT_KINDS: [(&str, &str); 5] = [
    ("marketing", "create.kind.marketing"),
    ("cards", "create.kind.cards"),
    ("documents", "create.kind.documents"),
    ("gifts", "create.kind.gifts"),
    ("labels", "create.kind.labels"),
];

/// The platforms of «Redes sociales», with their chip's key.
const PLATFORMS: [(&str, &str); 8] = [
    ("facebook", "create.platform.facebook"),
    ("instagram", "create.platform.instagram"),
    ("linkedin", "create.platform.linkedin"),
    ("pinterest", "create.platform.pinterest"),
    ("tiktok", "create.platform.tiktok"),
    ("x", "create.platform.x"),
    ("whatsapp", "create.platform.whatsapp"),
    ("youtube", "create.platform.youtube"),
];

/// The size named `key`.
pub(crate) fn by_key(key: &str) -> Option<&'static DesignSize> {
    SIZES.iter().find(|s| s.key == key)
}

/// The rows of «Para ti»: their title's key and their sizes.
pub(crate) const FOR_YOU: [(&str, &[&str]); 3] = [
    (
        "create.most_used",
        &["size.a4_h", "size.video_h", "size.tiktok", "size.ig_square"],
    ),
    (
        "create.popular",
        &[
            "size.pres_16_9",
            "size.a4_v",
            "size.ig_portrait",
            "size.poster_a2",
            "size.yt_thumb",
        ],
    ),
    (
        "create.try_something",
        &[
            "size.video_v",
            "size.ig_story",
            "size.business_card",
            "size.invitation",
            "size.website",
        ],
    ),
];

/// The sizes «Elegir el tamaño» shows under its fields.
pub(crate) const POPULAR_LAYOUTS: [&str; 8] = [
    "size.letter_v",
    "size.a4_h",
    "size.a3_v",
    "size.a3_h",
    "size.a4_v",
    "size.a5_v",
    "size.pres_16_9",
    "size.ig_square",
];

/// The sizes of `category`, of `group` when one is chosen.
pub(crate) fn of(category: Category, group: Option<&str>) -> Vec<&'static DesignSize> {
    SIZES
        .iter()
        .filter(|s| s.category == category)
        .filter(|s| group.is_none_or(|g| s.group == Some(g)))
        .collect()
}

/// The sizes whose words or name hold every word of `text`.
pub(crate) fn search(text: &str, name: impl Fn(&DesignSize) -> String) -> Vec<&'static DesignSize> {
    let words: Vec<String> = text.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    SIZES
        .iter()
        .filter(|s| {
            let own = format!("{} {}", s.words, name(s)).to_lowercase();
            words.iter().all(|w| own.contains(w.as_str()))
        })
        .collect()
}

impl DesignSize {
    /// Its page size.
    pub(crate) fn page_size(&self) -> PageSize {
        page_size(self.w, self.h, self.unit)
    }

    /// Its size as people know it: px, inches, or centimetres for
    /// millimetre sizes («21 × 29,7 cm»).
    pub(crate) fn label(&self, locale: Locale) -> String {
        size_label(self.w, self.h, self.unit, locale)
    }

    /// Width over height.
    pub(crate) fn aspect(&self) -> f64 {
        self.w / self.h
    }

    /// The guides its page starts with, in its own unit: the folds, locked
    /// (drawn dashed), and the safe margins, the safe zone of stories and
    /// videos, the thirds of a photo or the column of a website, free.
    pub(crate) fn guides(&self) -> GuideSet {
        let (w, h) = (self.w, self.h);
        let mut out = Vec::new();
        let mut line = |axis, at: f64, locked| {
            out.push(Guide {
                axis,
                position: Length::new((at * 1000.0).round() / 1000.0, self.unit),
                locked,
            });
        };
        let inset = |line: &mut dyn FnMut(GuideAxis, f64, bool),
                     (top, side, bottom): (f64, f64, f64)| {
            line(GuideAxis::Vertical, side, false);
            line(GuideAxis::Vertical, w - side, false);
            line(GuideAxis::Horizontal, top, false);
            line(GuideAxis::Horizontal, h - bottom, false);
        };
        // A safe margin of `mm` millimetres in this size's unit.
        let mm = |v: f64| match self.unit {
            Unit::In => v / 25.4,
            Unit::Cm => v / 10.0,
            _ => v,
        };
        match self.key {
            "size.trifold" => {
                line(GuideAxis::Vertical, w / 3.0, true);
                line(GuideAxis::Vertical, w * 2.0 / 3.0, true);
                inset(&mut line, (mm(6.0), mm(6.0), mm(6.0)));
            }
            "size.folding_card" | "size.postcard" => {
                line(
                    GuideAxis::Vertical,
                    w / 2.0,
                    self.key == "size.folding_card",
                );
                inset(&mut line, (mm(5.0), mm(5.0), mm(5.0)));
            }
            "size.gift_certificate" => {
                line(GuideAxis::Vertical, w * 0.74, true);
                inset(&mut line, (mm(5.0), mm(5.0), mm(5.0)));
            }
            "size.letterhead" => {
                line(GuideAxis::Horizontal, 40.0, false);
                inset(&mut line, (20.0, 20.0, 20.0));
            }
            "size.yt_banner" => {
                // The part every screen shows: 1546 × 423 in the middle.
                line(GuideAxis::Vertical, (w - 1546.0) / 2.0, false);
                line(GuideAxis::Vertical, (w + 1546.0) / 2.0, false);
                line(GuideAxis::Horizontal, (h - 423.0) / 2.0, false);
                line(GuideAxis::Horizontal, (h + 423.0) / 2.0, false);
            }
            "size.website" => {
                line(GuideAxis::Vertical, (w - 1200.0) / 2.0, false);
                line(GuideAxis::Vertical, (w + 1200.0) / 2.0, false);
                line(GuideAxis::Horizontal, 80.0, false);
            }
            _ => match self.look {
                // Stories and vertical video: what the app's own buttons
                // leave free above and below.
                Look::Phone => inset(&mut line, (h * 0.13, w * 0.06, h * 0.18)),
                Look::Photo => {
                    for k in [1.0, 2.0] {
                        line(GuideAxis::Vertical, w * k / 3.0, false);
                        line(GuideAxis::Horizontal, h * k / 3.0, false);
                    }
                }
                Look::Video | Look::Slide => {
                    let m = h * 0.05;
                    inset(&mut line, (m, m, m));
                    line(GuideAxis::Vertical, w / 2.0, false);
                    line(GuideAxis::Horizontal, h / 2.0, false);
                }
                Look::Email => inset(&mut line, (20.0, 20.0, 20.0)),
                Look::Post | Look::Web => {
                    let m = w.min(h) * 0.05;
                    inset(&mut line, (m, m, m));
                }
                Look::Sheet => {
                    let m = mm(20.0);
                    inset(&mut line, (m, m, m));
                }
                Look::Poster | Look::Flyer => {
                    let m = mm(10.0);
                    inset(&mut line, (m, m, m));
                }
                _ => {
                    let m = mm(if self.category == Category::Print {
                        4.0
                    } else {
                        0.0
                    });
                    if m > 0.0 {
                        inset(&mut line, (m, m, m));
                    }
                }
            },
        }
        GuideSet(out)
    }

    /// Its first page: its size and guides.
    pub(crate) fn page(&self) -> Page {
        let mut page = Page::new(PageId::new(1), String::new(), self.page_size());
        page.guides = self.guides();
        page
    }
}

/// A page of `w` × `h` in `unit`.
pub(crate) fn page_size(w: f64, h: f64, unit: Unit) -> PageSize {
    PageSize {
        width: Length::new(w, unit),
        height: Length::new(h, unit),
    }
}

/// `w × h` in its unit, with the decimal mark of `locale`.
pub(crate) fn size_label(w: f64, h: f64, unit: Unit, locale: Locale) -> String {
    let (w, h, name) = match unit {
        Unit::Mm => (w / 10.0, h / 10.0, "cm"),
        Unit::In => (w, h, "in"),
        Unit::Cm => (w, h, "cm"),
        _ => (w, h, "px"),
    };
    let number = |v: f64| {
        let text = format!("{:.2}", v);
        let text = text.trim_end_matches('0').trim_end_matches('.').to_owned();
        if matches!(locale, Locale::Es) {
            text.replace('.', ",")
        } else {
            text
        }
    };
    format!("{} × {} {name}", number(w), number(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec 076: every size is a valid page; every key of «Para ti» and of
    // the popular layouts names one; the sizes say their unit.
    #[test]
    fn every_size_is_a_page() {
        let dpi = gafira_doc::Dpi::DEFAULT;
        for size in SIZES {
            assert!(size.page_size().check(dpi).is_ok(), "{}", size.key);
        }
        for (_, keys) in FOR_YOU {
            for key in keys {
                assert!(by_key(key).is_some(), "{key}");
            }
        }
        for key in POPULAR_LAYOUTS {
            assert!(by_key(key).is_some(), "{key}");
        }
        let a4 = by_key("size.a4_v").expect("a4");
        assert_eq!(a4.label(Locale::Es), "21 × 29,7 cm");
        assert_eq!(a4.label(Locale::En), "21 × 29.7 cm");
        assert_eq!(
            by_key("size.letter_v").expect("letter").label(Locale::Es),
            "8,5 × 11 in"
        );
        assert_eq!(
            by_key("size.ig_square").expect("ig").label(Locale::Es),
            "1080 × 1080 px"
        );
    }

    // Spec 076: each size starts with its guides, inside its page and
    // valid; a tri-fold brochure has its two folds, locked.
    #[test]
    fn sizes_start_with_their_guides() {
        let dpi = gafira_doc::Dpi::DEFAULT;
        for size in SIZES {
            let page = size.page();
            assert!(page.check_layout(dpi).is_ok(), "{}", size.key);
            for guide in &page.guides.0 {
                let at = guide.position.value();
                let room = match guide.axis {
                    GuideAxis::Vertical => size.w,
                    GuideAxis::Horizontal => size.h,
                };
                assert!(at > 0.0 && at < room, "{} {at}", size.key);
            }
        }
        let trifold = by_key("size.trifold").expect("trifold").guides();
        let folds: Vec<f64> = trifold
            .0
            .iter()
            .filter(|g| g.locked)
            .map(|g| g.position.value())
            .collect();
        assert_eq!(folds.len(), 2);
        assert!((folds[0] - 11.0 / 3.0).abs() < 1e-3 && (folds[1] - 22.0 / 3.0).abs() < 1e-3);
    }

    // Spec 076: a search finds by words in Spanish or English; a group (a
    // platform, a kind of print) narrows its category, and none is empty.
    #[test]
    fn sizes_are_found_and_filtered() {
        let none = |_: &DesignSize| String::new();
        assert!(
            search("historia instagram", none)
                .iter()
                .any(|s| s.key == "size.ig_story")
        );
        assert!(
            search("resume", none)
                .iter()
                .any(|s| s.key == "size.resume")
        );
        assert!(search("zzz", none).is_empty());
        let youtube = of(Category::Social, Some("youtube"));
        assert!(youtube.len() >= 3 && youtube.iter().all(|s| s.group == Some("youtube")));
        assert!(of(Category::Print, None).len() >= 15);
        for category in Category::ALL {
            for (group, _) in groups(category) {
                assert!(!of(category, Some(group)).is_empty(), "{group}");
            }
        }
    }
}

#[cfg(test)]
mod texts {
    use gafira_doc::msg::MessageKey;
    use gafira_i18n::CatalogTranslator;
    use gafira_ports::i18n::{Locale, Translator};

    use super::*;

    // Spec 076: every name of «Crear un diseño» reads in both languages.
    #[test]
    fn every_name_is_written() {
        let translator = CatalogTranslator::new();
        let mut keys: Vec<&str> = SIZES.iter().map(|s| s.key).collect();
        for category in Category::ALL {
            keys.push(category.key());
            keys.extend(groups(category).iter().map(|(_, key)| *key));
        }
        keys.extend(FOR_YOU.iter().map(|(key, _)| *key));
        keys.extend([
            "create.title",
            "create.search_hint",
            "create.results",
            "create.no_results",
            "create.popular_layouts",
            "create.all",
            "create.width",
            "create.height",
            "create.units",
            "create.lock_ratio",
            "create.unlock_ratio",
            "create.create",
            "create.out_of_range",
            "create.size_limits",
        ]);
        for key in keys {
            for locale in [Locale::Es, Locale::En] {
                assert!(
                    translator.has(locale, MessageKey(key)),
                    "{key} ({locale:?})"
                );
            }
        }
    }
}
