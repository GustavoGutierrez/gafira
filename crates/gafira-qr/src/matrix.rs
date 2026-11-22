//! The dark and light modules of a code.

use gafira_doc::QrEcc;

use crate::QrError;

/// The modules of a code, without its margin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Matrix {
    size: usize,
    dark: Vec<bool>,
}

impl Matrix {
    /// The smallest code of `text`, at least of version `least`.
    pub fn encode(text: &str, ecc: QrEcc, least: i16) -> Result<Self, QrError> {
        use qrcode::{EcLevel, QrCode, Version};
        let level = match ecc {
            QrEcc::L => EcLevel::L,
            QrEcc::M => EcLevel::M,
            QrEcc::Q => EcLevel::Q,
            QrEcc::H => EcLevel::H,
        };
        let made = QrCode::with_error_correction_level(text.as_bytes(), level)
            .map_err(|_| QrError::TooLong)?;
        let made = match made.version() {
            Version::Normal(v) if v < least => {
                QrCode::with_version(text.as_bytes(), Version::Normal(least), level)
                    .map_err(|_| QrError::TooLong)?
            }
            _ => made,
        };
        let size = made.width();
        let dark = made
            .to_colors()
            .into_iter()
            .map(|c| c == qrcode::Color::Dark)
            .collect();
        Ok(Self { size, dark })
    }

    /// A pattern of 5 × 5 modules with every kind of neighbourhood: runs,
    /// corners, a hole and modules on their own.
    pub(crate) fn swatch() -> Self {
        let rows = ["11010", "10011", "11101", "10110", "00101"];
        let dark = rows
            .iter()
            .flat_map(|r| r.bytes())
            .map(|b| b == b'1')
            .collect();
        Self { size: 5, dark }
    }

    /// The modules of a side.
    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Whether the module is dark; `false` outside the code.
    #[must_use]
    pub fn dark(&self, x: i32, y: i32) -> bool {
        match (usize::try_from(x), usize::try_from(y)) {
            (Ok(x), Ok(y)) if x < self.size && y < self.size => self.dark[y * self.size + x],
            _ => false,
        }
    }

    /// Whether the module is part of one of the three eyes.
    #[must_use]
    pub fn in_eye(&self, x: i32, y: i32) -> bool {
        // At most 177: the cast is exact.
        let far = self.size as i32 - 7;
        (x < 7 && (y < 7 || y >= far)) || (x >= far && y < 7)
    }
}
