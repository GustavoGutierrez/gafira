//! The filters (S85-03): data, not code.

use gafira_doc::ImageAdjust;

/// A filter: what it does first, and over that.
#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    pub id: &'static str,
    pub first: ImageAdjust,
    pub then: ImageAdjust,
}

/// Filters alike.
#[derive(Clone, Debug, PartialEq)]
pub struct FilterGroup {
    pub id: &'static str,
    pub filters: [&'static str; 6],
}

/// The groups, in the order they are shown.
pub const FILTER_GROUPS: [FilterGroup; 6] = [
    FilterGroup {
        id: "natural",
        filters: ["alba", "brisa", "prado", "lino", "rocio", "sauce"],
    },
    FilterGroup {
        id: "warm",
        filters: ["ambar", "miel", "terracota", "ocaso", "canela", "duna"],
    },
    FilterGroup {
        id: "cold",
        filters: [
            "marea", "glaciar", "niebla", "cobalto", "pizarra", "invierno",
        ],
    },
    FilterGroup {
        id: "vivid",
        filters: ["carnaval", "tropico", "neon", "arrecife", "fiesta", "selva"],
    },
    FilterGroup {
        id: "soft",
        filters: ["nube", "algodon", "petalo", "perla", "susurro", "crema"],
    },
    FilterGroup {
        id: "classic",
        filters: ["sepia", "postal", "cinta", "recuerdo", "album", "mono"],
    },
];

/// The filter `id`; none for one that is not known.
#[must_use]
pub fn filter(id: &str) -> Option<Filter> {
    let adjust = |list: [f64; 11]| {
        let [
            temperature,
            tint,
            brightness,
            contrast,
            highlights,
            shadows,
            saturation,
            vibrance,
            blacks,
            whites,
            vignette,
        ] = list;
        ImageAdjust {
            temperature,
            tint,
            brightness,
            contrast,
            highlights,
            shadows,
            saturation,
            vibrance,
            blacks,
            whites,
            vignette,
            ..ImageAdjust::default()
        }
    };
    let none = [0.0; 11];
    // Temperature, tint, brightness, contrast, highlights, shadows,
    // saturation, vibrance, blacks, whites, vignette.
    let (id, first, then) = match id {
        "alba" => (
            "alba",
            [6., 2., 6., 4., -10., 12., 4., 12., 0., 0., 0.],
            none,
        ),
        "brisa" => (
            "brisa",
            [-8., -2., 5., 8., 0., 8., 6., 14., 0., 4., 0.],
            none,
        ),
        "prado" => (
            "prado",
            [2., -10., 2., 10., -8., 6., 12., 16., 0., 0., 0.],
            none,
        ),
        "lino" => (
            "lino",
            [4., 0., 8., -8., -6., 14., -14., 0., -8., 0., 0.],
            none,
        ),
        "rocio" => (
            "rocio",
            [-4., -6., 8., 2., 4., 10., -4., 18., 0., 6., 0.],
            none,
        ),
        "sauce" => (
            "sauce",
            [2., -6., -4., 12., -14., 4., -8., 8., 6., 0., -8.],
            none,
        ),
        "ambar" => (
            "ambar",
            [34., 4., 2., 10., -6., 8., 8., 10., 0., 0., 0.],
            none,
        ),
        "miel" => (
            "miel",
            [26., 0., 10., 2., -10., 14., 2., 8., -6., 0., 0.],
            none,
        ),
        "terracota" => (
            "terracota",
            [30., 12., -4., 16., -12., 0., 6., 6., 8., 0., -10.],
            none,
        ),
        "ocaso" => (
            "ocaso",
            [42., 16., 0., 12., -16., 8., 14., 8., 0., 0., -14.],
            none,
        ),
        "canela" => (
            "canela",
            [22., 6., -8., 14., -8., -4., -18., 0., 4., 0., -8.],
            none,
        ),
        "duna" => (
            "duna",
            [28., -2., 8., -4., -14., 16., -16., 4., -10., 0., 0.],
            none,
        ),
        "marea" => (
            "marea",
            [-26., -6., 0., 12., -4., 6., 6., 12., 0., 0., 0.],
            none,
        ),
        "glaciar" => (
            "glaciar",
            [-34., 0., 10., 6., 4., 12., -10., 4., 0., 8., 0.],
            none,
        ),
        "niebla" => (
            "niebla",
            [-16., 2., 12., -18., -8., 18., -24., 0., -18., 0., 0.],
            none,
        ),
        "cobalto" => (
            "cobalto",
            [-40., 8., -6., 20., -8., 0., 16., 8., 8., 0., -12.],
            none,
        ),
        "pizarra" => (
            "pizarra",
            [-14., -2., -6., 18., -10., 4., -46., 0., 6., 0., -10.],
            none,
        ),
        "invierno" => (
            "invierno",
            [-28., 6., 6., 10., 6., 6., -22., 6., 0., 10., 0.],
            none,
        ),
        "carnaval" => (
            "carnaval",
            [6., 6., 4., 18., -6., 8., 32., 24., 4., 4., 0.],
            none,
        ),
        "tropico" => (
            "tropico",
            [10., -12., 4., 14., -10., 12., 26., 30., 0., 0., 0.],
            none,
        ),
        "neon" => (
            "neon",
            [-10., 18., 2., 28., 0., 6., 40., 20., 10., 8., 0.],
            none,
        ),
        "arrecife" => (
            "arrecife",
            [-18., -14., 4., 16., -8., 10., 28., 26., 0., 4., 0.],
            none,
        ),
        "fiesta" => (
            "fiesta",
            [18., 10., 6., 22., -8., 10., 30., 20., 6., 6., 0.],
            none,
        ),
        "selva" => (
            "selva",
            [0., -20., -2., 20., -12., 6., 22., 20., 8., 0., -8.],
            none,
        ),
        "nube" => (
            "nube",
            [-6., 2., 14., -22., -6., 22., -12., 6., -16., 0., 0.],
            none,
        ),
        "algodon" => (
            "algodon",
            [8., 4., 16., -18., -10., 20., -10., 8., -14., 0., 0.],
            none,
        ),
        "petalo" => (
            "petalo",
            [10., 20., 10., -12., -8., 16., -4., 8., -12., 0., 0.],
            none,
        ),
        "perla" => (
            "perla",
            [0., 6., 14., -10., -14., 16., -26., 0., -10., 0., 0.],
            none,
        ),
        "susurro" => (
            "susurro",
            [-10., 10., 8., -20., -10., 20., -20., 4., -20., 0., 0.],
            none,
        ),
        "crema" => (
            "crema",
            [20., 4., 12., -16., -12., 18., -14., 4., -14., 0., 0.],
            none,
        ),
        "sepia" => (
            "sepia",
            [0., 0., 4., 4., 0., 8., -100., 0., -6., 0., 0.],
            [46., 8., 0., 0., 0., 0., 0., 0., 0., 0., -14.],
        ),
        "postal" => (
            "postal",
            [16., -6., 6., -10., -12., 10., -22., 0., -20., 0., -10.],
            none,
        ),
        "cinta" => (
            "cinta",
            [12., 8., 0., 16., -18., 14., -10., 6., -14., 0., -18.],
            none,
        ),
        "recuerdo" => (
            "recuerdo",
            [24., 12., 8., -14., -14., 14., -30., 0., -24., 0., -12.],
            none,
        ),
        "album" => (
            "album",
            [0., 0., 6., -4., -8., 12., -72., 0., -16., 0., 0.],
            [18., 2., 0., 0., 0., 0., 0., 0., 0., 0., -16.],
        ),
        "mono" => (
            "mono",
            [0., 0., 0., 18., -6., 6., -100., 0., 6., 4., 0.],
            none,
        ),
        _ => return None,
    };
    Some(Filter {
        id,
        first: adjust(first),
        then: adjust(then),
    })
}
