use std::time::Duration;

use serde_json::{Value, json};

use super::*;

fn embedded_value() -> Value {
    serde_json::from_str(TOKENS_JSON).expect("embedded JSON")
}

/// Loads the embedded tokens after `edit` changes them.
fn edited(edit: impl FnOnce(&mut Value)) -> Result<ThemeTokens, ThemeError> {
    let mut v = embedded_value();
    edit(&mut v);
    ThemeTokens::from_json(&v.to_string())
}

fn hex(text: &str) -> Rgb8 {
    Rgb8::parse_hex(text).expect("hex")
}

fn remove(v: &mut Value, parent: &[&str], key: &str) {
    let mut node = v;
    for p in parent {
        node = &mut node[*p];
    }
    node.as_object_mut().expect("object").remove(key);
}

// S14-01
#[test]
fn s14_01_embedded_tokens_load() {
    let t = ThemeTokens::embedded().expect("embedded tokens are valid");
    assert_eq!(t.schema, 1);
    assert_eq!(t.name, "gafira");
    assert_eq!(t.brand.ink, hex("#18243B"));
    assert_eq!(t.brand.coral, hex("#FF6B57"));
    assert_eq!(t.light.get("bg"), Some(hex("#FAFAF7")));
    assert_eq!(t.dark.get("surface.raised"), Some(hex("#223250")));
    assert_eq!(t.light.len(), 45);
    assert_eq!(t.dark.len(), 45);
    let h3 = &t.typography.scale["h3"];
    assert_eq!(
        (h3.family.as_str(), h3.size, h3.line, h3.weight),
        ("Manrope", 24, 32, 650)
    );
    assert_eq!(t.typography.scale["mono-num"].features, ["tnum"]);
    assert_eq!(t.typography.ui, "Inter");
    assert_eq!(t.typography.display, "Manrope");
    assert_eq!(t.space.unit, 4);
    assert_eq!(t.space.scale.last(), Some(&64));
    assert_eq!(
        (t.radius.sm, t.radius.md, t.radius.lg, t.radius.pill),
        (8, 12, 20, 999)
    );
    assert_eq!(t.size.control.md, 32);
    assert_eq!(t.size.target_min, 24);
    assert_eq!(t.size.timeline_min, 220);
    assert_eq!(
        (t.border.width, t.border.focus, t.border.focus_offset),
        (1, 2, 2)
    );
    assert_eq!(t.motion.base, Duration::from_millis(160));
    assert_eq!(t.motion.easing["standard"], [0.2, 0.0, 0.0, 1.0]);
    assert_eq!(t.motion.reduced.fast, Duration::ZERO);
    assert_eq!(t.motion.reduced.base, Duration::ZERO);
    assert_eq!(t.motion.reduced.slow, Duration::ZERO);
    assert_eq!(t.z["tooltip"], 70);
    assert_eq!(t.shadow[&0], Shadow::None);
    assert_eq!(
        t.shadow[&2],
        Shadow::Drop {
            x: 0.0,
            y: 4.0,
            blur: 12.0,
            color: hex("#18243B"),
            alpha: 0.16
        }
    );
    assert_eq!(t.splash.panel, hex("#FAFAF7"));
    assert_eq!(t.splash.window, (800, 450));
    assert_eq!(t.splash.logo_width, 200);
}

// S14-02
#[test]
fn s14_02_schema_newer_than_supported_is_rejected() {
    let err = edited(|v| v["schema"] = json!(2)).unwrap_err();
    assert_eq!(
        err,
        ThemeError::SchemaTooNew {
            found: 2,
            supported: 1
        }
    );
}

#[test]
fn s14_02_schema_zero_is_unknown_and_a_string_is_a_bad_value() {
    assert_eq!(
        edited(|v| v["schema"] = json!(0)).unwrap_err(),
        ThemeError::SchemaUnknown { found: 0 }
    );
    assert_eq!(
        edited(|v| v["schema"] = json!("1")).unwrap_err(),
        ThemeError::BadValue {
            path: "schema".into()
        }
    );
}

#[test]
fn s14_02_text_that_is_not_json_is_a_json_error() {
    assert!(matches!(
        ThemeTokens::from_json("{ not json"),
        Err(ThemeError::Json { .. })
    ));
}

// S14-03
#[test]
fn s14_03_malformed_colours_name_their_token() {
    for bad in ["#18243", "#GG0000", "red", ""] {
        let err = edited(|v| v["modes"]["dark"]["link"] = json!(bad)).unwrap_err();
        assert_eq!(
            err,
            ThemeError::BadColor {
                path: "modes.dark.link".into(),
                value: bad.into()
            }
        );
    }
    let err = edited(|v| v["brand"]["mint"] = json!(7)).unwrap_err();
    assert_eq!(
        err,
        ThemeError::BadValue {
            path: "brand.mint".into()
        }
    );
}

