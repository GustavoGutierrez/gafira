//! `Progress`, `NoProgress` and `checkpoint` (ARQ-112, C3).

use std::sync::Mutex;

use gafira_doc::contract::{CancelToken, Cancelled, NoProgress, Progress, checkpoint};

/// Records the bits of every fraction, and cancels `token` inside report `at`.
struct Probe<'a> {
    seen: Mutex<Vec<u32>>,
    cancel_at: Option<(usize, &'a CancelToken)>,
}

impl<'a> Probe<'a> {
    fn new() -> Self {
        Probe {
            seen: Mutex::new(Vec::new()),
            cancel_at: None,
        }
    }
    fn cancelling(at: usize, token: &'a CancelToken) -> Self {
        Probe {
            seen: Mutex::new(Vec::new()),
            cancel_at: Some((at, token)),
        }
    }
    fn count(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
}

impl Progress for Probe<'_> {
    fn report(&self, fraction: f32) {
        let mut seen = self.seen.lock().unwrap();
        seen.push(fraction.to_bits());
        if let Some((at, token)) = self.cancel_at
            && seen.len() == at
        {
            token.cancel();
        }
    }
}

// ARQ-112
#[test]
fn arq_112_checkpoint_on_a_live_token_reports_once_and_is_ok() {
    let token = CancelToken::new();
    for f in [0.0, 0.25, 1.0, -0.0, f32::NAN, 2.0] {
        let p = Probe::new();
        assert_eq!(checkpoint(&token, &p, f), Ok(()));
        assert_eq!(*p.seen.lock().unwrap(), vec![f.to_bits()], "{f}");
    }
}

// ARQ-112
#[test]
fn arq_112_checkpoint_on_a_cancelled_token_is_err_and_reports_nothing() {
    let token = CancelToken::new();
    token.cancel();
    let p = Probe::new();
    assert_eq!(checkpoint(&token, &p, 0.5), Err(Cancelled));
    assert_eq!(p.count(), 0);
    let stale = CancelToken::new();
    let _live = stale.next_generation();
    assert_eq!(checkpoint(&stale, &p, 0.5), Err(Cancelled));
    assert_eq!(p.count(), 0);
}

// ARQ-112
#[test]
fn arq_112_a_cancel_inside_report_k_leaves_no_report_after_the_next_checkpoint() {
    for k in 1..=3 {
        let token = CancelToken::new();
        let p = Probe::cancelling(k, &token);
        let mut result = Ok(());
        for i in 0..5 {
            result = checkpoint(&token, &p, i as f32 / 10.0);
            if result.is_err() {
                break;
            }
        }
        assert_eq!(result, Err(Cancelled), "k = {k}");
        assert_eq!(p.count(), k, "k = {k}");
    }
}

// ARQ-112
#[test]
fn arq_112_no_progress_discards_reports_and_is_usable_as_dyn() {
    let p: &dyn Progress = &NoProgress;
    p.report(0.5);
    assert_eq!(checkpoint(&CancelToken::new(), p, 1.0), Ok(()));
    let token = CancelToken::new();
    token.cancel();
    assert_eq!(checkpoint(&token, p, 0.0), Err(Cancelled));
}

fn assert_send_sync<T: Send + Sync>() {}

// ARQ-112
#[test]
fn arq_112_progress_objects_are_send_and_sync() {
    assert_send_sync::<NoProgress>();
    assert_send_sync::<Box<dyn Progress>>();
}
