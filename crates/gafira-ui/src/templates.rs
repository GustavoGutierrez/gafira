//! The templates of Inicio and Plantillas (spec 091, D-81): the port the
//! shell lists them through, and what the screens make of the list: each
//! template's kind (a size of «Crear un diseño», spec 076), its category and
//! language, the search and filters over them, and the pages of a grid that
//! loads as it scrolls.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};

use gafira_ports::i18n::Locale;
use gafira_ports::repository::TemplateMeta;

use crate::design_sizes::{self, Category, DesignSize};
use crate::recent::RecentThumbnail;

/// A template as the store lists it: where it is and its `template` object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateListing {
    pub path: PathBuf,
    pub meta: TemplateMeta,
}

/// Where the templates live. The composition root supplies it (the bundled
/// ones, later the person's own); the shell does no I/O of its own (02 §2.1).
/// It is read on a thread of its own, so it is `Send + Sync`.
pub trait TemplateStore: Send + Sync {
    /// Every template, in the order to show them.
    fn list(&self) -> Vec<TemplateListing>;

    /// The thumbnail of the template at `path`, read without opening it.
    fn thumbnail(&self, path: &Path) -> Option<RecentThumbnail>;
}

/// The tag that gives a template's language.
const LANG: &str = "lang:";
/// The tag that gives a template's name; without it, the file's name.
const TITLE: &str = "title:";

/// One template as the screens use it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Template {
    pub(crate) path: PathBuf,
    /// Its name: its `title:` tag, else the file's without `.gaf`.
    pub(crate) name: String,
    /// The first tag that names a size of «Crear un diseño».
    pub(crate) kind: Option<&'static DesignSize>,
    /// The language of its texts; `None` for a neutral template.
    pub(crate) lang: Option<Locale>,
    /// Its words, lowercase, for the search.
    words: String,
    pub(crate) color: String,
}

impl Template {
    pub(crate) fn new(listing: &TemplateListing) -> Self {
        let meta = &listing.meta;
        let name = meta
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix(TITLE))
            .map(str::to_owned)
            .or_else(|| {
                let stem = listing.path.file_stem()?.to_string_lossy();
                Some(stem.split('.').next().unwrap_or_default().to_owned())
            })
            .unwrap_or_default();
        let kind = meta.tags.iter().find_map(|tag| design_sizes::by_key(tag));
        let lang = meta
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix(LANG))
            .and_then(|code| match code {
                "es" => Some(Locale::Es),
                "en" => Some(Locale::En),
                _ => None,
            });
        let mut words = vec![name.clone(), meta.style.clone(), meta.page_format.clone()];
        words.extend(
            meta.tags
                .iter()
                .filter(|t| !t.starts_with(LANG) && !t.starts_with(TITLE))
                .cloned(),
        );
        if let Some(kind) = kind {
            words.push(kind.words.to_owned());
        }
        Self {
            path: listing.path.clone(),
            name,
            kind,
            lang,
            words: words.join(" ").to_lowercase(),
            color: meta.color.clone(),
        }
    }

    /// The category of «Crear un diseño» its kind belongs to.
    pub(crate) fn category(&self) -> Option<Category> {
        self.kind.map(|kind| kind.category)
    }

    /// Where it goes in the list: by the categories of «Crear un diseño»
    /// in their order, those of no category last; the store's order within.
    pub(crate) fn order(&self) -> usize {
        self.category()
            .and_then(|c| Category::ALL.iter().position(|a| *a == c))
            .unwrap_or(Category::ALL.len())
    }

    /// Whether a person whose interface is in `locale` sees it: the
    /// templates of that language and the neutral ones (01 §7.3).
    pub(crate) fn shown_in(&self, locale: Locale) -> bool {
        self.lang.is_none_or(|lang| lang == locale)
    }
}

/// What Plantillas, or the search of Inicio, narrows the list to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Filter {
    pub(crate) query: String,
    pub(crate) category: Option<Category>,
    /// A size key: the templates of one kind.
    pub(crate) kind: Option<&'static str>,
}

/// The templates of `all` that `filter` keeps for `locale`, in their order.
/// Each word of the query must be in the template's words or in `names`
/// of its kind and category (the names in the interface's language).
pub(crate) fn filtered<'a>(
    all: &'a [Template],
    filter: &Filter,
    locale: Locale,
    names: impl Fn(&Template) -> String,
) -> Vec<&'a Template> {
    let words: Vec<String> = filter
        .query
        .split_whitespace()
        .map(str::to_lowercase)
        .collect();
    all.iter()
        .filter(|t| t.shown_in(locale))
        .filter(|t| filter.category.is_none_or(|c| t.category() == Some(c)))
        .filter(|t| {
            filter
                .kind
                .is_none_or(|k| t.kind.is_some_and(|own| own.key == k))
        })
        .filter(|t| {
            if words.is_empty() {
                return true;
            }
            let own = format!("{} {}", t.words, names(t).to_lowercase());
            words.iter().all(|w| own.contains(w.as_str()))
        })
        .collect()
}

