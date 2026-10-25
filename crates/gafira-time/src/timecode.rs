//! SMPTE-style timecode labels over exact flicks.
//!
//! A timecode is a frame label for display and interchange; flicks remain the
//! source of truth. Drop-frame changes labels only, never the time base.

use std::str::FromStr;

use crate::{Flicks, FrameRate, TimeError};

/// How frame numbers are labeled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimecodeMode {
    /// Every nominal frame number is used.
    NonDrop,
    /// Frame numbers are skipped to follow NTSC wall-clock time.
    DropFrame,
}

/// A frame rate plus the labeling mode it is displayed with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimecodeFormat {
    rate: FrameRate,
    mode: TimecodeMode,
}

impl TimecodeFormat {
    /// Validates a rate and mode pair.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::DropFrameUnsupported`] when drop-frame is requested
    /// for a rate other than 30000/1001 or 60000/1001.
    pub fn new(rate: FrameRate, mode: TimecodeMode) -> Result<Self, TimeError> {
        if mode == TimecodeMode::DropFrame && drop_count(rate).is_none() {
            return Err(TimeError::DropFrameUnsupported {
                num: rate.num(),
                den: rate.den(),
            });
        }
        Ok(Self { rate, mode })
    }

    /// The frame rate.
    pub const fn rate(self) -> FrameRate {
        self.rate
    }

    /// The labeling mode.
    pub const fn mode(self) -> TimecodeMode {
        self.mode
    }
}

/// A validated timecode label in a fixed format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Timecode {
    label_frames: i64,
    format: TimecodeFormat,
}

impl Timecode {
    /// Builds a timecode from a label, rejecting labels the format skips.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::DroppedFrameLabel`] when a drop-frame label names a
    /// skipped frame number.
    pub fn new(label_frames: i64, format: TimecodeFormat) -> Result<Self, TimeError> {
        if format.mode == TimecodeMode::DropFrame && is_dropped_label(label_frames, format.rate) {
            return Err(TimeError::DroppedFrameLabel);
        }
        Ok(Self {
            label_frames,
            format,
        })
    }

    /// The label of the frame that contains `at`.
    ///
    /// Floors towards negative infinity, like every frame conversion in the
    /// crate, so an instant inside a frame never moves to the next one.
    pub fn from_flicks(at: Flicks, format: TimecodeFormat) -> Self {
        let duration = format.rate.flicks_per_frame().ticks();
        let real = at.ticks().div_euclid(duration);
        let label_frames = match format.mode {
            TimecodeMode::NonDrop => real,
            TimecodeMode::DropFrame => label_from_real(real, format.rate),
        };
        Self {
            label_frames,
            format,
        }
    }

    /// Parses `HH:MM:SS:FF` (or `;` before the frames field for drop-frame).
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::MalformedTimecode`] for a wrong shape,
    /// [`TimeError::WrongTimecodeSeparator`] when the frames separator
    /// contradicts the mode, [`TimeError::InvalidTimecodeField`] for a field out
    /// of range, [`TimeError::DroppedFrameLabel`] for a skipped label, and
    /// [`TimeError::Overflow`] when the label does not fit.
    pub fn parse(text: &str, format: TimecodeFormat) -> Result<Self, TimeError> {
        let (negative, body) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let separator_index = body.rfind([':', ';']).ok_or(TimeError::MalformedTimecode)?;
        // The separator is ASCII, so the byte index is a character boundary.
        let separator = char::from(body.as_bytes()[separator_index]);
        let expected = match format.mode {
            TimecodeMode::NonDrop => ':',
            TimecodeMode::DropFrame => ';',
        };
        if separator != expected {
            return Err(TimeError::WrongTimecodeSeparator);
        }
        let (time_part, frames_part) = body.split_at(separator_index);
        let frames_part = &frames_part[1..];

        let mut fields = time_part.split(':');
        let hours_text = fields.next().ok_or(TimeError::MalformedTimecode)?;
        let minutes_text = fields.next().ok_or(TimeError::MalformedTimecode)?;
        let seconds_text = fields.next().ok_or(TimeError::MalformedTimecode)?;
        if fields.next().is_some() {
            return Err(TimeError::MalformedTimecode);
        }
        if !is_canonical_width(hours_text)
            || minutes_text.len() != 2
            || seconds_text.len() != 2
            || !is_canonical_width(frames_part)
        {
            return Err(TimeError::MalformedTimecode);
        }

        let hours: u64 = parse_digits(hours_text)?;
        let minutes: u32 = parse_digits(minutes_text)?;
        let seconds: u32 = parse_digits(seconds_text)?;
        let frames: u32 = parse_digits(frames_part)?;
        if minutes >= 60 || seconds >= 60 || frames >= format.rate.nominal_fps() {
            return Err(TimeError::InvalidTimecodeField);
        }

        let total_seconds = hours
            .checked_mul(3_600)
            .and_then(|value| value.checked_add(u64::from(minutes) * 60 + u64::from(seconds)))
            .ok_or(TimeError::Overflow)?;
        let label = total_seconds
            .checked_mul(u64::from(format.rate.nominal_fps()))
            .and_then(|value| value.checked_add(u64::from(frames)))
            .ok_or(TimeError::Overflow)?;
        let label = i64::try_from(label).map_err(|_| TimeError::Overflow)?;
        let label = if negative {
            if label == 0 {
                return Err(TimeError::MalformedTimecode);
            }
            label.checked_neg().ok_or(TimeError::Overflow)?
        } else {
            label
        };
        Self::new(label, format)
    }

    /// The exact time of this label.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::Overflow`] when the time does not fit.
    pub fn to_flicks(self) -> Result<Flicks, TimeError> {
        let real = match self.format.mode {
            TimecodeMode::NonDrop => self.label_frames,
            TimecodeMode::DropFrame => real_from_label(self.label_frames, self.format.rate),
        };
        Flicks::from_frames(real, self.format.rate)
    }

    /// The label frame count.
    pub const fn label_frames(self) -> i64 {
        self.label_frames
    }

    /// The format this label belongs to.
    pub const fn format_spec(self) -> TimecodeFormat {
        self.format
    }

    /// Formats the label as `HH:MM:SS:FF` (or `HH:MM:SS;FF` for drop-frame).
    pub fn format(self) -> String {
        let nominal = u64::from(self.format.rate.nominal_fps());
        let magnitude = self.label_frames.unsigned_abs();
        let frames = magnitude % nominal;
        let total_seconds = magnitude / nominal;
        let seconds = total_seconds % 60;
        let total_minutes = total_seconds / 60;
        let minutes = total_minutes % 60;
        let hours = total_minutes / 60;
        let sign = if self.label_frames < 0 { "-" } else { "" };
        let separator = match self.format.mode {
            TimecodeMode::NonDrop => ':',
            TimecodeMode::DropFrame => ';',
        };
        format!("{sign}{hours:02}:{minutes:02}:{seconds:02}{separator}{frames:02}")
    }
}

