//! QR codes (spec 080, FD-20): what a code says and how it looks, as the
//! person wrote it. The text it encodes and its outlines are worked out by
//! `gafira-qr`, so editing the code makes it again.

use gafira_color::Color;
use serde::{Deserialize, Serialize};

use crate::Gradient;
use crate::timeline::AssetRef;

/// How much of the code can be lost and still be read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrEcc {
    /// About 7 %.
    L,
    /// About 15 %.
    #[default]
    M,
    /// About 25 %.
    Q,
    /// About 30 %.
    H,
}

/// How a Wi-Fi network is protected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrWifiSecurity {
    Open,
    Wep,
    #[default]
    Wpa,
}

/// The format a contact is written in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrContactFormat {
    #[default]
    Vcard,
    Mecard,
}

/// The coin of a payment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrCoin {
    #[default]
    Bitcoin,
    BitcoinCash,
    Ethereum,
    Litecoin,
    Dash,
}

/// The fields of a contact; an empty one is left out.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct QrContact {
    pub format: QrContactFormat,
    pub first_name: String,
    pub last_name: String,
    pub organization: String,
    pub job: String,
    pub phone: String,
    pub work_phone: String,
    pub email: String,
    pub url: String,
    pub street: String,
    pub city: String,
    pub region: String,
    pub postcode: String,
    pub country: String,
}

/// What a code says, by kind, as it was written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QrContent {
    Url {
        url: String,
    },
    Text {
        text: String,
    },
    Email {
        address: String,
        subject: String,
        body: String,
    },
    Phone {
        number: String,
    },
    Sms {
        number: String,
        text: String,
    },
    Wifi {
        ssid: String,
        password: String,
        security: QrWifiSecurity,
        hidden: bool,
    },
    Contact(Box<QrContact>),
    /// `start` and `end` are local times, `YYYY-MM-DD HH:MM`.
    Event {
        title: String,
        place: String,
        start: String,
        end: String,
    },
    Location {
        latitude: String,
        longitude: String,
    },
    Social {
        url: String,
    },
    Crypto {
        coin: QrCoin,
        address: String,
        amount: String,
    },
}

impl QrContent {
    /// The words of the content, in order.
    pub fn fields(&self) -> Vec<&str> {
        match self {
            Self::Url { url } | Self::Social { url } => vec![url],
            Self::Text { text } => vec![text],
            Self::Email {
                address,
                subject,
                body,
            } => vec![address, subject, body],
            Self::Phone { number } => vec![number],
            Self::Sms { number, text } => vec![number, text],
            Self::Wifi { ssid, password, .. } => vec![ssid, password],
            Self::Contact(c) => vec![
                &c.first_name,
                &c.last_name,
                &c.organization,
                &c.job,
                &c.phone,
                &c.work_phone,
                &c.email,
                &c.url,
                &c.street,
                &c.city,
                &c.region,
                &c.postcode,
                &c.country,
            ],
            Self::Event {
                title,
                place,
                start,
                end,
            } => vec![title, place, start, end],
            Self::Location {
                latitude,
                longitude,
            } => vec![latitude, longitude],
            Self::Crypto {
                address, amount, ..
            } => vec![address, amount],
        }
    }
}

/// The shape of the modules of the body.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrBody {
    #[default]
    Square,
    Separate,
    Dot,
    SmallDot,
    Diamond,
    Star,
    Cross,
    Fluid,
    FluidCorners,
    BarsHorizontal,
    BarsVertical,
    Bevel,
    Leaf,
}

impl QrBody {
    pub const ALL: [Self; 13] = [
        Self::Square,
        Self::Separate,
        Self::Dot,
        Self::SmallDot,
        Self::Diamond,
        Self::Star,
        Self::Cross,
        Self::Fluid,
        Self::FluidCorners,
        Self::BarsHorizontal,
        Self::BarsVertical,
        Self::Bevel,
        Self::Leaf,
    ];
}

/// The shape of the ring of an eye.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrEyeFrame {
    #[default]
    Square,
    Rounded,
    Circle,
    Leaf,
    Drop,
    Bevel,
    Dotted,
    Octagon,
}

impl QrEyeFrame {
    pub const ALL: [Self; 8] = [
        Self::Square,
        Self::Rounded,
        Self::Circle,
        Self::Leaf,
        Self::Drop,
        Self::Bevel,
        Self::Dotted,
        Self::Octagon,
    ];
}

/// The shape of the centre of an eye.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrEyeBall {
    #[default]
    Square,
    Rounded,
    Circle,
    Leaf,
    Diamond,
    Bars,
    Cushion,
    Flower,
}

