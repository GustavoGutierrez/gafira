//! Three-band equaliser: RBJ biquads, low shelf at 100 Hz, peak at 1 kHz and
//! high shelf at 8 kHz (ARQ-125, D-22).

use std::f64::consts::{PI, SQRT_2};

use gafira_doc::contract::{Describe, Descriptor, ParamSchema};
use gafira_doc::msg::MessageKey;

use crate::dsp::{Param, Setup, clean_block, clean_input, clean_output};
use crate::{AudioBlock, AudioProcessor, ParamSnapshot};

/// Id of the low-shelf gain parameter, in dB.
pub const PARAM_LOW: &str = "low";
/// Id of the peak gain parameter, in dB.
pub const PARAM_MID: &str = "mid";
/// Id of the high-shelf gain parameter, in dB.
pub const PARAM_HIGH: &str = "high";

static DESCRIPTOR: Descriptor = Descriptor::new("audio.eq3band", MessageKey("audio.eq3band"), 1);

/// Gain range of a band, in dB.
const RANGE_DB: f64 = 15.0;

#[derive(Clone, Copy, Debug)]
enum Kind {
    /// Shelf slope `S = 1`.
    LowShelf,
    /// `Q = 1`.
    Peak,
    /// Shelf slope `S = 1`.
    HighShelf,
}