// S14-04
#[test]
fn s14_04_references_resolve_through_brand_and_modes() {
    let t = ThemeTokens::embedded().expect("valid");
    assert_eq!(t.splash.panel, hex("#FAFAF7"));
    assert_eq!(t.splash.progress_track, hex("#18243B"));
    assert_eq!(t.splash.progress_fill, hex("#FF6B57"));
    // A mode token may also refer to another token, through a chain.
    let t = edited(|v| {
        v["modes"]["light"]["focus.ring"] = json!("{modes.light.text}");
        v["modes"]["light"]["text"] = json!("{brand.ink}");
    })
    .expect("valid");
    assert_eq!(t.light.get("focus.ring"), Some(hex("#18243B")));
}

#[test]
fn s14_04_unknown_reference_and_cycle_are_rejected() {
    let err = edited(|v| v["splash"]["panel"] = json!("{brand.paper}")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::UnresolvedReference {
            path: "splash.panel".into(),
            target: "brand.paper".into()
        }
    );
    let err = edited(|v| {
        v["modes"]["light"]["link"] = json!("{modes.light.accent}");
        v["modes"]["light"]["accent"] = json!("{modes.light.link}");
    })
    .unwrap_err();
    assert!(
        matches!(&err, ThemeError::ReferenceCycle { path } if path == "modes.light.accent" || path == "modes.light.link"),
        "{err:?}"
    );
    let err = edited(|v| v["brand"]["ink"] = json!("{brand.ink}")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::ReferenceCycle {
            path: "brand.ink".into()
        }
    );
}

// S14-05
#[test]
fn s14_05_a_token_missing_in_one_mode_is_a_mismatch() {
    let err = edited(|v| remove(v, &["modes", "dark"], "link")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::ModeMismatch {
            token: "link".into(),
            missing_in: "dark".into()
        }
    );
    let err = edited(|v| v["modes"]["light"]["extra"] = json!("#000000")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::ModeMismatch {
            token: "extra".into(),
            missing_in: "dark".into()
        }
    );
    let err = edited(|v| v["modes"]["dark"]["extra"] = json!("#000000")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::ModeMismatch {
            token: "extra".into(),
            missing_in: "light".into()
        }
    );
}

#[test]
fn s14_05_a_required_token_missing_in_both_modes_is_missing() {
    let err = edited(|v| {
        remove(v, &["modes", "light"], "canvas.bleed");
        remove(v, &["modes", "dark"], "canvas.bleed");
    })
    .unwrap_err();
    assert_eq!(
        err,
        ThemeError::MissingToken {
            path: "modes.light.canvas.bleed".into()
        }
    );
    let err = edited(|v| remove(v, &["type", "scale"], "body")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::MissingToken {
            path: "type.scale.body".into()
        }
    );
    let err = edited(|v| remove(v, &["size"], "target.min")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::MissingToken {
            path: "size.target.min".into()
        }
    );
}

// S14-06
#[test]
fn s14_06_every_declared_pair_meets_its_minimum() {
    let t = ThemeTokens::embedded().expect("valid");
    let checked = t.contrast_report().to_vec();
    assert_eq!(checked.len(), 84, "41 light, 41 dark and 2 splash pairs");
    let exempt = checked.iter().filter(|p| p.min.is_none()).count();
    assert_eq!(exempt, 2);
    for p in &checked {
        if let Some(min) = p.min {
            assert!(
                p.ratio >= min,
                "{}: {} on {} is {}",
                p.group,
                p.fg,
                p.bg,
                p.ratio
            );
        }
    }
}

#[test]
fn s14_06_a_pair_below_its_minimum_is_rejected() {
    let err = edited(|v| v["modes"]["light"]["text.muted"] = json!("#8E97AA")).unwrap_err();
    let ThemeError::Contrast {
        group,
        fg,
        bg,
        ratio,
        min,
    } = err
    else {
        panic!("{err:?}")
    };
    assert_eq!(
        (group.as_str(), fg.as_str(), bg.as_str()),
        ("light", "text.muted", "bg")
    );
    assert_eq!(min, 4.5);
    assert!(ratio < 4.5 && ratio > 2.5, "{ratio}");
    // A `ui` pair uses the 3:1 minimum: an almost-white border fails it.
    let err = edited(|v| v["modes"]["dark"]["border.strong"] = json!("#33456A")).unwrap_err();
    assert!(
        matches!(&err, ThemeError::Contrast { group, fg, min, .. } if group == "dark" && fg == "border.strong" && *min == 3.0),
        "{err:?}"
    );
}

