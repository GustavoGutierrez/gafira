//! The pictures of «Crear un diseño» (spec 076): each size drawn as an SVG
//! in the colours of Gafira, in its own proportions (a sheet with its
//! corner folded, a business card, a phone…), and the icons of the side
//! list. They are rasterised once with resvg and kept as textures.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::design_sizes::{Category, Look};

/// The view box of a card picture.
const W: f64 = 240.0;
const H: f64 = 170.0;

const INK: &str = "#18243B";
const CORAL: &str = "#FF6B57";
const MINT: &str = "#3FBFA9";
const MIST: &str = "#E7EAF2";
const WHITE: &str = "#FFFFFF";
const SHADOW: &str = "#B9C1D3";

/// A scheme: the main colour, the motif's, and a light tint of the main.
#[derive(Clone, Copy)]
struct Scheme {
    main: &'static str,
    motif: &'static str,
    light: &'static str,
}

const SCHEMES: [Scheme; 3] = [
    Scheme {
        main: MINT,
        motif: INK,
        light: "#CFEFE8",
    },
    Scheme {
        main: INK,
        motif: CORAL,
        light: "#8C97B0",
    },
    Scheme {
        main: CORAL,
        motif: INK,
        light: "#FFD3CC",
    },
];

/// The scheme of a size, picked from its key so neighbours differ.
fn scheme(key: &str) -> Scheme {
    let sum: usize = key.bytes().map(usize::from).sum();
    SCHEMES[sum % SCHEMES.len()]
}

/// The box `aspect` (width over height) takes inside `max_w` × `max_h`,
/// centred in the picture.
fn fit(aspect: f64, max_w: f64, max_h: f64) -> (f64, f64, f64, f64) {
    let aspect = aspect.clamp(0.2, 5.0);
    let (w, h) = if aspect >= max_w / max_h {
        (max_w, max_w / aspect)
    } else {
        (max_h * aspect, max_h)
    };
    ((W - w) / 2.0, (H - h) / 2.0, w, h)
}

/// The flower of Gafira's pictures: seven petals round a light heart.
fn bloom(out: &mut String, (cx, cy): (f64, f64), r: f64, petal: &str, heart: &str) {
    for i in 0..7 {
        let turn = f64::from(i) * 360.0 / 7.0;
        let _ = write!(
            out,
            r#"<ellipse cx="{cx:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" fill="{petal}" transform="rotate({turn:.1} {cx:.1} {cy:.1})"/>"#,
            cy - r * 0.5,
            r * 0.27,
            r * 0.5
        );
    }
    let _ = write!(
        out,
        r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{:.1}" fill="{heart}"/>"#,
        r * 0.16
    );
}

/// A rounded bar: a line of text.
fn bar(out: &mut String, (x, y, w, h): (f64, f64, f64, f64), fill: &str) {
    let _ = write!(
        out,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" rx="{:.1}" fill="{fill}"/>"#,
        h / 2.0
    );
}

fn rect(out: &mut String, (x, y, w, h): (f64, f64, f64, f64), r: f64, fill: &str) {
    let _ = write!(
        out,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" rx="{r:.1}" fill="{fill}"/>"#
    );
}

/// A box with its shadow, down and to the right.
fn shadowed(out: &mut String, b: (f64, f64, f64, f64), r: f64, fill: &str) {
    rect(out, (b.0 + 5.0, b.1 + 5.0, b.2, b.3), r, SHADOW);
    rect(out, b, r, fill);
}

/// A play button.
fn play(out: &mut String, (cx, cy): (f64, f64), r: f64) {
    let _ = write!(
        out,
        r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{INK}" fill-opacity="0.85"/><path d="M{:.1} {:.1} L{:.1} {cy:.1} L{:.1} {:.1} Z" fill="{WHITE}"/>"#,
        cx - r * 0.3,
        cy - r * 0.42,
        cx + r * 0.45,
        cx - r * 0.3,
        cy + r * 0.42
    );
}