/// Normalised coefficients (`a0 = 1`) of one biquad.
#[derive(Clone, Copy, Debug)]
struct Coefficients {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Coefficients {
    /// The Audio EQ Cookbook design, or `None` when the band does nothing:
    /// 0 dB, or a centre frequency at or above `fs / 2`.
    fn design(kind: Kind, centre_hz: u32, sample_rate: u32, db: f64) -> Option<Self> {
        if db == 0.0 || u64::from(centre_hz) * 2 >= u64::from(sample_rate) {
            return None;
        }
        let a = 10_f64.powf(db / 40.0);
        let w0 = 2.0 * PI * f64::from(centre_hz) / f64::from(sample_rate);
        let (cos, sin) = (w0.cos(), w0.sin());
        let (b, den) = match kind {
            Kind::Peak => {
                // alpha = sin(w0) / (2 Q), Q = 1
                let alpha = sin / 2.0;
                (
                    [1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a],
                    [1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a],
                )
            }
            Kind::LowShelf => {
                // 2 sqrt(A) alpha, with alpha = sin(w0) / 2 * sqrt(2) for S = 1
                let t = 2.0 * a.sqrt() * (sin / 2.0 * SQRT_2);
                (
                    [
                        a * ((a + 1.0) - (a - 1.0) * cos + t),
                        2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                        a * ((a + 1.0) - (a - 1.0) * cos - t),
                    ],
                    [
                        (a + 1.0) + (a - 1.0) * cos + t,
                        -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                        (a + 1.0) + (a - 1.0) * cos - t,
                    ],
                )
            }
            Kind::HighShelf => {
                let t = 2.0 * a.sqrt() * (sin / 2.0 * SQRT_2);
                (
                    [
                        a * ((a + 1.0) + (a - 1.0) * cos + t),
                        -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                        a * ((a + 1.0) + (a - 1.0) * cos - t),
                    ],
                    [
                        (a + 1.0) - (a - 1.0) * cos + t,
                        2.0 * ((a - 1.0) - (a + 1.0) * cos),
                        (a + 1.0) - (a - 1.0) * cos - t,
                    ],
                )
            }
        };
        Some(Self {
            b0: b[0] / den[0],
            b1: b[1] / den[0],
            b2: b[2] / den[0],
            a1: den[1] / den[0],
            a2: den[2] / den[0],
        })
    }
}

/// Direct form I history of one band on one channel.
#[derive(Clone, Copy, Debug, Default)]
struct History {
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl History {
    /// Runs one sample. `None` passes it through, and the history follows it,
    /// so a band switched on later starts from a consistent state.
    fn step(&mut self, c: Option<&Coefficients>, x: f64) -> f64 {
        let y = match c {
            Some(c) => {
                let y =
                    c.b0 * x + c.b1 * self.x1 + c.b2 * self.x2 - c.a1 * self.y1 - c.a2 * self.y2;
                // Below the smallest normal `f32` the ring-down is silence.
                if y.abs() < f64::from(f32::MIN_POSITIVE) {
                    0.0
                } else {
                    y
                }
            }
            None => x,
        };
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

#[derive(Debug)]
struct Band {
    kind: Kind,
    centre_hz: u32,
    gain: Param,
    /// The gain the coefficients were designed for.
    designed_db: f64,
    coefficients: Option<Coefficients>,
}

impl Band {
    fn new(id: &'static str, kind: Kind, centre_hz: u32) -> Self {
        Self {
            kind,
            centre_hz,
            gain: Param::new(id, -RANGE_DB, RANGE_DB, 0.0),
            designed_db: 0.0,
            coefficients: None,
        }
    }

    /// True when the band changes the signal at `sample_rate`: its centre is
    /// below `fs / 2` and the last valid gain is not 0 dB.
    fn is_active(&self, sample_rate: u32) -> bool {
        u64::from(self.centre_hz) * 2 < u64::from(sample_rate) && self.gain.value() != 0.0
    }

    fn design(&mut self, sample_rate: u32) {
        self.coefficients =
            Coefficients::design(self.kind, self.centre_hz, sample_rate, self.designed_db);
    }
}

/// Three biquad bands in series. A band at 0 dB, or at or above `fs / 2`,
/// does nothing: with every band at 0 dB the output equals the input bit for
/// bit. A new gain applies from the first sample of the next block and the
/// histories carry on.
#[derive(Debug)]
pub struct Eq3Band {
    setup: Setup,
    bands: [Band; 3],
    /// Per channel, the history of each band.
    state: Vec<[History; 3]>,
    schema: ParamSchema,
}

impl Eq3Band {
    /// All bands at 0 dB, not prepared.
    #[must_use]
    pub fn new() -> Self {
        let bands = [
            Band::new(PARAM_LOW, Kind::LowShelf, 100),
            Band::new(PARAM_MID, Kind::Peak, 1000),
            Band::new(PARAM_HIGH, Kind::HighShelf, 8000),
        ];
        let labels = [
            MessageKey("audio.eq3band.low"),
            MessageKey("audio.eq3band.mid"),
            MessageKey("audio.eq3band.high"),
        ];
        let defs = bands
            .iter()
            .zip(labels)
            .map(|(b, label)| b.gain.def(label, false))
            .collect();
        Self {
            schema: ParamSchema::new(defs),
            setup: Setup::default(),
            bands,
            state: Vec::new(),
        }
    }
}

impl Default for Eq3Band {
    fn default() -> Self {
        Self::new()
    }
}

impl Describe for Eq3Band {
    fn descriptor(&self) -> &Descriptor {
        &DESCRIPTOR
    }
}

impl AudioProcessor for Eq3Band {
    fn params(&self) -> &ParamSchema {
        &self.schema
    }

    fn prepare(&mut self, sample_rate: u32, max_block: usize, channels: u16) {
        self.setup = Setup {
            sample_rate,
            max_block,
            channels,
        };
        self.state = vec![[History::default(); 3]; usize::from(channels)];
        for band in &mut self.bands {
            band.design(sample_rate);
        }
    }

    fn process(&mut self, block: &mut AudioBlock<'_>, params: &ParamSnapshot) {
        if !self.setup.accepts(block) {
            clean_block(block);
            return;
        }
        for band in &mut self.bands {
            let db = band.gain.read(params);
            if db.to_bits() != band.designed_db.to_bits() {
                band.designed_db = db;
                band.design(self.setup.sample_rate);
            }
        }
        for (channel, history) in block.channels.iter_mut().zip(&mut self.state) {
            for sample in channel.iter_mut() {
                let mut v = f64::from(clean_input(*sample));
                for (band, h) in self.bands.iter().zip(history.iter_mut()) {
                    v = h.step(band.coefficients.as_ref(), v);
                }
                *sample = clean_output(v);
            }
        }
    }

    /// `ceil(0.04 × fs)` frames with a band active, else 0: the ring-down is
    /// below −120 dB by then. Integer arithmetic keeps it exact.
    fn tail(&self) -> u32 {
        let fs = self.setup.sample_rate;
        if self.bands.iter().any(|b| b.is_active(fs)) {
            // fs ≤ u32::MAX, so ceil(4 fs / 100) fits in a u32.
            u32::try_from((u64::from(fs) * 4).div_ceil(100)).unwrap_or(u32::MAX)
        } else {
            0
        }
    }

    fn reset(&mut self) {
        self.state.fill([History::default(); 3]);
    }
}
