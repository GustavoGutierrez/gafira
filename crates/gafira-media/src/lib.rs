//! The media engine of Gafira: the keyframe index, the frame cache, the
//! proxies, the playback clock and the video export (02 §6.5).
//!
//! The crate is pure in the sense of ARQ-02: it reads no clock of its own,
//! touches no file system of its own and creates no thread. The time comes
//! from `ports::Clock`, the file system from `ports::FileSystem` and the
//! workers from the pool the caller injects (ADR 0016, ARQ-138).

#![forbid(unsafe_code)]

mod cache;
mod exporter;
mod index;
mod medium;
mod playback;
mod ports;
mod proxy;

pub use cache::{
    DEFAULT_BUDGET, FrameCache, FrameHit, FrameKey, FrameRequest, HeadDirection, MediumRef,
    ProxyLevel, default_budget,
};
pub use exporter::{
    ExportMedium, ExportSource, ExportSourceFactory, OPTION_AUDIO_BITRATE, OPTION_AUDIO_CODEC,
    OPTION_BITRATE, OPTION_CHANNELS, OPTION_CODEC, OPTION_END_MS, OPTION_FPS, OPTION_HEIGHT,
    OPTION_SAMPLE_RATE, OPTION_START_MS, OPTION_WIDTH, VIDEO_FORMAT, VideoExporter,
};
pub use index::{INDEX_BUDGET_PER_MINUTE, IndexReport, KeyframeIndex};
pub use medium::{Medium, SeekOutcome};
pub use playback::{
    FrameDecision, PlayReport, PlaybackClock, PlaybackSource, SYNC_TOLERANCE, Speed, StepDirection,
};
pub use ports::{Job, Ports, WorkerPool};
pub use proxy::{
    DEFAULT_PROXY_BUDGET, ProxyKey, ProxyProducer, ProxyReport, ProxySource, ProxyStore, ProxyUse,
};