/// Drop-frame skipped numbers per minute, for the two supported rates.
fn drop_count(rate: FrameRate) -> Option<u32> {
    if rate == FrameRate::FPS_29_97 {
        Some(2)
    } else if rate == FrameRate::FPS_59_94 {
        Some(4)
    } else {
        None
    }
}

/// Whether `label` names a frame number that drop-frame skips.
fn is_dropped_label(label: i64, rate: FrameRate) -> bool {
    let Some(drop) = drop_count(rate) else {
        return false;
    };
    let nominal = u64::from(rate.nominal_fps());
    let magnitude = label.unsigned_abs();
    let total_minutes = magnitude / (nominal * 60);
    let remainder = magnitude % (nominal * 60);
    let seconds = remainder / nominal;
    let frames = remainder % nominal;
    !total_minutes.is_multiple_of(10) && seconds == 0 && frames < u64::from(drop)
}

/// Label for a real frame index, applied symmetrically to the magnitude.
fn label_from_real(real: i64, rate: FrameRate) -> i64 {
    let Some(drop) = drop_count(rate) else {
        return real;
    };
    let magnitude = real.unsigned_abs();
    let nominal = u64::from(rate.nominal_fps());
    let drop = u64::from(drop);
    let frames_per_10_minutes = nominal * 600 - drop * 9;
    let frames_per_minute = nominal * 60 - drop;
    let block = magnitude / frames_per_10_minutes;
    let remainder = magnitude % frames_per_10_minutes;
    let local = if remainder < nominal * 60 {
        remainder
    } else {
        let after_first_minute = remainder - nominal * 60;
        let minute = 1 + after_first_minute / frames_per_minute;
        minute * nominal * 60 + (after_first_minute % frames_per_minute) + drop
    };
    let label = block * nominal * 600 + local;
    if real.is_negative() {
        -(label as i64)
    } else {
        label as i64
    }
}

/// Real frame index for a valid label, applied symmetrically to the magnitude.
fn real_from_label(label: i64, rate: FrameRate) -> i64 {
    let Some(drop) = drop_count(rate) else {
        return label;
    };
    let nominal = u64::from(rate.nominal_fps());
    let magnitude = label.unsigned_abs();
    let total_minutes = magnitude / (nominal * 60);
    let skipped = u64::from(drop) * (total_minutes - total_minutes / 10);
    let real = magnitude - skipped;
    if label.is_negative() {
        -(real as i64)
    } else {
        real as i64
    }
}

/// Whether a hours or frames field has at least two digits and no zero padding
/// beyond that minimum.
fn is_canonical_width(text: &str) -> bool {
    text.len() >= 2 && (text.len() == 2 || !text.starts_with('0'))
}

/// Parses a non-empty run of ASCII digits.
fn parse_digits<T: FromStr>(text: &str) -> Result<T, TimeError> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(TimeError::MalformedTimecode);
    }
    text.parse::<T>().map_err(|_| TimeError::Overflow)
}