#[test]
fn s14_06_unknown_usage_and_unknown_pair_token_are_rejected() {
    let err =
        edited(|v| v["contrast"]["pairs"]["light"][0] = json!(["text", "bg", "huge"])).unwrap_err();
    assert_eq!(
        err,
        ThemeError::UnknownUsage {
            group: "light".into(),
            usage: "huge".into()
        }
    );
    let err = edited(|v| v["contrast"]["pairs"]["dark"][0] = json!(["text", "paper", "text"]))
        .unwrap_err();
    assert_eq!(
        err,
        ThemeError::MissingToken {
            path: "modes.dark.paper".into()
        }
    );
}

// S14-07
#[test]
fn s14_07_rejected_pairs_stay_below_the_text_minimum() {
    let t = ThemeTokens::embedded().expect("valid");
    assert_eq!(t.rejected.len(), 6);
    let err = edited(|v| v["contrast"]["rejected"][0] = json!(["#18243B", "#FAFAF7"])).unwrap_err();
    assert_eq!(
        err,
        ThemeError::RejectedPairPasses {
            fg: "#18243B".into(),
            bg: "#FAFAF7".into()
        }
    );
}

#[test]
fn s14_07_a_declared_pair_with_rejected_colours_is_rejected() {
    // White text on coral (2.80:1) declared as an exempt pair still is a rejected pair.
    let err = edited(|v| {
        v["modes"]["light"]["danger.fg"] = json!("#FFFFFF");
        v["modes"]["light"]["danger.bg"] = json!("#FF6B57");
        v["contrast"]["pairs"]["light"][28] = json!(["danger.fg", "danger.bg", "exempt"]);
    })
    .unwrap_err();
    assert_eq!(
        err,
        ThemeError::RejectedPairDeclared {
            group: "light".into(),
            fg: "danger.fg".into(),
            bg: "danger.bg".into()
        }
    );
    // A rejection is directional: mint on warm white is rejected, the reverse is not listed.
    let declare = |fg: &'static str, bg: &'static str| {
        edited(move |v| {
            v["modes"]["light"]["danger.fg"] = json!(fg);
            v["modes"]["light"]["danger.bg"] = json!(bg);
            v["contrast"]["pairs"]["light"][28] = json!(["danger.fg", "danger.bg", "exempt"]);
            v["contrast"]["pairs"]["light"][29] = json!(["danger.fg", "danger.bg.hover", "exempt"]);
        })
    };
    assert!(matches!(
        declare("#3FBFA9", "#FAFAF7"),
        Err(ThemeError::RejectedPairDeclared { .. })
    ));
    assert!(declare("#FAFAF7", "#3FBFA9").is_ok());
}

// S14-08
#[test]
fn s14_08_reduced_motion_must_exist_and_be_zero() {
    let err = edited(|v| remove(v, &["motion"], "reduced")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::MissingToken {
            path: "motion.reduced".into()
        }
    );
    let err = edited(|v| v["motion"]["reduced"]["fast"] = json!(120)).unwrap_err();
    assert_eq!(
        err,
        ThemeError::ReducedMotionNotZero {
            name: "fast".into()
        }
    );
}

// S14-09
#[test]
fn s14_09_shadows_parse_or_are_rejected() {
    let t = edited(|v| v["shadow"]["1"] = json!("-1.5px 2px 0 rgba(0,0,0,0.45)")).expect("valid");
    assert_eq!(
        t.shadow[&1],
        Shadow::Drop {
            x: -1.5,
            y: 2.0,
            blur: 0.0,
            color: hex("#000000"),
            alpha: 0.45
        }
    );
    for bad in [
        "0 1px rgba(24,36,59,0.12)",
        "0 1px 2px #18243B",
        "0 1px 2px rgba(24,36,59,1.5)",
        "nope",
    ] {
        let err = edited(|v| v["shadow"]["3"] = json!(bad)).unwrap_err();
        assert_eq!(
            err,
            ThemeError::BadShadow {
                path: "shadow.3".into(),
                value: bad.into()
            }
        );
    }
}

// S14-10
#[test]
fn s14_10_type_families_must_be_embedded() {
    let err = edited(|v| v["type"]["family"]["ui"] = json!("Roboto")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::UnknownFamily {
            family: "Roboto".into()
        }
    );
    let err = edited(|v| v["type"]["scale"]["body"]["family"] = json!("mono")).unwrap_err();
    assert_eq!(
        err,
        ThemeError::MissingToken {
            path: "type.family.mono".into()
        }
    );
}