impl QrEyeBall {
    pub const ALL: [Self; 8] = [
        Self::Square,
        Self::Rounded,
        Self::Circle,
        Self::Leaf,
        Self::Diamond,
        Self::Bars,
        Self::Cushion,
        Self::Flower,
    ];
}

/// A colour or a gradient.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QrFill {
    Solid(Color),
    Gradient(Gradient),
}

impl QrFill {
    /// Every colour it shows.
    pub fn colors(&self) -> Vec<Color> {
        match self {
            Self::Solid(color) => vec![*color],
            Self::Gradient(gradient) => gradient.stops.iter().map(|s| s.color).collect(),
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Solid(color) => color_is_valid(*color),
            Self::Gradient(gradient) => gradient.problem().is_none(),
        }
    }
}

fn color_is_valid(color: Color) -> bool {
    color.is_finite() && (0.0..=1.0).contains(&color.alpha())
}

/// How a code looks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QrStyle {
    pub body: QrBody,
    pub eye_frame: QrEyeFrame,
    pub eye_ball: QrEyeBall,
    /// The shapes with a direction face the middle of the code.
    pub mirror_eyes: bool,
    /// The colour of the body, spread over the whole code.
    pub fill: QrFill,
    /// The colour of the rings of the eyes; `None` takes `fill`.
    pub eye_frame_fill: Option<QrFill>,
    /// The colour of the centres of the eyes; `None` takes `fill`.
    pub eye_ball_fill: Option<QrFill>,
    /// `None` is transparent.
    pub background: Option<Color>,
}

/// An image in the middle of a code.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct QrLogo {
    pub asset: AssetRef,
    /// Its size in px.
    pub natural: (u32, u32),
    /// Its longer side, as a share of the code without its margin.
    pub share: f64,
    /// Whether the modules behind it are left out.
    pub clear_behind: bool,
}

impl QrLogo {
    pub const MIN_SHARE: f64 = 0.1;
    pub const MAX_SHARE: f64 = 0.3;
}

/// A QR code (spec 080, FD-20).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QrNode {
    pub content: QrContent,
    /// The correction without a logo; see [`QrNode::effective_ecc`].
    pub ecc: QrEcc,
    /// The empty modules around the code.
    pub quiet_zone: u8,
    pub style: QrStyle,
    pub logo: Option<QrLogo>,
}

impl QrNode {
    /// The widest margin, in modules.
    pub const MAX_QUIET_ZONE: u8 = 10;
    /// The margin the standard asks for.
    pub const QUIET_ZONE: u8 = 4;
    /// The most bytes the fields of a content hold together.
    pub const MAX_CONTENT: usize = 16 * 1024;

    /// A code of `content`: `dark` squares on `light`.
    #[must_use]
    pub fn new(content: QrContent, dark: Color, light: Color) -> Self {
        Self {
            content,
            ecc: QrEcc::M,
            quiet_zone: Self::QUIET_ZONE,
            style: QrStyle {
                body: QrBody::Square,
                eye_frame: QrEyeFrame::Square,
                eye_ball: QrEyeBall::Square,
                mirror_eyes: true,
                fill: QrFill::Solid(dark),
                eye_frame_fill: None,
                eye_ball_fill: None,
                background: Some(light),
            },
            logo: None,
        }
    }

    /// The correction the code is made with: a logo takes the highest
    /// (S80-10), and taking it off gives back the one chosen.
    #[must_use]
    pub fn effective_ecc(&self) -> QrEcc {
        if self.logo.is_some() {
            QrEcc::H
        } else {
            self.ecc
        }
    }

    /// Whether its numbers, colours and lengths are ones a document holds.
    /// Whether the content makes a code is for `gafira-qr` to say: a code
    /// half written is saved as it is.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        let style = &self.style;
        self.quiet_zone <= Self::MAX_QUIET_ZONE
            && self.content.fields().iter().map(|f| f.len()).sum::<usize>() <= Self::MAX_CONTENT
            && style.fill.is_valid()
            && style.eye_frame_fill.as_ref().is_none_or(QrFill::is_valid)
            && style.eye_ball_fill.as_ref().is_none_or(QrFill::is_valid)
            && style.background.is_none_or(color_is_valid)
            && self.logo.is_none_or(|logo| {
                (QrLogo::MIN_SHARE..=QrLogo::MAX_SHARE).contains(&logo.share)
                    && logo.natural.0 > 0
                    && logo.natural.1 > 0
            })
    }
}