/// A rising chart in a box: an area under a broken line.
fn chart(out: &mut String, (x, y, w, h): (f64, f64, f64, f64), fill: &str) {
    let _ = write!(
        out,
        r#"<path d="M{x:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} L{:.1} {y:.1} L{:.1} {:.1} Z" fill="{fill}"/>"#,
        y + h,
        x + w * 0.3,
        y + h * 0.55,
        x + w * 0.5,
        y + h * 0.7,
        x + w * 0.75,
        y + h * 0.3,
        x + w,
        x + w,
        y + h
    );
}

/// The SVG of the card of a size: `look` in `aspect`, coloured by `key`.
pub(crate) fn card_svg(look: Look, aspect: f64, key: &str) -> String {
    let s = scheme(key);
    let mut o = String::new();
    match look {
        Look::Sheet => sheet(&mut o, aspect, s),
        Look::Card => {
            let (x, y, w, h) = fit(aspect, 140.0, 92.0);
            let _ = write!(
                o,
                r#"<g transform="rotate(-6 {:.1} {:.1})">"#,
                x + w / 2.0,
                y + h / 2.0
            );
            shadowed(&mut o, (x + 8.0, y + 12.0, w, h), 3.0, WHITE);
            o.push_str("</g>");
            shadowed(&mut o, (x, y, w, h), 3.0, s.main);
            let m = h * 0.22;
            rect(&mut o, (x + m, y + m, h * 0.42, h * 0.42), h * 0.1, s.light);
            rect(
                &mut o,
                (x + m + h * 0.12, y + m + h * 0.12, h * 0.18, h * 0.18),
                2.0,
                s.main,
            );
            bar(&mut o, (x + w * 0.52, y + h * 0.28, w * 0.34, 6.0), s.light);
            bar(&mut o, (x + w * 0.52, y + h * 0.42, w * 0.22, 5.0), s.light);
            bar(&mut o, (x + w * 0.52, y + h * 0.72, w * 0.36, 5.0), s.light);
        }
        Look::Poster => {
            let (x, y, w, h) = fit(aspect, 140.0, 140.0);
            shadowed(&mut o, (x, y, w, h), 2.0, s.main);
            let cell = w * 0.32;
            let (gx, gy) = (x + w / 2.0 - cell, y + h * 0.1);
            let _ = write!(
                o,
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}"/>"#,
                gx + cell * 1.5,
                gy + cell * 0.5,
                cell / 2.0,
                s.light
            );
            rect(&mut o, (gx, gy, cell, cell), 3.0, s.light);
            let _ = write!(
                o,
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}"/>"#,
                gx + cell * 0.5,
                gy + cell * 1.5,
                cell / 2.0,
                s.motif
            );
            rect(&mut o, (gx + cell, gy + cell, cell, cell), 3.0, s.light);
            bar(&mut o, (x + w * 0.2, y + h * 0.8, w * 0.6, 5.0), s.motif);
            bar(&mut o, (x + w * 0.2, y + h * 0.88, w * 0.6, 5.0), s.motif);
        }
        Look::Invitation => {
            let (x, y, w, h) = fit(aspect, 140.0, 140.0);
            shadowed(&mut o, (x, y, w, h), 2.0, s.light);
            let (ix, iy, iw, ih) = (x + w * 0.1, y + h * 0.08, w * 0.8, h * 0.84);
            let r = iw.min(ih) * 0.16;
            let _ = write!(
                o,
                r#"<path d="M{:.1} {iy:.1} H{:.1} A{r:.1} {r:.1} 0 0 0 {:.1} {:.1} V{:.1} A{r:.1} {r:.1} 0 0 0 {:.1} {:.1} H{:.1} A{r:.1} {r:.1} 0 0 0 {ix:.1} {:.1} V{:.1} A{r:.1} {r:.1} 0 0 0 {:.1} {iy:.1} Z" fill="{}"/>"#,
                ix + r,
                ix + iw - r,
                ix + iw,
                iy + r,
                iy + ih - r,
                ix + iw - r,
                iy + ih,
                ix + r,
                iy + ih - r,
                iy + r,
                ix + r,
                s.main
            );
            bar(&mut o, (x + w * 0.3, y + h * 0.45, w * 0.4, 5.0), s.light);
            bar(&mut o, (x + w * 0.34, y + h * 0.53, w * 0.32, 5.0), s.light);
        }
        Look::Brochure => {
            let (x, y, w, h) = fit(aspect, 150.0, 110.0);
            shadowed(&mut o, (x, y, w, h), 2.0, WHITE);
            let p = w / 3.0;
            let m = p * 0.12;
            rect(&mut o, (x + m, y + m, p * 0.5, h * 0.4), 2.0, s.main);
            bar(&mut o, (x + m + p * 0.55, y + m, p * 0.3, 4.0), s.light);
            bar(
                &mut o,
                (x + m + p * 0.55, y + m + 8.0, p * 0.2, 4.0),
                s.light,
            );
            bar(&mut o, (x + m, y + h * 0.65, p * 0.7, 4.0), s.light);
            bar(&mut o, (x + m, y + h * 0.75, p * 0.6, 4.0), s.light);
            rect(
                &mut o,
                (x + p + m, y + h * 0.35, p * 0.76, h * 0.42),
                2.0,
                s.light,
            );
            rect(&mut o, (x + 2.0 * p, y, p, h), 0.0, s.main);
            bloom(
                &mut o,
                (x + 2.5 * p, y + h * 0.35),
                p * 0.3,
                s.light,
                s.motif,
            );
            bar(
                &mut o,
                (x + 2.0 * p + m, y + h * 0.72, p * 0.7, 4.0),
                s.light,
            );
            for k in 1..=2 {
                let fx = x + p * f64::from(k);
                let _ = write!(
                    o,
                    r#"<line x1="{fx:.1}" y1="{y:.1}" x2="{fx:.1}" y2="{:.1}" stroke="{SHADOW}" stroke-width="1" stroke-dasharray="3 3"/>"#,
                    y + h
                );
            }
        }
        Look::Postcard => {
            let (x, y, w, h) = fit(aspect, 140.0, 110.0);
            shadowed(&mut o, (x, y, w, h), 2.0, WHITE);
            rect(&mut o, (x, y, w * 0.4, h), 0.0, s.main);
            bloom(
                &mut o,
                (x + w * 0.2, y + h * 0.5),
                w * 0.13,
                s.light,
                s.motif,
            );
            rect(
                &mut o,
                (x + w * 0.78, y + h * 0.12, w * 0.12, w * 0.12),
                2.0,
                s.light,
            );
            for k in 0..3 {
                bar(
                    &mut o,
                    (
                        x + w * 0.52,
                        y + h * (0.55 + 0.12 * f64::from(k)),
                        w * 0.38,
                        4.0,
                    ),
                    s.light,
                );
            }
        }
        Look::Label => {
            let (x, y, w, h) = fit(aspect, 140.0, 100.0);
            shadowed(&mut o, (x, y, w, h), h.min(w) * 0.12, s.light);
            bloom(
                &mut o,
                (x + w * 0.27, y + h * 0.5),
                h.min(w) * 0.3,
                s.main,
                s.light,
            );
            bar(&mut o, (x + w * 0.52, y + h * 0.4, w * 0.36, 6.0), s.main);
            bar(&mut o, (x + w * 0.52, y + h * 0.54, w * 0.24, 5.0), s.main);
        }
        Look::Flyer => {
            let (x, y, w, h) = fit(aspect, 140.0, 140.0);
            shadowed(&mut o, (x, y, w, h), 2.0, s.main);
            rect(
                &mut o,
                (x + w * 0.1, y + h * 0.08, w * 0.8, h * 0.3),
                0.0,
                s.motif,
            );
            rect(
                &mut o,
                (x + w * 0.16, y + h * 0.13, w * 0.16, w * 0.16),
                2.0,
                s.light,
            );
            for k in 0..3 {
                bar(
                    &mut o,
                    (
                        x + w * 0.1,
                        y + h * (0.46 + 0.08 * f64::from(k)),
                        w * 0.8,
                        4.0,
                    ),
                    s.light,
                );
            }
            rect(
                &mut o,
                (x + w * 0.1, y + h * 0.74, w * 0.8, h * 0.14),
                0.0,
                s.light,
            );
        }
        Look::Infographic => {
            let (x, y, w, h) = fit(aspect, 140.0, 140.0);
            shadowed(&mut o, (x, y, w, h), 2.0, s.main);
            rect(&mut o, (x, y, w, h * 0.22), 0.0, s.light);
            let _ = write!(
                o,
                r#"<path d="M{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} Z" fill="{}"/>"#,
                x + w * 0.12,
                y + h * 0.17,
                x + w * 0.24,
                y + h * 0.05,
                x + w * 0.36,
                y + h * 0.17,
                s.main
            );
            bar(&mut o, (x + w * 0.12, y + h * 0.28, w * 0.6, 4.0), s.light);
            bar(&mut o, (x + w * 0.12, y + h * 0.34, w * 0.4, 4.0), s.light);
            chart(&mut o, (x, y + h * 0.6, w, h * 0.4), s.motif);
        }
        Look::Certificate => {
            let (x, y, w, h) = fit(aspect, 150.0, 100.0);
            shadowed(&mut o, (x, y, w, h), 2.0, WHITE);
            bloom(
                &mut o,
                (x + h * 0.32, y + h * 0.35),
                h * 0.22,
                s.main,
                s.light,
            );
            bar(&mut o, (x + h * 0.12, y + h * 0.7, w * 0.55, 5.0), s.light);
            bar(&mut o, (x + h * 0.12, y + h * 0.82, w * 0.4, 5.0), s.light);
            rect(&mut o, (x + w * 0.74, y, 0.0, h), 0.0, s.light);
            let _ = write!(
                o,
                r#"<line x1="{0:.1}" y1="{1:.1}" x2="{0:.1}" y2="{2:.1}" stroke="{3}" stroke-width="1.5" stroke-dasharray="3 3"/>"#,
                x + w * 0.74,
                y,
                y + h,
                s.main
            );
        }
        Look::Slide => {
            let (x, y, w, h) = fit(aspect, 150.0, 110.0);
            shadowed(&mut o, (x, y, w, h), 3.0, WHITE);
            bar(&mut o, (x + w * 0.08, y + h * 0.12, w * 0.3, 5.0), MIST);
            bar(&mut o, (x + w * 0.08, y + h * 0.22, w * 0.2, 5.0), MIST);
            chart(
                &mut o,
                (x + w * 0.06, y + h * 0.4, w * 0.88, h * 0.5),
                s.light,
            );
            chart(
                &mut o,
                (x + w * 0.06, y + h * 0.55, w * 0.88, h * 0.35),
                s.main,
            );
        }
        Look::Phone => {
            let (x, y, w, h) = fit(aspect, 140.0, 150.0);
            shadowed(&mut o, (x - 3.0, y - 3.0, w + 6.0, h + 6.0), 9.0, WHITE);
            let _ = write!(
                o,
                r#"<defs><linearGradient id="p" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient></defs>"#,
                s.main, s.light
            );
            rect(&mut o, (x, y, w, h), 7.0, "url(#p)");
            bloom(
                &mut o,
                (x + w / 2.0, y + h * 0.36),
                w * 0.3,
                s.motif,
                s.light,
            );
            play(&mut o, (x + w / 2.0, y + h * 0.36), w * 0.12);
            bar(&mut o, (x + w * 0.2, y + h * 0.78, w * 0.6, 4.0), CORAL);
            bar(&mut o, (x + w * 0.28, y + h * 0.85, w * 0.44, 4.0), WHITE);
            let _ = write!(
                o,
                r#"<circle cx="{:.1}" cy="{:.1}" r="3" fill="{WHITE}"/>"#,
                x + 8.0,
                y + 8.0
            );
        }
        Look::Post => {
            let (x, y, w, h) = fit(aspect, 130.0, 108.0);
            let (fx, fy, fw, fh) = (x - 6.0, y - 16.0, w + 12.0, h + 30.0);
            shadowed(&mut o, (fx, fy, fw, fh), 4.0, WHITE);
            let _ = write!(
                o,
                r#"<circle cx="{:.1}" cy="{:.1}" r="4" fill="{}"/>"#,
                fx + 9.0,
                fy + 8.0,
                s.main
            );
            bar(&mut o, (fx + 17.0, fy + 6.0, fw * 0.4, 4.0), MIST);
            rect(&mut o, (x, y, w, h), 1.0, s.main);
            bloom(
                &mut o,
                (x + w * 0.65, y + h * 0.55),
                h.min(w) * 0.34,
                s.motif,
                s.light,
            );
            bar(&mut o, (x + w * 0.08, y + h * 0.18, w * 0.3, 4.0), s.light);
            bar(&mut o, (x + w * 0.08, y + h * 0.3, w * 0.2, 4.0), s.light);
            bar(&mut o, (fx + 6.0, y + h + 5.0, fw * 0.5, 4.0), MIST);
        }
        Look::Video => {
            let (x, y, w, h) = fit(aspect, 150.0, 110.0);
            shadowed(&mut o, (x, y, w, h), 5.0, s.main);
            let r = w.min(h * 1.4) * 0.2;
            bloom(&mut o, (x + w * 0.08 + r, y + h * 0.5), r, s.light, s.motif);
            bar(&mut o, (x + w * 0.55, y + h * 0.72, w * 0.3, 4.0), s.light);
            play(&mut o, (x + w / 2.0, y + h / 2.0), h * 0.14);
        }
        Look::Web => {
            let (x, y, w, h) = fit(aspect, 150.0, 110.0);
            let (fx, fy, fw, fh) = (x, y - 10.0, w, h + 10.0);
            shadowed(&mut o, (fx, fy, fw, fh), 4.0, WHITE);
            rect(&mut o, (fx, fy, fw, 10.0), 4.0, MIST);
            for (k, c) in [CORAL, MINT, INK].iter().enumerate() {
                let _ = write!(
                    o,
                    r#"<circle cx="{:.1}" cy="{:.1}" r="2" fill="{c}"/>"#,
                    fx + 6.0 + 6.0 * k as f64,
                    fy + 5.0
                );
            }
            rect(
                &mut o,
                (x + 4.0, y + 4.0, w - 8.0, (h - 8.0) * 0.6),
                2.0,
                s.main,
            );
            bloom(
                &mut o,
                (x + w * 0.75, y + 4.0 + (h - 8.0) * 0.3),
                (h - 8.0) * 0.22,
                s.light,
                s.motif,
            );
            bar(
                &mut o,
                (x + 10.0, y + 4.0 + (h - 8.0) * 0.2, w * 0.3, 4.0),
                s.light,
            );
            bar(&mut o, (x + 4.0, y + h * 0.75, w * 0.5, 4.0), MIST);
            bar(&mut o, (x + 4.0, y + h * 0.87, w * 0.35, 4.0), MIST);
        }
        Look::Email => {
            let (x, y, w, h) = fit(aspect, 150.0, 130.0);
            shadowed(&mut o, (x, y, w, h), 3.0, WHITE);
            let band = (h * 0.35).min(50.0);
            rect(&mut o, (x, y, w, band), 3.0, s.main);
            let (ex, ey, ew, eh) = (
                x + w / 2.0 - band * 0.4,
                y + band * 0.25,
                band * 0.8,
                band * 0.5,
            );
            rect(&mut o, (ex, ey, ew, eh), 2.0, s.light);
            let _ = write!(
                o,
                r#"<path d="M{ex:.1} {ey:.1} L{:.1} {:.1} L{:.1} {ey:.1}" fill="none" stroke="{}" stroke-width="2"/>"#,
                ex + ew / 2.0,
                ey + eh * 0.6,
                ex + ew,
                s.main
            );
            let mut ly = y + band + 8.0;
            while ly + 4.0 < y + h - 4.0 {
                bar(&mut o, (x + w * 0.12, ly, w * 0.76, 4.0), MIST);
                ly += 9.0;
            }
        }
        Look::Photo => {
            let (x, y, w, h) = fit(aspect, 140.0, 130.0);
            shadowed(&mut o, (x - 4.0, y - 4.0, w + 8.0, h + 8.0), 3.0, WHITE);
            rect(&mut o, (x, y, w, h), 0.0, s.light);
            let _ = write!(
                o,
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{CORAL}"/><path d="M{x:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} Z" fill="{}"/><path d="M{:.1} {:.1} L{:.1} {:.1} L{:.1} {:.1} Z" fill="{}"/>"#,
                x + w * 0.72,
                y + h * 0.3,
                h.min(w) * 0.1,
                y + h,
                x + w * 0.35,
                y + h * 0.45,
                x + w * 0.6,
                y + h * 0.75,
                x + w,
                y + h * 0.55,
                x + w,
                y + h,
                s.main,
                x + w * 0.25,
                y + h,
                x + w * 0.55,
                y + h * 0.65,
                x + w * 0.85,
                y + h,
                s.motif
            );
        }
    }
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">{o}</svg>"#
    )
}

