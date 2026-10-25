//! Exact frame and sample rates and their conversions to and from flicks.

use serde::{Deserialize, Serialize};

use crate::{FLICKS_PER_SECOND, Flicks, TimeError};

/// Greatest common divisor of two non-zero values.
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

/// A rational frame rate with a whole-flick frame duration.
///
/// Fractions are normalized, must be at least one frame per second, and must
/// have a frame duration that is an exact number of flicks. No rate is rounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "FrameRateWire", into = "FrameRateWire")]
pub struct FrameRate {
    num: u32,
    den: u32,
}

#[derive(Serialize, Deserialize)]
struct FrameRateWire {
    num: u32,
    den: u32,
}

impl TryFrom<FrameRateWire> for FrameRate {
    type Error = TimeError;

    fn try_from(wire: FrameRateWire) -> Result<Self, TimeError> {
        Self::new(wire.num, wire.den)
    }
}

impl From<FrameRate> for FrameRateWire {
    fn from(rate: FrameRate) -> Self {
        Self {
            num: rate.num,
            den: rate.den,
        }
    }
}

impl FrameRate {
    /// 23.976 fps (24000/1001).
    pub const FPS_23_976: Self = Self {
        num: 24_000,
        den: 1_001,
    };
    /// 24 fps.
    pub const FPS_24: Self = Self { num: 24, den: 1 };
    /// 25 fps.
    pub const FPS_25: Self = Self { num: 25, den: 1 };
    /// 29.97 fps (30000/1001).
    pub const FPS_29_97: Self = Self {
        num: 30_000,
        den: 1_001,
    };
    /// 30 fps.
    pub const FPS_30: Self = Self { num: 30, den: 1 };
    /// 48 fps.
    pub const FPS_48: Self = Self { num: 48, den: 1 };
    /// 50 fps.
    pub const FPS_50: Self = Self { num: 50, den: 1 };
    /// 59.94 fps (60000/1001).
    pub const FPS_59_94: Self = Self {
        num: 60_000,
        den: 1_001,
    };
    /// 60 fps.
    pub const FPS_60: Self = Self { num: 60, den: 1 };
    /// 120 fps.
    pub const FPS_120: Self = Self { num: 120, den: 1 };

    /// Validates and normalizes a frame rate.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ZeroDenominator`], [`TimeError::ZeroFrameRate`],
    /// [`TimeError::FrameRateBelowOneFps`] or [`TimeError::FrameRateNotIntegral`].
    pub fn new(num: u32, den: u32) -> Result<Self, TimeError> {
        if den == 0 {
            return Err(TimeError::ZeroDenominator);
        }
        if num == 0 {
            return Err(TimeError::ZeroFrameRate);
        }
        let divisor = gcd(num, den);
        let num = num / divisor;
        let den = den / divisor;
        if num < den {
            return Err(TimeError::FrameRateBelowOneFps { num, den });
        }
        let ticks = i128::from(FLICKS_PER_SECOND) * i128::from(den);
        if ticks % i128::from(num) != 0 {
            return Err(TimeError::FrameRateNotIntegral { num, den });
        }
        Ok(Self { num, den })
    }

    /// Normalized numerator.
    pub const fn num(self) -> u32 {
        self.num
    }

    /// Normalized denominator.
    pub const fn den(self) -> u32 {
        self.den
    }

    /// The rate as `f64`; lossy, for display only.
    pub fn fps_f64(self) -> f64 {
        f64::from(self.num) / f64::from(self.den)
    }

    /// The nearest whole frame rate, ties upward; used by timecode labels.
    pub fn nominal_fps(self) -> u32 {
        let rounded = (u64::from(self.num) + u64::from(self.den) / 2) / u64::from(self.den);
        // The invariant num >= den >= 1 keeps this in the u32 range.
        rounded as u32
    }

    /// The exact duration of one frame.
    pub fn flicks_per_frame(self) -> Flicks {
        // `new` guarantees the division is exact; num >= den >= 1 bounds the
        // result by FLICKS_PER_SECOND * u32::MAX, which fits in i64.
        let ticks = i128::from(FLICKS_PER_SECOND) * i128::from(self.den) / i128::from(self.num);
        Flicks::from_ticks(ticks as i64)
    }
}

/// A positive sample rate with a whole-flick sample duration.
///
/// Only rates that divide 705,600,000 exactly are accepted; no rate is rounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "SampleRateWire", into = "SampleRateWire")]
pub struct SampleRate(u32);

#[derive(Serialize, Deserialize)]
struct SampleRateWire {
    hz: u32,
}

impl TryFrom<SampleRateWire> for SampleRate {
    type Error = TimeError;

    fn try_from(wire: SampleRateWire) -> Result<Self, TimeError> {
        Self::new(wire.hz)
    }
}

impl From<SampleRate> for SampleRateWire {
    fn from(rate: SampleRate) -> Self {
        Self { hz: rate.0 }
    }
}

impl SampleRate {
    /// 44,100 Hz.
    pub const HZ_44_100: Self = Self(44_100);
    /// 48,000 Hz.
    pub const HZ_48_000: Self = Self(48_000);
    /// 96,000 Hz.
    pub const HZ_96_000: Self = Self(96_000);

    /// Validates a sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::ZeroSampleRate`] for zero and
    /// [`TimeError::SampleRateNotIntegral`] when one sample would not be a
    /// whole number of flicks.
    pub fn new(hz: u32) -> Result<Self, TimeError> {
        if hz == 0 {
            return Err(TimeError::ZeroSampleRate);
        }
        if FLICKS_PER_SECOND % i64::from(hz) != 0 {
            return Err(TimeError::SampleRateNotIntegral { hz });
        }
        Ok(Self(hz))
    }

    /// The rate in hertz.
    pub const fn hz(self) -> u32 {
        self.0
    }

    /// The exact duration of one sample.
    pub fn flicks_per_sample(self) -> Flicks {
        Flicks::from_ticks(FLICKS_PER_SECOND / i64::from(self.0))
    }
}

impl Flicks {
    /// Exact conversion from a whole frame count.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] when the result does not fit.
    pub fn from_frames(frames: i64, rate: FrameRate) -> Result<Self, TimeError> {
        frames
            .checked_mul(rate.flicks_per_frame().ticks())
            .map(Self::from_ticks)
            .ok_or(TimeError::Overflow)
    }

    /// Frame index that contains this instant, floored towards negative infinity.
    pub fn to_frames_floor(self, rate: FrameRate) -> i64 {
        self.ticks().div_euclid(rate.flicks_per_frame().ticks())
    }

    /// Whether this instant is an exact frame boundary.
    pub fn is_frame_boundary(self, rate: FrameRate) -> bool {
        self.ticks().rem_euclid(rate.flicks_per_frame().ticks()) == 0
    }

    /// Exact conversion from a whole sample count.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] when the result does not fit.
    pub fn from_samples(samples: i64, rate: SampleRate) -> Result<Self, TimeError> {
        samples
            .checked_mul(rate.flicks_per_sample().ticks())
            .map(Self::from_ticks)
            .ok_or(TimeError::Overflow)
    }

    /// Sample index that contains this instant, floored towards negative infinity.
    pub fn to_samples_floor(self, rate: SampleRate) -> i64 {
        self.ticks().div_euclid(rate.flicks_per_sample().ticks())
    }
}
