//! One medium: the container opened and the keyframe index built from it
//! (ARQ-144).

use std::path::Path;

use gafira_doc::contract::{CancelToken, Progress};
use gafira_ports::media::{Container, Decoder, Demuxer, Frame, MediaError, SeekMode, StreamIdx};
use gafira_time::Flicks;

use crate::index::{IndexReport, KeyframeIndex};
use crate::ports::Ports;

/// Where a seek landed (ARQ-144).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeekOutcome {
    /// The keyframe at or before the requested time.
    Keyframe(Flicks),
    /// The frame the forward decode landed on: the first frame at or after
    /// the requested time.
    Exact(Flicks),
    /// The medium has no keyframe to land on: its index is empty.
    NoKeyframe,
    /// The medium ends before the requested time: the forward decode found
    /// no frame there.
    NoFrame,
}

/// One medium: the demuxer of the container and the index of its keyframes.
pub struct Medium {
    demuxer: Box<dyn Demuxer>,
    index: KeyframeIndex,
    report: IndexReport,
}

impl Medium {
    /// Opens a medium and indexes its keyframes (ARQ-144).
    ///
    /// The file is read through the file system port and the container
    /// gives the demuxer. The index is built in memory and nothing is
    /// written: no index file reaches the user cache nor the `.gafira`
    /// (ARQ-144, 02 §7.1).
    ///
    /// # Errors
    ///
    /// [`MediaError`] when the file cannot be read, when the container
    /// refuses it or when reading a packet fails, and
    /// [`MediaError::Cancelled`] when the caller cancels the scan.
    pub fn open(
        ports: &Ports<'_>,
        path: &Path,
        container: &dyn Container,
        stream: StreamIdx,
        cancel: &CancelToken,
        progress: &dyn Progress,
    ) -> Result<Self, MediaError> {
        let source = ports.file_system.open(path).map_err(MediaError::Io)?;
        let mut demuxer = container.open(source)?;
        let (index, report) = KeyframeIndex::build(
            &mut *demuxer,
            stream,
            ports.pool,
            ports.clock,
            cancel,
            progress,
        )?;
        Ok(Self {
            demuxer,
            index,
            report,
        })
    }

    /// The index of the keyframes of the medium.
    #[must_use]
    pub fn index(&self) -> &KeyframeIndex {
        &self.index
    }

    /// What the index build cost.
    #[must_use]
    pub fn report(&self) -> IndexReport {
        self.report
    }

    /// Seeks to `time` (ARQ-144).
    ///
    /// `SeekMode::Keyframe` lands on the keyframe at or before `time`.
    /// `SeekMode::Exact` lands on that same keyframe and decodes forward to
    /// the frame at `time`. The decoder comes from the caller and must be
    /// fresh: the port has no way to reset it.
    ///
    /// # Errors
    ///
    /// [`MediaError`] when the container cannot seek or the decoder fails,
    /// and [`MediaError::Cancelled`] when the caller cancels the decode.
    pub fn seek(
        &mut self,
        decoder: &mut dyn Decoder,
        time: Flicks,
        mode: SeekMode,
        cancel: &CancelToken,
        progress: &dyn Progress,
    ) -> Result<SeekOutcome, MediaError> {
        let Some(keyframe) = self.index.keyframe_at_or_before(time) else {
            return Ok(SeekOutcome::NoKeyframe);
        };
        self.demuxer.seek(keyframe, SeekMode::Keyframe)?;
        if mode == SeekMode::Keyframe {
            progress.report(1.0);
            return Ok(SeekOutcome::Keyframe(keyframe));
        }
        loop {
            cancel.check()?;
            let Some(packet) = self.demuxer.read_packet()? else {
                progress.report(1.0);
                return Ok(SeekOutcome::NoFrame);
            };
            decoder.send(Some(&packet))?;
            while let Some(frame) = decoder.receive()? {
                let Frame::Video(frame) = &frame else {
                    continue;
                };
                if frame.pts >= time {
                    progress.report(1.0);
                    return Ok(SeekOutcome::Exact(frame.pts));
                }
            }
        }
    }
}
