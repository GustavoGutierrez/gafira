//! Spec 104 §7.3: the `{{field}}` syntax, keys and file names.

use gafira_bulk::{FieldRef, Names, Piece, column_letter, key_for, parse, sanitize, token};

fn key(k: &str) -> Piece {
    Piece::Field(FieldRef::Key(k.to_owned()))
}

fn text(t: &str) -> Piece {
    Piece::Text(t.to_owned())
}

fn pieces(source: &str) -> Vec<Piece> {
    parse(source).into_iter().map(|(_, piece)| piece).collect()
}

// S104-10: simple fields among fixed text.
#[test]
fn s104_10_simple_fields() {
    assert_eq!(
        pieces("Hola, {{nombre}}."),
        vec![text("Hola, "), key("nombre"), text(".")]
    );
    assert_eq!(pieces("{{ curso }}"), vec![key("curso")]);
    assert_eq!(pieces("{{a}}{{b}}"), vec![key("a"), key("b")]);
    assert_eq!(pieces("sin campos"), vec![text("sin campos")]);
}

// S104-11: a field by its original title, with JSON escapes (rule 6).
#[test]
fn s104_11_titles_in_brackets() {
    assert_eq!(
        pieces(r#"{{["Nombre completo"]}}"#),
        vec![Piece::Field(FieldRef::Name("Nombre completo".to_owned()))]
    );
    assert_eq!(
        pieces(r#"{{["Dice \"hola\""]}}"#),
        vec![Piece::Field(FieldRef::Name("Dice \"hola\"".to_owned()))]
    );
    assert_eq!(token(&FieldRef::Name("Año".into())), r#"{{["Año"]}}"#);
    assert_eq!(token(&FieldRef::Key("nombre".into())), "{{nombre}}");
}

// S104-12: what is not a field stays as it is (rules 5 and 7).
#[test]
fn s104_12_literals_and_escapes() {
    assert_eq!(pieces("{nombre}"), vec![text("{nombre}")]);
    assert_eq!(pieces("{{9x}}"), vec![text("{{9x}}")]);
    assert_eq!(pieces("{{a b}}"), vec![text("{{a b}}")]);
    assert_eq!(pieces("{{abierto"), vec![text("{{abierto")]);
    assert_eq!(pieces(r"\{{nombre}}"), vec![text("{{"), text("nombre}}")]);
    assert_eq!(pieces(r"a\\b"), vec![text("a"), text("\\"), text("b")]);
    assert_eq!(pieces(r"C:\ruta"), vec![text(r"C:\ruta")]);
    assert_eq!(pieces("{{{a}}}"), vec![text("{"), key("a"), text("}")]);
}

// S104-13: each piece knows the bytes of the source it came from.
#[test]
fn s104_13_ranges_cover_the_source() {
    let source = r"Hola {{n}} \{{ y ñ";
    let parsed = parse(source);
    let mut at = 0;
    for (range, _) in &parsed {
        assert_eq!(range.start, at);
        at = range.end;
    }
    assert_eq!(at, source.len());
}

// S104-14: keys from titles: ASCII, unique, never starting with a digit.
#[test]
fn s104_14_keys_from_titles() {
    let mut taken = std::collections::BTreeSet::new();
    let mut add = |name: &str| {
        let k = key_for(name, &taken);
        taken.insert(k.clone());
        k
    };
    assert_eq!(add("Nombre completo"), "nombre_completo");
    assert_eq!(add("Año de ingreso"), "ano_de_ingreso");
    assert_eq!(add("Nombre completo"), "nombre_completo_2");
    assert_eq!(add("2024"), "c_2024");
    assert_eq!(add("¿?"), "col");
    assert_eq!(add("Ñandú"), "nandu");
    for k in &taken {
        assert!(gafira_doc::DataSet::is_key(k), "{k}");
    }
}

// S104-15: the letters of the columns, as in a spreadsheet.
#[test]
fn s104_15_column_letters() {
    assert_eq!(column_letter(0), "A");
    assert_eq!(column_letter(25), "Z");
    assert_eq!(column_letter(26), "AA");
    assert_eq!(column_letter(27), "AB");
    assert_eq!(column_letter(701), "ZZ");
    assert_eq!(column_letter(702), "AAA");
}

// S104-16: names of files safe on every system (§10.2).
#[test]
fn s104_16_safe_file_names() {
    assert_eq!(sanitize("certificado_Ana Pérez"), "certificado_Ana Pérez");
    assert_eq!(sanitize("a/b\\c:d*e?f\"g<h>i|j"), "a_b_c_d_e_f_g_h_i_j");
    assert_eq!(sanitize("  fin. . "), "fin");
    assert_eq!(sanitize("CON"), "_CON");
    assert_eq!(sanitize("lpt1"), "_lpt1");
    assert_eq!(sanitize("línea\nnueva\t"), "línea_nueva");
    assert_eq!(sanitize("..."), "");
    assert_eq!(sanitize(&"ñ".repeat(300)).chars().count(), 120);
}

// S104-17: equal names get a stable index and never take a used one (AC 10).
#[test]
fn s104_17_collisions() {
    let mut names = Names::default();
    names.take("ana.png");
    assert_eq!(names.unique("ana", "png"), "ana-2.png");
    assert_eq!(names.unique("Ana", "png"), "Ana-3.png");
    assert_eq!(names.unique("luis", "png"), "luis.png");
    assert_eq!(names.unique("", "png"), "_.png");
}