/// How many cards one page of the grid adds.
pub(crate) const PAGE: usize = 24;

/// The first `pages` pages of `list`: what the grid shows so far.
pub(crate) fn shown<T>(list: &[T], pages: usize) -> &[T] {
    &list[..list.len().min(pages.saturating_mul(PAGE))]
}

/// The kinds `templates` have, each once, in the order they first appear,
/// with their templates: the rows of «Plantillas para ti» and the cards of
/// «Explorar por tipo».
pub(crate) fn by_kind<'a>(
    templates: &[&'a Template],
) -> Vec<(&'static DesignSize, Vec<&'a Template>)> {
    let mut rows: Vec<(&'static DesignSize, Vec<&'a Template>)> = Vec::new();
    for template in templates {
        let Some(kind) = template.kind else { continue };
        match rows.iter_mut().find(|(k, _)| k.key == kind.key) {
            Some((_, list)) => list.push(template),
            None => rows.push((kind, vec![template])),
        }
    }
    rows
}

/// What the worker thread hands back.
enum Loaded {
    List(Vec<TemplateListing>),
    Thumbnail(PathBuf, Option<RecentThumbnail>),
}

/// A thumbnail as the screens have it.
enum Thumb {
    Asked,
    Ready(egui::TextureHandle, (u32, u32)),
    Missing,
}

/// The templates while the app runs: the list and the thumbnails come from
/// the store on a thread of its own (03 §10: the index of templates is a
/// deferred step), and each thumbnail is asked for the first time its card
/// is drawn, so a long list loads as it scrolls.
pub(crate) struct Catalogue {
    list: Option<Vec<Template>>,
    thumbs: BTreeMap<PathBuf, Thumb>,
    asks: Sender<PathBuf>,
    loaded: Receiver<Loaded>,
}

