//! Autosave over a snapshot (02 §7.2 "Autoguardado rotativo", design P13).
//!
//! The engine schedules and the host runs: `ProjectService::autosave_due`
//! hands out an [`AutosaveJob`] that holds everything it writes, the host runs
//! it on its I/O thread or inline, and hands the [`AutosaveOutcome`] back to
//! `ProjectService::autosave_finished`. The engine spawns nothing and reads
//! every instant from `ports::Clock` (P1, P8).
//!
//! Interim (owner decision O5 = (a)): a slot holds a whole project written by
//! `ProjectRepository::save`. The JSON-only journal and the 200 ms figure of
//! §7.2 are not met here; a journal method in `gafira-ports` replaces
//! [`AutosaveJob::run`]'s write before the release.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gafira_doc::Document;
use gafira_ports::repository::{ProjectRepository, Version};
use gafira_ports::system::{FileSystem, ProcessId};

use crate::engine::EngineError;
use crate::session::DocVersion;

/// How many recovery slots a project rotates through (02 §7.2).
pub(crate) const SLOTS: u8 = 3;

/// One recovery slot to write: the document as it was when the job was made,
/// and the ports to write it with (design P13). `Send`, so the host can run
/// it on its I/O thread.
pub struct AutosaveJob {
    pub(crate) snapshot: Arc<Document>,
    /// The versions of the session, written with the slot so a recovery
    /// keeps them (spec 031).
    pub(crate) versions: Vec<Version>,
    pub(crate) version: DocVersion,
    pub(crate) slot: u8,
    pub(crate) folder: PathBuf,
    pub(crate) session_key: String,
    /// The user's file the session lives in, recorded beside the slots so a
    /// recovery knows where the work belongs (spec 016).
    pub(crate) origin: Option<PathBuf>,
    /// This process, recorded in the marker so another running instance
    /// leaves the slots alone (spec 028).
    pub(crate) owner: ProcessId,
    pub(crate) repository: Arc<dyn ProjectRepository>,
    pub(crate) filesystem: Arc<dyn FileSystem>,
}

/// The file beside the slots that names the newest one, the process that
/// wrote it and the user's file (specs 016 and 028): `latest`, three lines,
/// written atomically after each slot.
pub(crate) const MARKER: &str = "latest";

/// What a marker says.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Marker {
    /// The newest slot, `0..SLOTS`.
    pub(crate) slot: u8,
    /// The process whose session wrote the slots (spec 028).
    pub(crate) owner: ProcessId,
    /// The user's file the slots belong to, if it has one.
    pub(crate) origin: Option<PathBuf>,
}

/// `"<slot>\n<pid>\n<path>"`; the path line is empty for a document never
/// saved or a path that is not UTF-8 (the recovery then asks for «Guardar
/// como»).
pub(crate) fn marker_text(slot: u8, owner: ProcessId, origin: Option<&Path>) -> String {
    let path = origin.and_then(Path::to_str).unwrap_or_default();
    format!("{slot}\n{}\n{path}", owner.0)
}

/// What a marker names; `None` when it is not one (a marker of spec 016,
/// without the owner, is not one either: its slots stay on disk).
pub(crate) fn parse_marker(text: &str) -> Option<Marker> {
    let mut lines = text.splitn(3, '\n');
    let slot: u8 = lines.next()?.trim().parse().ok().filter(|n| *n < SLOTS)?;
    let owner = ProcessId(lines.next()?.trim().parse().ok()?);
    let origin = lines
        .next()
        .map(|p| p.trim_end_matches('\n'))
        .filter(|p| !p.is_empty())
        .map(PathBuf::from);
    Some(Marker {
        slot,
        owner,
        origin,
    })
}

impl AutosaveJob {
    /// The version of the document this job writes.
    #[must_use]
    pub fn version(&self) -> DocVersion {
        self.version
    }

    /// `<folder>/slot-<n>.gaf-journal`.
    fn slot_path(&self) -> PathBuf {
        self.folder.join(format!("slot-{}.gaf-journal", self.slot))
    }

    /// Write the slot: create its folder, let the repository write the
    /// snapshot to `slot-<n>.gaf-journal.tmp` in it, then replace the slot
    /// with that file. On a failure the temporary file is removed on a
    /// best-effort basis and the slot keeps its previous bytes.
    #[must_use]
    pub fn run(self) -> AutosaveOutcome {
        let slot = self.slot_path();
        let mut tmp = slot.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        let result = self.write(&tmp, &slot);
        if result.is_err() {
            // Best effort: the first error is the one the host needs.
            let _ = self.filesystem.remove(&tmp);
        }
        AutosaveOutcome {
            session_key: self.session_key,
            result,
        }
    }

