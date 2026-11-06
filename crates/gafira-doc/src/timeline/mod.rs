//! The timeline model of 02 §3.1: sequences, tracks, clips, markers and the asset
//! table, with the laws that make them a timeline.
//!
//! Everything here is the data of the timeline; the commands that rest on it and
//! the persistence wire form are later work. The time types are exactly
//! `gafira-time`'s: a cut lands on a whole frame of the sequence's rate, and no
//! rate is ever rounded.

mod asset;
mod clip;
mod error;
mod sequence;
mod track;

pub use asset::{AssetEntry, AssetRef, AssetStorage, AssetTable};
pub use clip::{Clip, ClipSource, Speed};
pub use error::TimelineError;
pub use sequence::{AUDIO_TRACK_CAP, PixelSize, PlaceSpan, Sequence};
pub use track::{Marker, Track, TrackKind};