impl Catalogue {
    /// Starts reading `store`; `ctx` repaints the window as things arrive.
    pub(crate) fn new(store: Option<Arc<dyn TemplateStore>>, ctx: &egui::Context) -> Self {
        let (asks, asked) = mpsc::channel::<PathBuf>();
        let (send, loaded) = mpsc::channel();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("templates".to_owned())
            .spawn(move || {
                let list = store.as_ref().map(|s| s.list()).unwrap_or_default();
                if send.send(Loaded::List(list)).is_err() {
                    return;
                }
                ctx.request_repaint();
                while let Ok(path) = asked.recv() {
                    let thumb = store.as_ref().and_then(|s| s.thumbnail(&path));
                    if send.send(Loaded::Thumbnail(path, thumb)).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            });
        if let Err(error) = spawned {
            eprintln!("templates: {error}");
        }
        Self {
            list: None,
            thumbs: BTreeMap::new(),
            asks,
            loaded,
        }
    }

    /// Take what the thread has finished; the textures are made here.
    pub(crate) fn pump(&mut self, ctx: &egui::Context) {
        while let Ok(loaded) = self.loaded.try_recv() {
            match loaded {
                Loaded::List(list) => {
                    let mut list: Vec<Template> = list.iter().map(Template::new).collect();
                    list.sort_by_key(Template::order);
                    self.list = Some(list);
                }
                Loaded::Thumbnail(path, Some(thumb)) => {
                    let texture = ctx.load_texture(
                        format!("template:{}", path.display()),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [thumb.width as usize, thumb.height as usize],
                            &thumb.rgba,
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    self.thumbs
                        .insert(path, Thumb::Ready(texture, (thumb.width, thumb.height)));
                }
                Loaded::Thumbnail(path, None) => {
                    self.thumbs.insert(path, Thumb::Missing);
                }
            }
        }
    }

    /// Every template, once the list has been read.
    pub(crate) fn list(&self) -> Option<&[Template]> {
        self.list.as_deref()
    }

    /// The thumbnail of `template` and its size, asking for it the first
    /// time; `None` while it loads or when it has none.
    pub(crate) fn thumbnail(
        &mut self,
        template: &Template,
    ) -> Option<(egui::TextureId, (u32, u32))> {
        match self.thumbs.get(&template.path) {
            Some(Thumb::Ready(texture, size)) => Some((texture.id(), *size)),
            Some(Thumb::Asked | Thumb::Missing) => None,
            None => {
                if self.asks.send(template.path.clone()).is_ok() {
                    self.thumbs.insert(template.path.clone(), Thumb::Asked);
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(name: &str, tags: &[&str]) -> TemplateListing {
        TemplateListing {
            path: PathBuf::from(format!("/templates/{name}.gaf")),
            meta: TemplateMeta {
                tags: tags.iter().map(|t| (*t).to_owned()).collect(),
                page_format: "social".to_owned(),
                style: "minimal".to_owned(),
                color: "#FF6B57".to_owned(),
            },
        }
    }

    fn catalogue() -> Vec<Template> {
        [
            listing("Café de barrio", &["size.ig_square", "lang:es", "comida"]),
            listing("Coffee shop", &["size.ig_square", "lang:en", "food"]),
            listing("Oferta de verano", &["size.fb_post", "lang:es", "ventas"]),
            listing("Informe anual", &["size.pres_16_9", "lang:es"]),
            listing("Geometría", &["size.a4_v"]),
            listing("Sin tipo", &["lang:es"]),
        ]
        .iter()
        .map(Template::new)
        .collect()
    }

    fn names(all: &[&Template]) -> Vec<String> {
        all.iter().map(|t| t.name.clone()).collect()
    }

    fn none(_: &Template) -> String {
        String::new()
    }

    #[test]
    fn s91_04_the_kind_language_and_name_come_from_the_file() {
        let all = catalogue();
        assert_eq!(all[0].name, "Café de barrio");
        assert_eq!(all[0].kind.map(|k| k.key), Some("size.ig_square"));
        assert_eq!(all[0].category(), Some(Category::Social));
        assert_eq!(all[0].lang, Some(Locale::Es));
        assert_eq!(all[3].category(), Some(Category::Presentations));
        assert_eq!(all[4].lang, None, "no lang tag: neutral");
        assert_eq!(all[5].kind, None, "no size tag: no kind");
    }

    #[test]
    fn s91_04_the_title_tag_names_it_over_the_file() {
        let titled = Template::new(&listing(
            "corner-coffee.es",
            &[
                "size.ig_square",
                "lang:es",
                "title:Café de barrio",
                "coffee",
            ],
        ));
        assert_eq!(titled.name, "Café de barrio");
        let plain = Template::new(&listing("corner-coffee.es", &["size.ig_square"]));
        assert_eq!(
            plain.name, "corner-coffee",
            "the file name without its language"
        );
        // Its words in the other language find it too.
        let all = [titled];
        let query = Filter {
            query: "coffee".to_owned(),
            ..Filter::default()
        };
        assert_eq!(
            filtered(&all, &query, Locale::Es, |_| String::new()).len(),
            1
        );
    }

    #[test]
    fn s91_04_a_person_sees_their_language_and_the_neutral_ones() {
        let all = catalogue();
        let es = filtered(&all, &Filter::default(), Locale::Es, none);
        assert_eq!(
            names(&es),
            [
                "Café de barrio",
                "Oferta de verano",
                "Informe anual",
                "Geometría",
                "Sin tipo"
            ]
        );
        let en = filtered(&all, &Filter::default(), Locale::En, none);
        assert_eq!(names(&en), ["Coffee shop", "Geometría"]);
    }

    #[test]
    fn s91_04_category_kind_and_words_narrow_the_list() {
        let all = catalogue();
        let social = Filter {
            category: Some(Category::Social),
            ..Filter::default()
        };
        assert_eq!(
            names(&filtered(&all, &social, Locale::Es, none)),
            ["Café de barrio", "Oferta de verano"]
        );
        let facebook = Filter {
            kind: Some("size.fb_post"),
            ..Filter::default()
        };
        assert_eq!(
            names(&filtered(&all, &facebook, Locale::Es, none)),
            ["Oferta de verano"]
        );
        // S91-05: «facebook» finds the templates of a Facebook size, by the
        // words of the size; every word must match, in any case.
        let query = |q: &str| Filter {
            query: q.to_owned(),
            ..Filter::default()
        };
        assert_eq!(
            names(&filtered(&all, &query("FACEBOOK"), Locale::Es, none)),
            ["Oferta de verano"]
        );
        assert_eq!(
            names(&filtered(&all, &query("café barrio"), Locale::Es, none)),
            ["Café de barrio"]
        );
        assert!(filtered(&all, &query("café verano"), Locale::Es, none).is_empty());
        // The names of the kind in the interface's language count too.
        let named = filtered(&all, &query("presentación"), Locale::Es, |t| {
            if t.category() == Some(Category::Presentations) {
                "Presentación".to_owned()
            } else {
                String::new()
            }
        });
        assert_eq!(names(&named), ["Informe anual"]);
    }

    #[test]
    fn s91_04_pages_cover_the_list_once() {
        let list: Vec<usize> = (0..50).collect();
        assert!(shown(&list, 0).is_empty());
        assert_eq!(shown(&list, 1), &list[..PAGE]);
        assert_eq!(shown(&list, 2), &list[..2 * PAGE]);
        assert_eq!(shown(&list, 3), &list[..], "the last page is shorter");
        assert_eq!(shown(&list, usize::MAX), &list[..]);
    }

    #[test]
    fn s91_07_rows_by_kind_keep_the_first_order() {
        let all = catalogue();
        let es = filtered(&all, &Filter::default(), Locale::Es, none);
        let rows = by_kind(&es);
        let keys: Vec<&str> = rows.iter().map(|(k, _)| k.key).collect();
        assert_eq!(
            keys,
            [
                "size.ig_square",
                "size.fb_post",
                "size.pres_16_9",
                "size.a4_v"
            ]
        );
        assert_eq!(rows[0].1.len(), 1);
    }
}
