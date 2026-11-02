//! The mix: a plain sum of the tracks that are not muted (ARQ-123, ARQ-129).

use crate::AudioBlock;
use crate::dsp::{clean_input, clean_output};

/// One track as read by the mix: planar samples and its mute flag.
#[derive(Clone, Copy, Debug)]
pub struct MixTrack<'a> {
    /// One slice per channel.
    pub channels: &'a [&'a [f32]],
    /// A muted track adds nothing.
    pub muted: bool,
}

/// Overwrites `out` with the sum of the tracks that are not muted.
///
/// There is no track limit and no clipping to ±1. A NaN or infinite sample
/// counts as 0, the sum never leaves `±f32::MAX`, and nothing is subnormal.
/// A track with fewer channels or frames than `out` adds only what it has.
pub fn mix(tracks: &[MixTrack<'_>], out: &mut AudioBlock<'_>) {
    for (c, channel) in out.channels.iter_mut().enumerate() {
        for (i, sample) in channel.iter_mut().enumerate() {
            let mut sum = 0.0_f64;
            for track in tracks.iter().filter(|t| !t.muted) {
                if let Some(&x) = track.channels.get(c).and_then(|t| t.get(i)) {
                    sum += f64::from(clean_input(x));
                }
            }
            *sample = clean_output(sum);
        }
    }
}
