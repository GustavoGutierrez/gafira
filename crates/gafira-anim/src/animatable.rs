use gafira_time::Flicks;
use serde::{Deserialize, Serialize};

use crate::{Easing, Interpolate, KeyTrack, Keyframe};

/// A value that is either fixed or animated by a [`KeyTrack`] (ARQ-16, ARQ-18).
///
/// Serde is untagged: `Static(v)` is written as `v`, `Keyed` as
/// `{"keys":[...]}`. A `Keyed` track that breaks the [`KeyTrack`] invariant
/// fails to read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Animatable<T: Interpolate> {
    Static(T),
    Keyed(KeyTrack<T>),
}

impl<T: Interpolate> Animatable<T> {
    /// The value at `t`, following the sampling table of ARQ-16. Total.
    pub fn sample(&self, t: Flicks) -> T {
        match self {
            Self::Static(v) => v.clone(),
            Self::Keyed(track) => sample_keys(track.keys(), t),
        }
    }
}

/// `keys` is non-empty with strictly increasing `at` (the `KeyTrack` invariant).
fn sample_keys<T: Interpolate>(keys: &[Keyframe<T>], t: Flicks) -> T {
    let last = &keys[keys.len() - 1];
    if t <= keys[0].at {
        return keys[0].value.clone();
    }
    if t >= last.at {
        return last.value.clone();
    }
    // keys[0].at < t < last.at, so `i` is in 0..len-1 and keys[i].at <= t < keys[i + 1].at.
    let i = keys.partition_point(|k| k.at <= t) - 1;
    let (from, to) = (&keys[i], &keys[i + 1]);
    if t == from.at || from.out == Easing::Hold {
        return from.value.clone();
    }
    // i128 holds any difference of two i64 values; the quotient is in (0, 1).
    let elapsed = i128::from(t.ticks()) - i128::from(from.at.ticks());
    let span = i128::from(to.at.ticks()) - i128::from(from.at.ticks());
    let u = (elapsed as f64 / span as f64) as f32;
    from.value.interpolate(&to.value, from.out.apply(u))
}
