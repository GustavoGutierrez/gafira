//! Notices outside the app, best effort (02 §6.4).

use gafira_ports::system::Notifier;

/// Writes the notice to the standard error until the OS notifications land.
#[derive(Clone, Copy, Debug, Default)]
pub struct StderrNotifier;

impl Notifier for StderrNotifier {
    fn notify(&self, title: &str, body: &str) {
        eprintln!("{title}: {body}");
    }
}