/// The sheet: the Gafira «hojita», a page with its corner folded, the
/// flower and two lines; on its side, the lines stand to the right.
fn sheet(o: &mut String, aspect: f64, s: Scheme) {
    let (x, y, w, h) = fit(aspect, 140.0, 132.0);
    let f = w.min(h) * 0.2;
    let path = format!(
        "M{x:.1} {y:.1} H{:.1} L{:.1} {:.1} V{:.1} H{x:.1} Z",
        x + w - f,
        x + w,
        y + f,
        y + h
    );
    let _ = write!(
        o,
        r#"<path d="{path}" fill="{SHADOW}" transform="translate(5 5)"/><path d="{path}" fill="{}"/><path d="M{:.1} {y:.1} L{:.1} {:.1} L{:.1} {:.1} Z" fill="{}"/>"#,
        s.main,
        x + w - f,
        x + w - f,
        y + f,
        x + w,
        y + f,
        s.light
    );
    if aspect <= 1.0 {
        bloom(o, (x + w / 2.0, y + h * 0.4), w * 0.34, s.motif, s.light);
        bar(o, (x + w * 0.22, y + h * 0.76, w * 0.56, 5.0), s.light);
        bar(o, (x + w * 0.3, y + h * 0.85, w * 0.4, 5.0), s.light);
    } else {
        bloom(o, (x + w * 0.38, y + h / 2.0), h * 0.36, s.motif, s.light);
        bar(o, (x + w * 0.74, y + h * 0.28, 5.0, h * 0.44), s.light);
        bar(o, (x + w * 0.84, y + h * 0.28, 5.0, h * 0.44), s.light);
    }
}

