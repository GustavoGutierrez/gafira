//! Wire format and public API seen from outside the crate.

use gafira_color::{Color, ColorSpaceId, DisplayP3Rgba, LinearRgba, Rgba};

fn from_json(json: &str) -> Result<Color, serde_json::Error> {
    serde_json::from_str(json)
}

fn channel_bits(c: Color) -> [u32; 4] {
    let [r, g, b, a] = match c {
        Color::Srgb(v) => [v.r, v.g, v.b, v.a],
        Color::LinearSrgb(v) => [v.r, v.g, v.b, v.a],
        Color::DisplayP3(v) => [v.r, v.g, v.b, v.a],
    };
    [r.to_bits(), g.to_bits(), b.to_bits(), a.to_bits()]
}

#[test]
fn color_serializes_as_one_flat_object_with_the_tag_first() {
    let json = serde_json::to_string(&Color::Srgb(Rgba::new(1.0, 0.5, 0.0, 1.0))).unwrap();
    assert_eq!(json, r#"{"space":"srgb","r":1.0,"g":0.5,"b":0.0,"a":1.0}"#);
}

#[test]
fn every_variant_uses_its_css_name_and_matches_space() {
    let colors = [
        Color::Srgb(Rgba::new(0.25, 0.5, 0.75, 1.0)),
        Color::LinearSrgb(LinearRgba::new(0.25, 0.5, 0.75, 1.0)),
        Color::DisplayP3(DisplayP3Rgba::new(0.25, 0.5, 0.75, 1.0)),
    ];
    let names = ["srgb", "srgb-linear", "display-p3"];
    for (color, name) in colors.into_iter().zip(names) {
        let json = serde_json::to_string(&color).unwrap();
        assert_eq!(
            json,
            format!(r#"{{"space":"{name}","r":0.25,"g":0.5,"b":0.75,"a":1.0}}"#)
        );
        let tag = serde_json::to_string(&color.space()).unwrap();
        assert_eq!(tag, format!("\"{name}\""));
    }
}

#[test]
fn bare_channel_type_has_no_tag() {
    assert_eq!(
        serde_json::to_string(&Rgba::new(1.0, 0.5, 0.0, 1.0)).unwrap(),
        r#"{"r":1.0,"g":0.5,"b":0.0,"a":1.0}"#
    );
    assert_eq!(
        serde_json::to_string(&LinearRgba::new(0.0, 0.5, 1.0, 0.5)).unwrap(),
        r#"{"r":0.0,"g":0.5,"b":1.0,"a":0.5}"#
    );
}

#[test]
fn color_space_id_uses_css_identifiers() {
    assert_eq!(
        serde_json::to_string(&ColorSpaceId::DisplayP3).unwrap(),
        r#""display-p3""#
    );
    assert_eq!(
        serde_json::to_string(&ColorSpaceId::LinearSrgb).unwrap(),
        r#""srgb-linear""#
    );
}

#[test]
fn unknown_case_changed_or_missing_space_is_rejected() {
    for bad in [
        r#"{"space":"rec2020","r":0.1,"g":0.2,"b":0.3,"a":1.0}"#,
        r#"{"space":"SRGB","r":0.1,"g":0.2,"b":0.3,"a":1.0}"#,
        r#"{"space":"linear-srgb","r":0.1,"g":0.2,"b":0.3,"a":1.0}"#,
        r#"{"r":0.1,"g":0.2,"b":0.3,"a":1.0}"#,
    ] {
        assert!(from_json(bad).is_err(), "{bad} must be rejected");
    }
    assert!(from_json(r#"{"space":"srgb","r":0.1,"g":0.2,"b":0.3,"a":1.0}"#).is_ok());
}

#[test]
fn missing_null_or_non_numeric_channels_are_rejected() {
    for bad in [
        r#"{"space":"srgb","r":0.1,"g":0.2,"b":0.3}"#,
        r#"{"space":"srgb","g":0.2,"b":0.3,"a":1.0}"#,
        r#"{"space":"srgb","r":null,"g":0.2,"b":0.3,"a":1.0}"#,
        r#"{"space":"srgb","r":"x","g":0.2,"b":0.3,"a":1.0}"#,
        r#"{"space":"display-p3","r":0.1,"g":0.2,"b":0.3,"a":null}"#,
    ] {
        assert!(from_json(bad).is_err(), "{bad} must be rejected");
    }
}

#[test]
fn extra_fields_are_ignored() {
    let color = from_json(r#"{"space":"srgb","r":0.1,"g":0.2,"b":0.3,"a":1.0,"x":1}"#).unwrap();
    assert_eq!(color, Color::Srgb(Rgba::new(0.1, 0.2, 0.3, 1.0)));
}

#[test]
fn a_keyed_object_does_not_parse_as_color() {
    assert!(from_json(r#"{"keys":[]}"#).is_err());
}

#[test]
fn non_finite_channels_are_written_as_null_and_cannot_be_read_back() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let colors = [
            Color::Srgb(Rgba::new(bad, 0.5, 0.5, 1.0)),
            Color::LinearSrgb(LinearRgba::new(0.5, bad, 0.5, 1.0)),
            Color::DisplayP3(DisplayP3Rgba::new(0.5, 0.5, 0.5, bad)),
        ];
        for color in colors {
            let json = serde_json::to_string(&color).unwrap();
            assert!(json.contains("null"), "{json}");
            assert!(from_json(&json).is_err(), "{json} must not decode");
        }
    }
}

#[test]
fn conservation_and_signed_zero_survive_a_round_trip() {
    let original = Color::Srgb(Rgba::new(-0.25, 1.75, 0.5, 2.0));
    let back = from_json(&serde_json::to_string(&original).unwrap()).unwrap();
    assert_eq!(channel_bits(back), channel_bits(original));

    let edge = Color::DisplayP3(DisplayP3Rgba::new(-0.0, 1e-45, f32::MAX, 1.0));
    let back = from_json(&serde_json::to_string(&edge).unwrap()).unwrap();
    assert_eq!(channel_bits(back), channel_bits(edge));
    assert!(matches!(back, Color::DisplayP3(v) if v.r.is_sign_negative()));
}
