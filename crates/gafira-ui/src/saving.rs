//! The save on its own thread (spec 036): the engine prepares a
//! [`SaveJob`], this module runs it off the window thread, and the shell
//! hands its outcome back to the engine. The window keeps drawing and
//! answering while the file, its thumbnail and its backup are written.

use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;

use egui::RichText;
use gafira_engine::{SaveJob, SaveOutcome};

use crate::style::Theme;
use crate::text::Texts;

/// How a running save ended.
pub(crate) enum Ended {
    /// The job ran; its outcome goes to `ProjectService::save_finished`.
    Ran(SaveOutcome),
    /// The thread stopped without an answer (a panic in a port): a failed
    /// save, never a crash of the window.
    Lost,
}

/// A save running on its thread.
pub(crate) struct Saving {
    receiver: mpsc::Receiver<SaveOutcome>,
    thread: Option<JoinHandle<()>>,
}

impl Saving {
    /// How the save ended, once it did.
    pub(crate) fn poll(&mut self) -> Option<Ended> {
        let ended = match self.receiver.try_recv() {
            Ok(outcome) => Ended::Ran(outcome),
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Ended::Lost,
        };
        self.join();
        Some(ended)
    }

    /// Wait for the save to end: the window is closing, and a save is never
    /// cut short.
    pub(crate) fn wait(mut self) -> Ended {
        let ended = match self.receiver.recv() {
            Ok(outcome) => Ended::Ran(outcome),
            Err(_) => Ended::Lost,
        };
        self.join();
        ended
    }

    fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Run `job` on a new thread; `wake` is called when it ends.
pub(crate) fn spawn(job: SaveJob, wake: Arc<dyn Fn() + Send + Sync>) -> Saving {
    let (sender, receiver) = mpsc::channel();
    let thread = std::thread::Builder::new()
        .name("gafira-save".to_owned())
        .spawn(move || {
            let _ = sender.send(job.run());
            wake();
        })
        .ok();
    // No thread: the sender went with the closure, so the receiver reads
    // as disconnected, a failed save.
    Saving { receiver, thread }
}

/// The card of a running save, over the bottom right of the window and
/// never modal (03 §8.1).
pub(crate) fn card(ctx: &egui::Context, theme: &Theme, texts: &Texts) {
    egui::Area::new(egui::Id::new("save-progress"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -48.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(280.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new(texts.t(&gafira_doc::msg!("file.saving")))
                            .font(theme.font("ui-label")),
                    );
                });
            });
        });
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use gafira_engine::SaveRequest;

    use super::*;
    use crate::format::StartFormat;
    use crate::shell::start;

    fn finish(saving: &mut Saving) -> Ended {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(ended) = saving.poll() {
                return ended;
            }
            assert!(Instant::now() < deadline, "the save ends");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // S36-06: the save runs on its own thread and the engine accounts for
    // it on the window's: the file appears and the session is clean.
    #[test]
    fn s36_06_the_save_runs_off_the_window_thread() {
        let mut engine =
            crate::editor::tests::engine_over(Arc::new(gafira_format::GafRepository::default()));
        start(&mut engine, StartFormat::Social).expect("a document");
        let dir = std::env::temp_dir().join(format!("gafira-ui-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join("cartel.gaf");
        let job = engine
            .projects()
            .prepare_save(SaveRequest::SaveAs(path.clone()))
            .expect("prepares");
        let woken = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&woken);
        let window = std::thread::current().id();
        let mut saving = spawn(
            job,
            Arc::new(move || {
                assert_ne!(std::thread::current().id(), window, "not the window thread");
                count.fetch_add(1, Ordering::Relaxed);
            }),
        );
        let result = match finish(&mut saving) {
            Ended::Ran(outcome) => engine.projects().save_finished(outcome),
            Ended::Lost => panic!("the thread answered"),
        };
        let exists = path.exists();
        let adopted = engine.projects().path();
        let modified = engine.projects().is_modified();
        engine.projects().close();
        let _ = std::fs::remove_dir_all(&dir);
        result.expect("saved");
        assert!(exists, "the file");
        assert_eq!(adopted.as_deref(), Some(Path::new(&path)));
        assert!(!modified, "clean once accounted for");
        assert_eq!(woken.load(Ordering::Relaxed), 1);
    }

    // S36-07: a thread that dies without an answer is a lost save.
    #[test]
    fn s36_07_a_thread_that_panics_is_a_lost_save() {
        let (sender, receiver) = mpsc::channel::<SaveOutcome>();
        let thread = std::thread::spawn(move || {
            let _keep = sender;
            panic!("a repository bug");
        });
        let mut saving = Saving {
            receiver,
            thread: Some(thread),
        };
        assert!(matches!(finish(&mut saving), Ended::Lost));
    }
}