    /// The folder, then the repository writes and syncs `tmp` (SP6), then
    /// the file system renames it over `slot` (the save's steps, P13, P18),
    /// then the marker names it as the newest (spec 016). A failed marker
    /// leaves the previous one, which still names a complete slot.
    fn write(&self, tmp: &Path, slot: &Path) -> Result<(), EngineError> {
        self.filesystem.create_dir_all(&self.folder)?;
        self.repository.save(tmp, &self.snapshot, &self.versions)?;
        self.filesystem.replace(tmp, slot)?;
        let marker = self.folder.join(MARKER);
        let marker_tmp = self.folder.join(format!("{MARKER}.tmp"));
        let text = marker_text(self.slot, self.owner, self.origin.as_deref());
        let written = (|| -> std::io::Result<()> {
            let mut file = self.filesystem.create(&marker_tmp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            self.filesystem.replace(&marker_tmp, &marker)
        })();
        if written.is_err() {
            let _ = self.filesystem.remove(&marker_tmp);
        }
        written.map_err(EngineError::Io)
    }
}

/// What a run of an [`AutosaveJob`] came to; hand it to
/// `ProjectService::autosave_finished`.
#[derive(Debug)]
pub struct AutosaveOutcome {
    pub(crate) session_key: String,
    pub(crate) result: Result<(), EngineError>,
}

impl AutosaveOutcome {
    /// Why the run failed, if it did, for the host's log.
    #[must_use]
    pub fn error(&self) -> Option<&EngineError> {
        self.result.as_ref().err()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use gafira_doc::{Document, PageId};
    use gafira_edit::Applied;
    use gafira_ports::repository::ProjectRepository;

    use super::*;
    use crate::edit::EditService;
    use crate::engine::Gafira;
    use crate::event::EngineEvent;
    use crate::fakes::*;

    const TARGET: &str = "/work/poster.gaf";
    const T0: Duration = Duration::from_secs(100);
    const INTERVAL: Duration = Duration::from_secs(10);
    const STEP: Duration = Duration::from_nanos(1);

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    /// Slot `n` of the `session`-th session of an engine (from 0): the
    /// fixture carries no project id, so each session mints the next one of
    /// `SequentialIds` and its slots live under it (D-63, spec 012).
    fn slot_of(session: u64, n: u8) -> String {
        let id = SequentialIds::nth(u128::from(session) + 1);
        format!("/data/gafira/recovery/{id}/slot-{n}.gaf-journal")
    }

    /// Slot `n` of the first session, the one [`Rig::opened`] opens.
    fn slot(n: u8) -> String {
        slot_of(0, n)
    }

    fn tmp(n: u8) -> PathBuf {
        PathBuf::from(format!("{}.tmp", slot(n)))
    }

    /// The trace of one job writing `len` bytes to slot `n` of the first
    /// session: the folder, the repository's create, two writes and sync,
    /// the `replace`, then the marker naming slot `n` and [`TARGET`]
    /// (spec 016) written, synced and renamed.
    fn job_trace(n: u8, len: usize) -> Vec<FsOp> {
        let folder = Path::new(&slot(n)).parent().expect("a folder").to_owned();
        let marker_tmp = folder.join("latest.tmp");
        let marker = marker_text(n, FakeProcesses::PID, Some(Path::new(TARGET)));
        vec![
            FsOp::CreateDirAll(folder.clone()),
            FsOp::Create(tmp(n)),
            FsOp::Write(tmp(n), len / 2),
            FsOp::Write(tmp(n), len - len / 2),
            FsOp::SyncAll(tmp(n)),
            FsOp::Replace(tmp(n), slot(n).into()),
            FsOp::Create(marker_tmp.clone()),
            FsOp::Write(marker_tmp.clone(), marker.len()),
            FsOp::SyncAll(marker_tmp.clone()),
            FsOp::Replace(marker_tmp, folder.join("latest")),
        ]
    }

    /// An engine over memory fakes and a clock the test moves, with
    /// [`two_page_document`] at [`TARGET`].
    struct Rig {
        engine: Gafira,
        fs: MemoryFs,
        repository: Arc<MemoryRepository>,
        clock: Arc<ManualClock>,
    }

    impl Rig {
        /// No session yet, the clock at [`T0`].
        fn new(read_only: bool) -> Self {
            let fs = MemoryFs::default();
            let json = two_page_document().to_json().expect("json");
            fs.put(TARGET, json.as_bytes());
            let repository = Arc::new(MemoryRepository::new(&fs, read_only));
            let clock = Arc::new(ManualClock::at(T0));
            let engine = engine_with_clock(&fs, &repository, Arc::clone(&clock) as _);
            Self {
                engine,
                fs,
                repository,
                clock,
            }
        }

        /// [`TARGET`] opened at [`T0`] (version 1, the first session), with
        /// the events, the trace and the repository calls cleared.
        fn opened(read_only: bool) -> Self {
            let rig = Self::new(read_only);
            rig.open();
            drain(&rig.engine);
            rig.fs.take_trace();
            rig.repository.take_calls();
            rig
        }

        fn open(&self) {
            let projects = self.engine.projects();
            projects.open(Path::new(TARGET)).expect("an open");
        }

        fn at(&self, t: Duration) {
            self.clock.set(t);
        }

        fn due_at(&self, t: Duration) -> Option<AutosaveJob> {
            self.at(t);
            self.engine.projects().autosave_due()
        }

        /// Append a page at `t`.
        fn edit_at(&mut self, t: Duration) -> PageId {
            self.at(t);
            append_page(&mut self.engine)
        }

        /// Run `job` and hand its outcome back at `t`.
        fn finish_at(&self, t: Duration, job: AutosaveJob) {
            let outcome = job.run();
            self.at(t);
            self.engine.projects().autosave_finished(outcome);
        }

        fn document(&mut self) -> Arc<Document> {
            self.engine.edit().document().expect("a session")
        }

        /// The bytes the open document serialises to.
        fn bytes(&mut self) -> Vec<u8> {
            self.document().to_json().expect("json").into_bytes()
        }
    }

    // ARQ-51
    #[test]
    fn ac2_due_at_exactly_the_interval_after_the_first_pending_change() {
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        rig.edit_at(T0 + secs(3));
        assert!(
            rig.due_at(T0 + INTERVAL - STEP).is_none(),
            "one step before"
        );
        let job = rig.due_at(T0 + INTERVAL).expect("due at the interval");
        assert_eq!(job.slot_path(), PathBuf::from(slot(0)));
        assert_eq!(job.version(), DocVersion(3), "the open, then two edits");
    }

    // ARQ-51
    #[test]
    fn ac3_no_job_without_changes() {
        let mut rig = Rig::new(false);
        let hour = secs(3600);
        assert!(rig.due_at(T0 + hour).is_none(), "no session");
        rig.engine
            .projects()
            .new_document()
            .expect("a new document");
        assert!(rig.due_at(T0 + hour * 2).is_none(), "after new_document");
        rig.open();
        assert!(rig.due_at(T0 + hour * 3).is_none(), "after open");
        rig.edit_at(T0 + hour * 3);
        rig.engine.projects().save().expect("a save");
        assert!(rig.due_at(T0 + hour * 4).is_none(), "after a save");
        rig.edit_at(T0 + hour * 4);
        let job = rig.due_at(T0 + hour * 4 + INTERVAL);
        assert!(job.is_some(), "a change after the save is due");
    }

    // ARQ-51
    #[test]
    fn ac3_a_save_leaves_pending_a_change_made_while_it_wrote() {
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        let shared = rig.engine.projects().shared.clone();
        let mut edit = EditService::new(shared, Arc::clone(&rig.clock) as _);
        rig.fs.before_replace(move || {
            let applied = edit.execute(rename(PageId::new(1), "later", None));
            assert_eq!(applied.ok(), Some(Applied::Changed));
        });
        rig.engine.projects().save().expect("a save");
        let job = rig
            .due_at(T0 + INTERVAL)
            .expect("the change made during the save");
        assert_eq!(
            job.version(),
            DocVersion(3),
            "the open, the edit, the rename"
        );
    }

    // ARQ-51
    #[test]
    fn ac4_the_slot_holds_the_document_as_it_was_when_the_job_was_made() {
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        let expected = rig.document();
        let job = rig.due_at(T0 + INTERVAL).expect("a job");
        rig.edit_at(T0 + INTERVAL);
        assert!(job.run().error().is_none(), "the run");
        let loaded = rig.repository.load(Path::new(&slot(0))).expect("the slot");
        assert_eq!(loaded.document, *expected, "three pages");
        assert_eq!(rig.document().pages().len(), 4, "the later edit");
    }

    // ARQ-51
    #[test]
    fn ac5_a_change_during_a_job_stays_pending_and_comes_one_interval_after_it() {
        // While the job is in flight there is no second one, however late.
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        let job = rig.due_at(T0 + INTERVAL).expect("the first job");
        rig.edit_at(T0 + secs(12));
        assert!(rig.due_at(T0 + secs(60)).is_none(), "in flight");
        rig.finish_at(T0 + secs(60), job);
        assert!(rig.due_at(T0 + secs(60)).is_some(), "the change during it");

        // Finished early: the next job is one interval after the change.
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        let job = rig.due_at(T0 + INTERVAL).expect("the first job");
        rig.edit_at(T0 + secs(12));
        rig.finish_at(T0 + secs(13), job);
        assert!(
            rig.due_at(T0 + secs(22) - STEP).is_none(),
            "one step before"
        );
        let next = rig.due_at(T0 + secs(22)).expect("one interval after it");
        assert_eq!(next.slot_path(), PathBuf::from(slot(1)));
        assert_eq!(next.version(), DocVersion(3), "with the change");
    }

    // ARQ-51
    #[test]
    fn ac6_slots_rotate_on_success_only_and_a_failed_job_writes_its_slot_again() {
        let mut rig = Rig::opened(false);
        let mut written = Vec::new();
        let mut t = T0;
        for fail in [false, false, true, false, false] {
            rig.edit_at(t);
            t += INTERVAL;
            let job = rig.due_at(t).expect("a job");
            rig.fs.fail_at(if fail { &[5] } else { &[] });
            let outcome = job.run();
            assert_eq!(outcome.error().is_some(), fail, "at {t:?}");
            rig.engine.projects().autosave_finished(outcome);
            let trace = rig.fs.take_trace();
            if fail {
                assert_eq!(trace.last(), Some(&FsOp::Remove(tmp(2))), "the failed one");
            }
            let replaced = trace.into_iter().find_map(|op| match op {
                FsOp::Replace(_, to) => Some(to),
                _ => None,
            });
            written.push(replaced.expect("a replace"));
        }
        let expected: Vec<PathBuf> = [0, 1, 2, 2, 0].map(|n| slot(n).into()).into();
        assert_eq!(written, expected);
    }

    // ARQ-51
    #[test]
    fn ac7_a_failed_job_is_retried_one_interval_later_never_on_the_next_tick() {
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        let job = rig.due_at(T0 + INTERVAL).expect("a job");
        rig.fs.fail_at(&[0]);
        let failed = T0 + secs(11);
        rig.finish_at(failed, job);
        rig.fs.fail_at(&[]);
        assert!(rig.due_at(failed).is_none(), "the next tick");
        assert!(
            rig.due_at(failed + INTERVAL - STEP).is_none(),
            "one step before"
        );
        let retry = rig.due_at(failed + INTERVAL).expect("the retry");
        assert_eq!(retry.slot_path(), PathBuf::from(slot(0)), "the same slot");
    }

    #[test]
    fn ac8_an_outcome_of_a_replaced_session_is_ignored() {
        // Session 0 (open) leaves a job that succeeds, session 1 (new) one
        // that fails; session 2 (open) has a job in flight and a change
        // pending since T0 + 31.
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        let stale_ok = rig.due_at(T0 + secs(10)).expect("session 0's job");
        rig.engine
            .projects()
            .new_document()
            .expect("a new document");
        rig.edit_at(T0 + secs(10));
        let stale_err = rig.due_at(T0 + secs(20)).expect("session 1's job");
        rig.open();
        rig.edit_at(T0 + secs(20));
        let current = rig.due_at(T0 + secs(30)).expect("session 2's job");
        rig.edit_at(T0 + secs(31));

        rig.finish_at(T0 + secs(32), stale_ok);
        rig.fs.take_trace();
        rig.fs.fail_at(&[0]);
        rig.finish_at(T0 + secs(32), stale_err);
        rig.fs.fail_at(&[]);

        assert!(rig.due_at(T0 + secs(41)).is_none(), "still in flight");
        rig.finish_at(T0 + secs(41), current);
        let next = rig.due_at(T0 + secs(41)).expect("pending since T0 + 31");
        assert_eq!(next.slot_path(), PathBuf::from(slot_of(2, 1)), "slot 1");
    }

    #[test]
    fn ac9_an_autosave_never_clears_modified_never_bumps_and_publishes_nothing() {
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        assert_eq!(drain(&rig.engine).len(), 1, "the edit");
        let job = rig.due_at(T0 + INTERVAL).expect("a job");
        let outcome = job.run();
        assert!(outcome.error().is_none(), "the run");
        rig.engine.projects().autosave_finished(outcome);
        assert!(rig.engine.projects().is_modified(), "still modified");
        assert_eq!(drain(&rig.engine), Vec::new(), "nothing published");
        let page = rig.edit_at(T0 + INTERVAL);
        let expected = EngineEvent::DocumentChanged {
            version: DocVersion(3),
            dirty: pages(&[page]),
        };
        assert_eq!(drain(&rig.engine), vec![expected], "no bump in between");
    }

    // ARQ-50, ARQ-51
    #[test]
    fn ac10_a_fault_at_every_operation_of_a_run_keeps_the_old_slot_and_removes_the_temporary() {
        let previous = b"the previous slot".to_vec();
        let prepared = || {
            let mut rig = Rig::opened(false);
            rig.edit_at(T0);
            rig.fs.put(&slot(0), &previous);
            let new = rig.bytes();
            let job = rig.due_at(T0 + INTERVAL).expect("a job");
            (rig, job, new)
        };
        let (rig, job, new) = prepared();
        assert!(job.run().error().is_none(), "the run");
        let trace = rig.fs.take_trace();
        assert_eq!(trace, job_trace(0, new.len()));
        assert_eq!(rig.fs.file(&slot(0)), Some(new), "the new bytes in full");

        // Indices 0-5 write the slot; 6-9 the marker, after the slot is in
        // place, so a fault there keeps the new slot whole.
        for fault in 0..trace.len() {
            let (rig, job, new) = prepared();
            rig.fs.fail_at(&[fault]);
            assert!(job.run().error().is_some(), "fault at {fault}");
            let mut expected = trace[..=fault].to_vec();
            if fault >= 6 {
                let folder = Path::new(&slot(0)).parent().expect("a folder").to_owned();
                expected.push(FsOp::Remove(folder.join("latest.tmp")));
            }
            expected.push(FsOp::Remove(tmp(0)));
            assert_eq!(rig.fs.take_trace(), expected, "fault at {fault}");
            let kept = rig.fs.file(&slot(0));
            let whole = if fault >= 6 { new } else { previous.clone() };
            assert_eq!(kept, Some(whole), "fault at {fault}");
        }
    }

    // ARQ-51
    #[test]
    fn an_undo_alone_after_the_last_snapshot_makes_a_job_due_one_interval_after_it() {
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        let job = rig.due_at(T0 + INTERVAL).expect("the edit's job");
        rig.finish_at(T0 + secs(11), job);
        assert!(rig.due_at(T0 + secs(60)).is_none(), "nothing pending");

        rig.at(T0 + secs(70));
        assert!(rig.engine.edit().undo().expect("an undo").is_some());
        assert!(
            rig.due_at(T0 + secs(80) - STEP).is_none(),
            "one step before"
        );
        let job = rig
            .due_at(T0 + secs(80))
            .expect("one interval after the undo");
        assert_eq!(job.version(), DocVersion(3), "the edit, then the undo");
    }

    // ARQ-51
    #[test]
    fn a_redo_alone_after_the_last_snapshot_makes_a_job_due_one_interval_after_it() {
        let mut rig = Rig::opened(false);
        rig.edit_at(T0);
        assert!(rig.engine.edit().undo().expect("an undo").is_some());
        let job = rig.due_at(T0 + INTERVAL).expect("the job of both");
        rig.finish_at(T0 + secs(11), job);
        assert!(rig.due_at(T0 + secs(60)).is_none(), "nothing pending");

        rig.at(T0 + secs(70));
        assert!(rig.engine.edit().redo().expect("a redo").is_some());
        assert!(
            rig.due_at(T0 + secs(80) - STEP).is_none(),
            "one step before"
        );
        let job = rig
            .due_at(T0 + secs(80))
            .expect("one interval after the redo");
        assert_eq!(job.version(), DocVersion(4), "the edit, undo, then redo");
    }

    // ARQ-55
    #[test]
    fn ac11_a_read_only_session_still_autosaves() {
        let mut rig = Rig::opened(true);
        rig.edit_at(T0);
        let new = rig.bytes();
        let job = rig.due_at(T0 + INTERVAL).expect("a job");
        assert!(job.run().error().is_none(), "the run");
        assert_eq!(rig.fs.file(&slot(0)), Some(new), "the slot");
        let old = two_page_document().to_json().expect("json").into_bytes();
        assert_eq!(rig.fs.file(TARGET), Some(old), "the user's file");
    }

    #[test]
    fn ac12_a_job_and_its_outcome_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AutosaveJob>();
        assert_send::<AutosaveOutcome>();
    }
}
