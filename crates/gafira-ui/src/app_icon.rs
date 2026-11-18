//! The window icon (03 §5.3, spec 046): the 256 px application tile that
//! `cargo xtask gen-icons` writes, embedded in the binary.

use std::sync::Arc;

/// The application id: the `StartupWMClass` and icon name of the Linux
/// desktop entry (`packaging/linux/gafira.desktop`), so docks match the
/// window with its entry.
pub(crate) const APP_ID: &str = "gafira";

const PNG_256: &[u8] = include_bytes!("../../../assets/app-icon/hicolor/256x256/apps/gafira.png");

/// The decoded icon, or `None` when it cannot be decoded: the window then
/// opens with the system's default icon and the error goes to the log.
pub(crate) fn window_icon() -> Option<Arc<egui::IconData>> {
    match eframe::icon_data::from_png_bytes(PNG_256) {
        Ok(icon) => Some(Arc::new(icon)),
        Err(error) => {
            eprintln!("window icon: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec 046: the embedded window icon decodes to the 256 × 256 RGBA tile.
    #[test]
    fn window_icon_decodes_to_256_rgba() {
        let icon = window_icon().expect("the embedded icon decodes");
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
        // The centre of the top edge is the opaque brand.canvas tile.
        let i = (2 * 256 + 128) * 4;
        assert_eq!(&icon.rgba[i..i + 4], &[0xFA, 0xFA, 0xF7, 0xFF]);
    }
}
