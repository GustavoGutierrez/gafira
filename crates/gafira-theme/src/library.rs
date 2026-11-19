//! The font library (spec 062, D-100): twelve OFL families, each in its
//! regular face and, when the family has them, bold, italic and bold
//! italic, compiled into the binary. Their licences and origins are in
//! `assets/LICENSES.toml`.

/// One face of a family of the library.
#[derive(Clone, Copy, Debug)]
pub struct LibraryFace {
    pub family: &'static str,
    pub bold: bool,
    pub italic: bool,
    /// The repository path of the file, for the licence register.
    pub path: &'static str,
    pub bytes: &'static [u8],
}

const FACES: [LibraryFace; 40] = [
    LibraryFace {
        family: "Open Sans",
        bold: false,
        italic: false,
        path: "assets/fonts/library/opensans/OpenSans-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Regular.ttf"),
    },
    LibraryFace {
        family: "Open Sans",
        bold: true,
        italic: false,
        path: "assets/fonts/library/opensans/OpenSans-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Bold.ttf"),
    },
    LibraryFace {
        family: "Open Sans",
        bold: false,
        italic: true,
        path: "assets/fonts/library/opensans/OpenSans-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/opensans/OpenSans-Italic.ttf"),
    },
    LibraryFace {
        family: "Open Sans",
        bold: true,
        italic: true,
        path: "assets/fonts/library/opensans/OpenSans-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/opensans/OpenSans-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "Montserrat",
        bold: false,
        italic: false,
        path: "assets/fonts/library/montserrat/Montserrat-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/montserrat/Montserrat-Regular.ttf"),
    },
    LibraryFace {
        family: "Montserrat",
        bold: true,
        italic: false,
        path: "assets/fonts/library/montserrat/Montserrat-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/montserrat/Montserrat-Bold.ttf"),
    },
    LibraryFace {
        family: "Montserrat",
        bold: false,
        italic: true,
        path: "assets/fonts/library/montserrat/Montserrat-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/montserrat/Montserrat-Italic.ttf"),
    },
    LibraryFace {
        family: "Montserrat",
        bold: true,
        italic: true,
        path: "assets/fonts/library/montserrat/Montserrat-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/montserrat/Montserrat-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "Poppins",
        bold: false,
        italic: false,
        path: "assets/fonts/library/poppins/Poppins-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/poppins/Poppins-Regular.ttf"),
    },
    LibraryFace {
        family: "Poppins",
        bold: true,
        italic: false,
        path: "assets/fonts/library/poppins/Poppins-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/poppins/Poppins-Bold.ttf"),
    },
    LibraryFace {
        family: "Poppins",
        bold: false,
        italic: true,
        path: "assets/fonts/library/poppins/Poppins-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/poppins/Poppins-Italic.ttf"),
    },
    LibraryFace {
        family: "Poppins",
        bold: true,
        italic: true,
        path: "assets/fonts/library/poppins/Poppins-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/poppins/Poppins-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "DM Sans",
        bold: false,
        italic: false,
        path: "assets/fonts/library/dmsans/DMSans-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/dmsans/DMSans-Regular.ttf"),
    },
    LibraryFace {
        family: "DM Sans",
        bold: true,
        italic: false,
        path: "assets/fonts/library/dmsans/DMSans-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/dmsans/DMSans-Bold.ttf"),
    },
    LibraryFace {
        family: "DM Sans",
        bold: false,
        italic: true,
        path: "assets/fonts/library/dmsans/DMSans-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/dmsans/DMSans-Italic.ttf"),
    },
    LibraryFace {
        family: "DM Sans",
        bold: true,
        italic: true,
        path: "assets/fonts/library/dmsans/DMSans-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/dmsans/DMSans-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "Roboto",
        bold: false,
        italic: false,
        path: "assets/fonts/library/roboto/Roboto-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/roboto/Roboto-Regular.ttf"),
    },
    LibraryFace {
        family: "Roboto",
        bold: true,
        italic: false,
        path: "assets/fonts/library/roboto/Roboto-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/roboto/Roboto-Bold.ttf"),
    },
    LibraryFace {
        family: "Roboto",
        bold: false,
        italic: true,
        path: "assets/fonts/library/roboto/Roboto-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/roboto/Roboto-Italic.ttf"),
    },
    LibraryFace {
        family: "Roboto",
        bold: true,
        italic: true,
        path: "assets/fonts/library/roboto/Roboto-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/roboto/Roboto-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "Lato",
        bold: false,
        italic: false,
        path: "assets/fonts/library/lato/Lato-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/lato/Lato-Regular.ttf"),
    },
    LibraryFace {
        family: "Lato",
        bold: true,
        italic: false,
        path: "assets/fonts/library/lato/Lato-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/lato/Lato-Bold.ttf"),
    },
    LibraryFace {
        family: "Lato",
        bold: false,
        italic: true,
        path: "assets/fonts/library/lato/Lato-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/lato/Lato-Italic.ttf"),
    },
    LibraryFace {
        family: "Lato",
        bold: true,
        italic: true,
        path: "assets/fonts/library/lato/Lato-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/lato/Lato-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "Inter",
        bold: false,
        italic: false,
        path: "assets/fonts/library/inter/Inter-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/inter/Inter-Regular.ttf"),
    },
    LibraryFace {
        family: "Inter",
        bold: true,
        italic: false,
        path: "assets/fonts/library/inter/Inter-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/inter/Inter-Bold.ttf"),
    },
    LibraryFace {
        family: "Inter",
        bold: false,
        italic: true,
        path: "assets/fonts/library/inter/Inter-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/inter/Inter-Italic.ttf"),
    },
    LibraryFace {
        family: "Inter",
        bold: true,
        italic: true,
        path: "assets/fonts/library/inter/Inter-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/inter/Inter-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "Manrope",
        bold: false,
        italic: false,
        path: "assets/fonts/library/manrope/Manrope-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/manrope/Manrope-Regular.ttf"),
    },
    LibraryFace {
        family: "Manrope",
        bold: true,
        italic: false,
        path: "assets/fonts/library/manrope/Manrope-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/manrope/Manrope-Bold.ttf"),
    },
    LibraryFace {
        family: "EB Garamond",
        bold: false,
        italic: false,
        path: "assets/fonts/library/ebgaramond/EBGaramond-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/ebgaramond/EBGaramond-Regular.ttf"),
    },
    LibraryFace {
        family: "EB Garamond",
        bold: true,
        italic: false,
        path: "assets/fonts/library/ebgaramond/EBGaramond-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/ebgaramond/EBGaramond-Bold.ttf"),
    },
    LibraryFace {
        family: "EB Garamond",
        bold: false,
        italic: true,
        path: "assets/fonts/library/ebgaramond/EBGaramond-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/ebgaramond/EBGaramond-Italic.ttf"),
    },
    LibraryFace {
        family: "EB Garamond",
        bold: true,
        italic: true,
        path: "assets/fonts/library/ebgaramond/EBGaramond-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/ebgaramond/EBGaramond-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "Source Serif 4",
        bold: false,
        italic: false,
        path: "assets/fonts/library/sourceserif4/SourceSerif4-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/sourceserif4/SourceSerif4-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Source Serif 4",
        bold: true,
        italic: false,
        path: "assets/fonts/library/sourceserif4/SourceSerif4-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/sourceserif4/SourceSerif4-Bold.ttf"),
    },
    LibraryFace {
        family: "Source Serif 4",
        bold: false,
        italic: true,
        path: "assets/fonts/library/sourceserif4/SourceSerif4-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/sourceserif4/SourceSerif4-Italic.ttf"),
    },
    LibraryFace {
        family: "Source Serif 4",
        bold: true,
        italic: true,
        path: "assets/fonts/library/sourceserif4/SourceSerif4-BoldItalic.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/sourceserif4/SourceSerif4-BoldItalic.ttf"
        ),
    },
    LibraryFace {
        family: "Anton",
        bold: false,
        italic: false,
        path: "assets/fonts/library/anton/Anton-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/anton/Anton-Regular.ttf"),
    },
    LibraryFace {
        family: "Pacifico",
        bold: false,
        italic: false,
        path: "assets/fonts/library/pacifico/Pacifico-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/pacifico/Pacifico-Regular.ttf"),
    },
];

