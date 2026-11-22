//! The monitor under the mouse pointer (spec 078): where the desktop opens
//! a new window, so the splash is centred there and not on whichever
//! monitor the windowing library lists first.

/// A monitor's box in desktop pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorBox {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl MonitorBox {
    /// Whether the desktop point `(x, y)` lies on it.
    #[must_use]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let right = i64::from(self.x) + i64::from(self.width);
        let bottom = i64::from(self.y) + i64::from(self.height);
        x >= self.x && i64::from(x) < right && y >= self.y && i64::from(y) < bottom
    }

    /// Where a window of `width` × `height` goes to sit in its middle.
    #[must_use]
    pub fn centre_for(&self, width: u32, height: u32) -> (i32, i32) {
        let x = i64::from(self.x) + (i64::from(self.width) - i64::from(width)) / 2;
        let y = i64::from(self.y) + (i64::from(self.height) - i64::from(height)) / 2;
        (
            i32::try_from(x).unwrap_or(self.x),
            i32::try_from(y).unwrap_or(self.y),
        )
    }
}

/// The monitor the pointer is on, asked of the X server (also under
/// XWayland). `None` with no X display or when it cannot say.
#[must_use]
pub fn pointer_monitor() -> Option<MonitorBox> {
    #[cfg(target_os = "linux")]
    {
        x11::pointer_monitor()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
mod x11 {
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::xproto::ConnectionExt as _;

    use super::MonitorBox;

    pub(super) fn pointer_monitor() -> Option<MonitorBox> {
        std::env::var_os("DISPLAY")?;
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        let pointer = conn.query_pointer(root).ok()?.reply().ok()?;
        let monitors = conn.randr_get_monitors(root, true).ok()?.reply().ok()?;
        let (x, y) = (i32::from(pointer.root_x), i32::from(pointer.root_y));
        monitors
            .monitors
            .iter()
            .map(|m| MonitorBox {
                x: i32::from(m.x),
                y: i32::from(m.y),
                width: u32::from(m.width),
                height: u32::from(m.height),
            })
            .find(|m| m.contains(x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Spec 078: the splash goes to the middle of the pointer's monitor,
    // also on a monitor placed right of and below the first.
    #[test]
    fn a_window_is_centred_on_its_monitor() {
        let hdmi = MonitorBox {
            x: 1920,
            y: 555,
            width: 1920,
            height: 1080,
        };
        assert!(hdmi.contains(1920, 555) && hdmi.contains(3839, 1634));
        assert!(!hdmi.contains(3840, 600) && !hdmi.contains(100, 600));
        assert_eq!(hdmi.centre_for(640, 400), (1920 + 640, 555 + 340));
    }
}