/// The SVG of the icon of a category, 24 × 24, in its colour.
pub(crate) fn category_svg(category: Category) -> String {
    let c = format!("#{:06X}", category.colour());
    let stroke = format!(
        r#"fill="none" stroke="{c}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round""#
    );
    let body = match category {
        Category::ForYou => format!(
            r#"<path d="M12 3 L13.8 9.2 L20 11 L13.8 12.8 L12 19 L10.2 12.8 L4 11 L10.2 9.2 Z" fill="{c}"/><path d="M19 3 V7 M17 5 H21" {stroke}/>"#
        ),
        Category::Social => format!(
            r#"<path d="M12 20 C5 15 3 11.5 3 8.5 C3 6 5 4 7.5 4 C9.3 4 10.9 5 12 6.6 C13.1 5 14.7 4 16.5 4 C19 4 21 6 21 8.5 C21 11.5 19 15 12 20 Z" fill="{c}"/>"#
        ),
        Category::Presentations => format!(
            r#"<rect x="3" y="4" width="18" height="12" rx="2" fill="{c}"/><path d="M12 16 V20 M8 21 L12 17 L16 21" {stroke}/><path d="M7 12 L10 9 L12 11 L16 7" fill="none" stroke="white" stroke-width="1.8" stroke-linecap="round"/>"#
        ),
        Category::Video => format!(
            r#"<rect x="2.5" y="5" width="19" height="14" rx="3" fill="{c}"/><path d="M10 9 L15 12 L10 15 Z" fill="white"/>"#
        ),
        Category::Print => format!(
            r#"<path d="M7 9 V3 H17 V9" {stroke}/><rect x="3" y="9" width="18" height="8" rx="2" fill="{c}"/><rect x="7" y="14" width="10" height="7" rx="1" fill="white" stroke="{c}" stroke-width="2"/>"#
        ),
        Category::Documents => format!(
            r#"<path d="M6 3 H14 L19 8 V21 H6 Z" fill="{c}"/><path d="M14 3 V8 H19" fill="white" fill-opacity="0.5"/><path d="M9 13 H16 M9 17 H14" stroke="white" stroke-width="1.8" stroke-linecap="round"/>"#
        ),
        Category::Photos => format!(
            r#"<path d="M3 8 C3 6.9 3.9 6 5 6 H7.5 L9 4 H15 L16.5 6 H19 C20.1 6 21 6.9 21 8 V18 C21 19.1 20.1 20 19 20 H5 C3.9 20 3 19.1 3 18 Z" fill="{c}"/><circle cx="12" cy="13" r="3.5" fill="white"/>"#
        ),
        Category::Web => format!(
            r#"<rect x="3" y="4" width="18" height="16" rx="2.5" fill="{c}"/><rect x="5" y="9" width="14" height="9" rx="1" fill="white"/><circle cx="6" cy="6.5" r="0.9" fill="white"/><circle cx="9" cy="6.5" r="0.9" fill="white"/>"#
        ),
        Category::Custom => format!(
            r#"<rect x="5" y="5" width="14" height="14" rx="2" {stroke}/><path d="M3 9 V3 H9 M15 21 H21 V15" {stroke}/>"#
        ),
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">{body}</svg>"#
    )
}