/// Every face of the library, family by family, regular first.
#[must_use]
pub fn library() -> &'static [LibraryFace] {
    &FACES
}

/// The families of the library, in their order.
#[must_use]
pub fn library_families() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for face in &FACES {
        if !out.contains(&face.family) {
            out.push(face.family);
        }
    }
    out
}

pub use gafira_doc::{face_key, nearest_face};

#[cfg(test)]
mod tests {
    use super::*;

    // S62-02: twelve families, each with a regular face; every file is the
    // one on disk; the registered names follow the faces.
    #[test]
    fn s62_02_library_faces() {
        let families = library_families();
        assert_eq!(families.len(), 12);
        assert_eq!(families[0], "Open Sans");
        for family in &families {
            assert!(
                library()
                    .iter()
                    .any(|f| f.family == *family && !f.bold && !f.italic),
                "{family}"
            );
        }
        assert_eq!(library().len(), 40, "every face of the library");
        assert_eq!(face_key("Lato", false, false), "Lato");
        assert_eq!(face_key("Lato", true, false), "Lato Bold");
        assert_eq!(face_key("Lato", false, true), "Lato Italic");
        assert_eq!(face_key("Lato", true, true), "Lato Bold Italic");
    }

    // S62-02: a face the family lacks falls to the nearest one: no italic
    // first, then no bold.
    #[test]
    fn s62_02_nearest_face() {
        let manrope = |_bold: bool, italic: bool| !italic;
        assert_eq!(nearest_face(manrope, true, true), Some((true, false)));
        assert_eq!(nearest_face(manrope, false, true), Some((false, false)));
        let anton = |bold: bool, italic: bool| !bold && !italic;
        assert_eq!(nearest_face(anton, true, true), Some((false, false)));
        assert_eq!(nearest_face(|_, _| true, true, true), Some((true, true)));
        assert_eq!(nearest_face(|_, _| false, false, false), None);
    }
}
