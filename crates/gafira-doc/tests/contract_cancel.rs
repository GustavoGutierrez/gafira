//! `CancelToken` and `Cancelled` (ARQ-112, ADR 0012, C4).

use gafira_doc::contract::{CancelToken, Cancelled};
use gafira_doc::msg::{Message, MessageKey, ToMessage, is_valid_key};

fn assert_send_sync_clone<T: Send + Sync + Clone>() {}

// ARQ-112
#[test]
fn arq_112_token_is_send_sync_and_clone() {
    assert_send_sync_clone::<CancelToken>();
}

// ARQ-112
#[test]
fn arq_112_a_new_token_is_live_and_checks_ok() {
    let t = CancelToken::new();
    assert!(!t.is_cancelled());
    assert_eq!(t.check(), Ok(()));
    assert!(!CancelToken::default().is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_cancel_is_seen_by_the_token_and_by_its_clones() {
    let t = CancelToken::new();
    let c = t.clone();
    let c2 = c.clone();
    c.cancel();
    for x in [&t, &c, &c2] {
        assert!(x.is_cancelled());
        assert_eq!(x.check(), Err(Cancelled));
    }
}

// ARQ-112
#[test]
fn arq_112_a_clone_made_after_cancel_is_cancelled() {
    let t = CancelToken::new();
    t.cancel();
    assert!(t.clone().is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_a_clone_shares_the_flag_in_both_directions() {
    let t = CancelToken::new();
    let c = t.clone();
    t.cancel();
    assert!(c.is_cancelled());
    let u = CancelToken::new();
    let d = u.clone();
    d.cancel();
    assert!(u.is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_next_generation_cancels_older_tokens_and_is_live() {
    let t0 = CancelToken::new();
    let c0 = t0.clone();
    let t1 = t0.next_generation();
    assert!(t0.is_cancelled());
    assert!(c0.is_cancelled());
    assert!(!t1.is_cancelled());
    assert_eq!(t1.check(), Ok(()));
    let t2 = t1.next_generation();
    assert!(t0.is_cancelled(), "two generations back");
    assert!(t1.is_cancelled());
    assert!(!t2.is_cancelled());
    let t3 = t2.next_generation();
    assert!(t0.is_cancelled() && t1.is_cancelled() && t2.is_cancelled());
    assert!(!t3.is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_next_generation_from_a_stale_token_is_live_and_cancels_the_rest() {
    let t0 = CancelToken::new();
    let t1 = t0.next_generation();
    let t2 = t0.next_generation();
    assert!(t0.is_cancelled());
    assert!(t1.is_cancelled());
    assert!(!t2.is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_a_clone_of_a_new_generation_is_live() {
    let t1 = CancelToken::new().next_generation();
    let c = t1.clone();
    assert!(!c.is_cancelled());
    t1.cancel();
    assert!(c.is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_a_stale_cancel_leaves_the_new_generation_live() {
    let t0 = CancelToken::new();
    let t1 = t0.next_generation();
    t0.cancel();
    assert!(!t1.is_cancelled());
    t0.cancel();
    assert!(!t1.is_cancelled());
    let t2 = t1.next_generation();
    t1.cancel();
    t0.cancel();
    assert!(!t2.is_cancelled(), "stale handles of two generations");
}

// ARQ-112
#[test]
fn arq_112_cancel_twice_does_not_skip_a_generation() {
    let t0 = CancelToken::new();
    t0.cancel();
    t0.cancel();
    t0.cancel();
    let t1 = t0.next_generation();
    assert!(!t1.is_cancelled());
    let u0 = CancelToken::new();
    let u1 = u0.next_generation();
    u1.cancel();
    u1.cancel();
    assert!(u1.is_cancelled());
    let u2 = u1.next_generation();
    assert!(!u2.is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_cancel_of_the_current_generation_leaves_no_live_token_behind() {
    let t0 = CancelToken::new();
    let t1 = t0.next_generation();
    t1.cancel();
    assert!(t0.is_cancelled() && t1.is_cancelled());
    let t2 = t1.next_generation();
    assert!(!t2.is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_independent_tokens_do_not_affect_each_other() {
    let a = CancelToken::new();
    let b = CancelToken::new();
    a.cancel();
    assert!(!b.is_cancelled());
    let _ = a.next_generation();
    assert!(!b.is_cancelled());
}

// ARQ-112
#[test]
fn arq_112_cancel_is_seen_across_threads() {
    let t = CancelToken::new();
    let c = t.clone();
    std::thread::scope(|s| {
        s.spawn(move || c.cancel());
    });
    assert!(t.is_cancelled());
}

// ARQ-102
#[test]
fn arq_102_cancelled_is_the_job_key_with_no_arguments() {
    let m = Cancelled.to_message();
    assert_eq!(
        m,
        Message {
            key: MessageKey("error.job.cancelled"),
            args: vec![]
        }
    );
    assert!(is_valid_key(m.key.0));
}