/// The rasterised pictures, by name and side in physical pixels.
#[derive(Clone, Default)]
struct Pictures(HashMap<(String, u32), Option<egui::TextureHandle>>);

/// The texture of the SVG named `name`, `side` physical pixels on its
/// longest side; made once. `None` when it does not draw.
pub(crate) fn texture(
    ctx: &egui::Context,
    name: &str,
    side: u32,
    svg: impl FnOnce() -> String,
) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("gafira.design_art");
    let key = (name.to_owned(), side);
    if let Some(found) = ctx.data(|d| {
        d.get_temp::<Pictures>(id)
            .and_then(|p| p.0.get(&key).cloned())
    }) {
        return found;
    }
    let made = gafira_io_vector::svg_thumbnail(svg().as_bytes(), side as f32)
        .ok()
        .map(|(rgba, w, h)| {
            let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            ctx.load_texture(
                format!("art/{name}/{side}"),
                image,
                egui::TextureOptions::LINEAR,
            )
        });
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Pictures>(id)
            .0
            .insert(key, made.clone());
    });
    made
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_sizes::SIZES;

    // Spec 076: every card and every icon is an SVG that draws, and the
    // sheet keeps the proportions of its size.
    #[test]
    fn every_picture_draws() {
        for size in SIZES {
            let svg = card_svg(size.look, size.aspect(), size.key);
            let (_, w, h) = gafira_io_vector::svg_thumbnail(svg.as_bytes(), 240.0)
                .unwrap_or_else(|_| panic!("{}", size.key));
            assert_eq!((w, h), (240, 170), "{}", size.key);
        }
        for category in Category::ALL {
            assert!(
                gafira_io_vector::svg_thumbnail(category_svg(category).as_bytes(), 24.0).is_ok()
            );
        }
        let tall = fit(210.0 / 297.0, 140.0, 132.0);
        assert!((tall.2 / tall.3 - 210.0 / 297.0).abs() < 1e-9);
    }
}
