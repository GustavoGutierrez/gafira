//! The font library (spec 062, D-100): forty-eight free families, each in its
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

const FACES: [LibraryFace; 87] = [
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
    LibraryFace {
        family: "Arimo",
        bold: false,
        italic: false,
        path: "assets/fonts/library/arimo/Arimo-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/arimo/Arimo-Regular.ttf"),
    },
    LibraryFace {
        family: "Arimo",
        bold: true,
        italic: false,
        path: "assets/fonts/library/arimo/Arimo-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/arimo/Arimo-Bold.ttf"),
    },
    LibraryFace {
        family: "Arimo",
        bold: false,
        italic: true,
        path: "assets/fonts/library/arimo/Arimo-Italic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/arimo/Arimo-Italic.ttf"),
    },
    LibraryFace {
        family: "Arimo",
        bold: true,
        italic: true,
        path: "assets/fonts/library/arimo/Arimo-BoldItalic.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/arimo/Arimo-BoldItalic.ttf"),
    },
    LibraryFace {
        family: "League Spartan",
        bold: false,
        italic: false,
        path: "assets/fonts/library/leaguespartan/LeagueSpartan-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/leaguespartan/LeagueSpartan-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "League Spartan",
        bold: true,
        italic: false,
        path: "assets/fonts/library/leaguespartan/LeagueSpartan-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/leaguespartan/LeagueSpartan-Bold.ttf"),
    },
    LibraryFace {
        family: "Archivo Black",
        bold: false,
        italic: false,
        path: "assets/fonts/library/archivoblack/ArchivoBlack-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/archivoblack/ArchivoBlack-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Questrial",
        bold: false,
        italic: false,
        path: "assets/fonts/library/questrial/Questrial-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/questrial/Questrial-Regular.ttf"),
    },
    LibraryFace {
        family: "Bebas Neue",
        bold: false,
        italic: false,
        path: "assets/fonts/library/bebasneue/BebasNeue-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/bebasneue/BebasNeue-Regular.ttf"),
    },
    LibraryFace {
        family: "Alice",
        bold: false,
        italic: false,
        path: "assets/fonts/library/alice/Alice-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/alice/Alice-Regular.ttf"),
    },
    LibraryFace {
        family: "Bree Serif",
        bold: false,
        italic: false,
        path: "assets/fonts/library/breeserif/BreeSerif-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/breeserif/BreeSerif-Regular.ttf"),
    },
    LibraryFace {
        family: "Shrikhand",
        bold: false,
        italic: false,
        path: "assets/fonts/library/shrikhand/Shrikhand-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/shrikhand/Shrikhand-Regular.ttf"),
    },
    LibraryFace {
        family: "Chau Philomene One",
        bold: false,
        italic: false,
        path: "assets/fonts/library/chauphilomeneone/ChauPhilomeneOne-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/chauphilomeneone/ChauPhilomeneOne-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Chau Philomene One",
        bold: false,
        italic: true,
        path: "assets/fonts/library/chauphilomeneone/ChauPhilomeneOne-Italic.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/chauphilomeneone/ChauPhilomeneOne-Italic.ttf"
        ),
    },
    LibraryFace {
        family: "Belleza",
        bold: false,
        italic: false,
        path: "assets/fonts/library/belleza/Belleza-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/belleza/Belleza-Regular.ttf"),
    },
    LibraryFace {
        family: "Cormorant Garamond",
        bold: false,
        italic: false,
        path: "assets/fonts/library/cormorantgaramond/CormorantGaramond-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/cormorantgaramond/CormorantGaramond-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Cormorant Garamond",
        bold: true,
        italic: false,
        path: "assets/fonts/library/cormorantgaramond/CormorantGaramond-Bold.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/cormorantgaramond/CormorantGaramond-Bold.ttf"
        ),
    },
    LibraryFace {
        family: "Cormorant Garamond",
        bold: false,
        italic: true,
        path: "assets/fonts/library/cormorantgaramond/CormorantGaramond-Italic.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/cormorantgaramond/CormorantGaramond-Italic.ttf"
        ),
    },
    LibraryFace {
        family: "Cormorant Garamond",
        bold: true,
        italic: true,
        path: "assets/fonts/library/cormorantgaramond/CormorantGaramond-BoldItalic.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/cormorantgaramond/CormorantGaramond-BoldItalic.ttf"
        ),
    },
    LibraryFace {
        family: "Chewy",
        bold: false,
        italic: false,
        path: "assets/fonts/library/chewy/Chewy-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/chewy/Chewy-Regular.ttf"),
    },
    LibraryFace {
        family: "DM Serif Display",
        bold: false,
        italic: false,
        path: "assets/fonts/library/dmserifdisplay/DMSerifDisplay-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/dmserifdisplay/DMSerifDisplay-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "DM Serif Display",
        bold: false,
        italic: true,
        path: "assets/fonts/library/dmserifdisplay/DMSerifDisplay-Italic.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/dmserifdisplay/DMSerifDisplay-Italic.ttf"
        ),
    },
    LibraryFace {
        family: "Kaushan Script",
        bold: false,
        italic: false,
        path: "assets/fonts/library/kaushanscript/KaushanScript-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/kaushanscript/KaushanScript-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Satisfy",
        bold: false,
        italic: false,
        path: "assets/fonts/library/satisfy/Satisfy-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/satisfy/Satisfy-Regular.ttf"),
    },
    LibraryFace {
        family: "Alex Brush",
        bold: false,
        italic: false,
        path: "assets/fonts/library/alexbrush/AlexBrush-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/alexbrush/AlexBrush-Regular.ttf"),
    },
    LibraryFace {
        family: "Dancing Script",
        bold: false,
        italic: false,
        path: "assets/fonts/library/dancingscript/DancingScript-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/dancingscript/DancingScript-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Allura",
        bold: false,
        italic: false,
        path: "assets/fonts/library/allura/Allura-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/allura/Allura-Regular.ttf"),
    },
    LibraryFace {
        family: "Great Vibes",
        bold: false,
        italic: false,
        path: "assets/fonts/library/greatvibes/GreatVibes-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/greatvibes/GreatVibes-Regular.ttf"),
    },
    LibraryFace {
        family: "Merienda",
        bold: false,
        italic: false,
        path: "assets/fonts/library/merienda/Merienda-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/merienda/Merienda-Regular.ttf"),
    },
    LibraryFace {
        family: "Courgette",
        bold: false,
        italic: false,
        path: "assets/fonts/library/courgette/Courgette-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/courgette/Courgette-Regular.ttf"),
    },
    LibraryFace {
        family: "Londrina Outline",
        bold: false,
        italic: false,
        path: "assets/fonts/library/londrinaoutline/LondrinaOutline-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/londrinaoutline/LondrinaOutline-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Rubik Bubbles",
        bold: false,
        italic: false,
        path: "assets/fonts/library/rubikbubbles/RubikBubbles-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/rubikbubbles/RubikBubbles-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Sniglet",
        bold: false,
        italic: false,
        path: "assets/fonts/library/sniglet/Sniglet-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/sniglet/Sniglet-Regular.ttf"),
    },
    LibraryFace {
        family: "Sniglet",
        bold: true,
        italic: false,
        path: "assets/fonts/library/sniglet/Sniglet-ExtraBold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/sniglet/Sniglet-ExtraBold.ttf"),
    },
    LibraryFace {
        family: "Cabin Sketch",
        bold: false,
        italic: false,
        path: "assets/fonts/library/cabinsketch/CabinSketch-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/cabinsketch/CabinSketch-Regular.ttf"),
    },
    LibraryFace {
        family: "Cabin Sketch",
        bold: true,
        italic: false,
        path: "assets/fonts/library/cabinsketch/CabinSketch-Bold.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/cabinsketch/CabinSketch-Bold.ttf"),
    },
    LibraryFace {
        family: "Michroma",
        bold: false,
        italic: false,
        path: "assets/fonts/library/michroma/Michroma-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/michroma/Michroma-Regular.ttf"),
    },
    LibraryFace {
        family: "Rye",
        bold: false,
        italic: false,
        path: "assets/fonts/library/rye/Rye-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/rye/Rye-Regular.ttf"),
    },
    LibraryFace {
        family: "Mr Dafoe",
        bold: false,
        italic: false,
        path: "assets/fonts/library/mrdafoe/MrDafoe-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/mrdafoe/MrDafoe-Regular.ttf"),
    },
    LibraryFace {
        family: "Damion",
        bold: false,
        italic: false,
        path: "assets/fonts/library/damion/Damion-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/damion/Damion-Regular.ttf"),
    },
    LibraryFace {
        family: "Mrs Saint Delafield",
        bold: false,
        italic: false,
        path: "assets/fonts/library/mrssaintdelafield/MrsSaintDelafield-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/mrssaintdelafield/MrsSaintDelafield-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Herr Von Muellerhoff",
        bold: false,
        italic: false,
        path: "assets/fonts/library/herrvonmuellerhoff/HerrVonMuellerhoff-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/herrvonmuellerhoff/HerrVonMuellerhoff-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Monsieur La Doulaise",
        bold: false,
        italic: false,
        path: "assets/fonts/library/monsieurladoulaise/MonsieurLaDoulaise-Regular.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/monsieurladoulaise/MonsieurLaDoulaise-Regular.ttf"
        ),
    },
    LibraryFace {
        family: "Meddon",
        bold: false,
        italic: false,
        path: "assets/fonts/library/meddon/Meddon.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/meddon/Meddon.ttf"),
    },
    LibraryFace {
        family: "UnifrakturMaguntia",
        bold: false,
        italic: false,
        path: "assets/fonts/library/unifrakturmaguntia/UnifrakturMaguntia-Book.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/unifrakturmaguntia/UnifrakturMaguntia-Book.ttf"
        ),
    },
    LibraryFace {
        family: "UnifrakturCook",
        bold: false,
        italic: false,
        path: "assets/fonts/library/unifrakturcook/UnifrakturCook-Bold.ttf",
        bytes: include_bytes!(
            "../../../assets/fonts/library/unifrakturcook/UnifrakturCook-Bold.ttf"
        ),
    },
    LibraryFace {
        family: "Pirata One",
        bold: false,
        italic: false,
        path: "assets/fonts/library/pirataone/PirataOne-Regular.ttf",
        bytes: include_bytes!("../../../assets/fonts/library/pirataone/PirataOne-Regular.ttf"),
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
        assert_eq!(families.len(), 48);
        assert_eq!(families[0], "Open Sans");
        for family in &families {
            assert!(
                library()
                    .iter()
                    .any(|f| f.family == *family && !f.bold && !f.italic),
                "{family}"
            );
        }
        assert_eq!(library().len(), 87, "every face of the library");
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
