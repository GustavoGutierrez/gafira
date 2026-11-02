//! `check_progress_contract` against a conforming fake and one fake per
//! violation (ARQ-112).

use std::sync::atomic::{AtomicUsize, Ordering};

use gafira_doc::contract::{CancelToken, Progress, checkpoint};
use gafira_testkit::progress::{ProgressLog, ProgressViolation as V, check_progress_contract};

/// How a `Fake` behaves in a cancelled run that is bad at `k`.
#[derive(Clone, Copy)]
enum Mode {
    /// Keeps reporting to the end and returns success.
    Ignore,
    /// Stops and returns success.
    StopOk,
    /// Reports the values, then returns cancelled.
    Extra(fn(usize, usize) -> Vec<f32>),
    /// Reports nothing in any run after the first and returns cancelled.
    Silent,
}

/// `n` even steps through `checkpoint`; in a cancelled run it is honest unless
/// `bad(k)`.
struct Fake {
    n: usize,
    bad: fn(usize) -> bool,
    mode: Mode,
    calls: AtomicUsize,
}

impl Fake {
    fn honest(n: usize) -> Self {
        Fake {
            n,
            bad: |_| false,
            mode: Mode::StopOk,
            calls: AtomicUsize::new(0),
        }
    }
    fn bad(n: usize, bad: fn(usize) -> bool, mode: Mode) -> Self {
        Fake {
            n,
            bad,
            mode,
            calls: AtomicUsize::new(0),
        }
    }
    fn frac(&self, i: usize) -> f32 {
        i as f32 / self.n as f32
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
    fn run(&self, t: &CancelToken, p: &dyn Progress) -> Result<bool, ()> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if matches!(self.mode, Mode::Silent) && call > 0 {
            return Ok(true);
        }
        for i in 1..=self.n {
            if checkpoint(t, p, self.frac(i)).is_err() {
                let k = i - 1;
                if !(self.bad)(k) {
                    return Ok(true);
                }
                return match self.mode {
                    Mode::Ignore => {
                        for j in i..=self.n {
                            p.report(self.frac(j));
                        }
                        Ok(false)
                    }
                    Mode::StopOk => Ok(false),
                    Mode::Extra(extra) => {
                        for x in extra(k, self.n) {
                            p.report(x);
                        }
                        Ok(true)
                    }
                    Mode::Silent => Ok(true),
                };
            }
        }
        Ok(false)
    }
}

fn check(fake: &Fake, max_step: f32) -> Result<(), V> {
    check_progress_contract(max_step, |t, p| fake.run(t, p)).unwrap()
}

/// Reports `seq` through `checkpoint`; honest when cancelled.
fn check_seq(seq: &[f32], max_step: f32) -> Result<(), V> {
    check_progress_contract::<()>(max_step, |t, p| {
        for &f in seq {
            if checkpoint(t, p, f).is_err() {
                return Ok(true);
            }
        }
        Ok(false)
    })
    .unwrap()
}

// ARQ-112
#[test]
fn arq_112_progress_log_records_fractions_in_order() {
    let log = ProgressLog::new();
    assert!(log.fractions().is_empty());
    for f in [0.5, 0.25, f32::NAN, 1.0] {
        log.report(f);
    }
    let got: Vec<u32> = log.fractions().iter().map(|f| f.to_bits()).collect();
    let want: Vec<u32> = [0.5f32, 0.25, f32::NAN, 1.0]
        .iter()
        .map(|f| f.to_bits())
        .collect();
    assert_eq!(got, want);
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProgressLog>();
}

// ARQ-112
#[test]
fn arq_112_a_conforming_operation_passes_for_every_report_count() {
    // More than 50 even steps at 0.02: f32 rounding of i / 50 exceeds it.
    for n in [1, 2, 3, 4, 5, 6, 59, 60, 61, 100] {
        let fake = Fake::honest(n);
        let max_step = if n > 50 { 0.02 } else { 1.0 };
        assert_eq!(check(&fake, max_step), Ok(()), "n = {n}");
    }
}

// ARQ-112
#[test]
fn arq_112_cancel_points_are_1_half_and_the_one_before_last() {
    // runs = 1 baseline + the distinct points in 1..=n-1
    for (n, runs) in [
        (0, 1),
        (1, 1),
        (2, 2),
        (3, 3),
        (4, 4),
        (5, 4),
        (6, 4),
        (60, 4),
    ] {
        let fake = Fake::honest(n);
        let seq: Vec<f32> = (1..=n).map(|i| fake.frac(i)).collect();
        let _ = check_progress_contract::<()>(1.0, |t, p| {
            fake.calls.fetch_add(1, Ordering::SeqCst);
            for &f in &seq {
                if checkpoint(t, p, f).is_err() {
                    return Ok(true);
                }
            }
            Ok(false)
        });
        assert_eq!(fake.calls(), runs, "n = {n}");
    }
}

// ARQ-112
#[test]
fn arq_112_only_the_three_cancel_points_are_tried() {
    // n = 60: points 1, 30, 59. A fault elsewhere is not seen; at a point it is.
    let ignore = |bad: fn(usize) -> bool| Fake::bad(60, bad, Mode::StopOk);
    for k in [2, 3, 29, 31, 58] {
        // `bad` takes a plain fn, so one fn per k.
        let fake = match k {
            2 => ignore(|k| k == 2),
            3 => ignore(|k| k == 3),
            29 => ignore(|k| k == 29),
            31 => ignore(|k| k == 31),
            _ => ignore(|k| k == 58),
        };
        assert_eq!(check(&fake, 0.02), Ok(()), "bad only at {k}");
    }
    assert_eq!(
        check(&ignore(|k| k == 1), 0.02),
        Err(V::OutputAfterCancel { k: 1 })
    );
    assert_eq!(
        check(&ignore(|k| k == 30), 0.02),
        Err(V::OutputAfterCancel { k: 30 })
    );
    assert_eq!(
        check(&ignore(|k| k == 59), 0.02),
        Err(V::OutputAfterCancel { k: 59 })
    );
    // Odd n = 61: n / 2 = 30.
    let odd = Fake::bad(61, |k| k == 30, Mode::StopOk);
    assert_eq!(check(&odd, 0.02), Err(V::OutputAfterCancel { k: 30 }));
    let odd = Fake::bad(61, |k| k == 31, Mode::StopOk);
    assert_eq!(check(&odd, 0.02), Ok(()));
}

// ARQ-112
#[test]
fn arq_112_the_first_cancel_point_in_order_is_reported() {
    let fake = Fake::bad(60, |k| k == 1 || k == 30 || k == 59, Mode::StopOk);
    assert_eq!(check(&fake, 0.02), Err(V::OutputAfterCancel { k: 1 }));
    let fake = Fake::bad(60, |k| k == 30 || k == 59, Mode::StopOk);
    assert_eq!(check(&fake, 0.02), Err(V::OutputAfterCancel { k: 30 }));
}

// ARQ-112
#[test]
fn arq_112_decreasing_is_strict() {
    assert_eq!(
        check_seq(&[0.2, 0.5, 0.4, 1.0], 1.0),
        Err(V::Decreasing { at: 2 })
    );
    assert_eq!(
        check_seq(&[0.5, 0.25, 1.0], 1.0),
        Err(V::Decreasing { at: 1 })
    );
    assert_eq!(
        check_seq(&[0.5, 0.5f32.next_down(), 1.0], 1.0),
        Err(V::Decreasing { at: 1 })
    );
    assert_eq!(check_seq(&[0.2, 0.5, 0.5, 1.0], 1.0), Ok(()));
    assert_eq!(check_seq(&[0.0, 0.0, 1.0], 1.0), Ok(()));
}

// ARQ-112
#[test]
fn arq_112_out_of_range_includes_nan_and_is_inclusive_at_0_and_1() {
    let cases: [(&[f32], Result<(), V>); 11] = [
        (&[0.0, 0.5, 1.0], Ok(())),
        (&[-0.0, 1.0], Ok(())),
        (&[0.2, -0.1, 1.0], Err(V::OutOfRange { at: 1 })),
        (&[-f32::MIN_POSITIVE, 1.0], Err(V::OutOfRange { at: 0 })),
        (&[0.5, 1.5, 1.0], Err(V::OutOfRange { at: 1 })),
        (&[0.5, 1.0f32.next_up()], Err(V::OutOfRange { at: 1 })),
        (&[0.1, f32::NAN, 1.0], Err(V::OutOfRange { at: 1 })),
        (&[f32::NAN, 1.0], Err(V::OutOfRange { at: 0 })),
        (&[0.5, f32::INFINITY], Err(V::OutOfRange { at: 1 })),
        (&[f32::NEG_INFINITY, 1.0], Err(V::OutOfRange { at: 0 })),
        // Range is judged before decreasing and before early completion.
        (&[0.5, -1.0], Err(V::OutOfRange { at: 1 })),
    ];
    for (seq, want) in cases {
        assert_eq!(check_seq(seq, 1.0), want, "{seq:?}");
    }
    assert_eq!(
        check_seq(&[1.5, 1.0], 1.0),
        Err(V::OutOfRange { at: 0 }),
        "before early completion"
    );
}

// ARQ-112
#[test]
fn arq_112_one_before_the_last_report_is_early_completion() {
    assert_eq!(
        check_seq(&[0.5, 1.0, 1.0], 1.0),
        Err(V::EarlyCompletion { at: 1 })
    );
    assert_eq!(
        check_seq(&[1.0, 1.0], 1.0),
        Err(V::EarlyCompletion { at: 0 })
    );
    assert_eq!(
        check_seq(&[1.0, 0.5, 1.0], 1.0),
        Err(V::EarlyCompletion { at: 0 })
    );
    assert_eq!(check_seq(&[0.5, 1.0], 1.0), Ok(()));
    assert_eq!(check_seq(&[1.0], 1.0), Ok(()));
}

// ARQ-112
#[test]
fn arq_112_not_completed_when_empty_or_not_ending_at_1() {
    assert_eq!(check_seq(&[], 1.0), Err(V::NotCompleted));
    assert_eq!(check_seq(&[0.5, 0.9], 1.0), Err(V::NotCompleted));
    assert_eq!(
        check_seq(&[0.5, 1.0f32.next_down()], 1.0),
        Err(V::NotCompleted)
    );
    assert_eq!(check_seq(&[0.0], 1.0), Err(V::NotCompleted));
}

// ARQ-112
#[test]
fn arq_112_a_baseline_that_reports_cancelled_is_not_completed() {
    let r = check_progress_contract::<()>(1.0, |_, p| {
        p.report(-1.0); // would be OutOfRange; the return value is judged first
        Ok(true)
    })
    .unwrap();
    assert_eq!(r, Err(V::NotCompleted));
}

// ARQ-112
#[test]
fn arq_112_step_too_large_is_strict_without_tolerance() {
    // Exactly the limit passes.
    assert_eq!(check_seq(&[0.0, 0.5, 1.0], 0.5), Ok(()));
    assert_eq!(
        check_seq(&[0.0, 0.5, 1.0], 0.4),
        Err(V::StepTooLarge { at: 1 })
    );
    assert_eq!(
        check_seq(&[0.0, 0.5f32.next_up(), 1.0], 0.5),
        Err(V::StepTooLarge { at: 1 })
    );
    assert_eq!(
        check_seq(&[0.25, 0.5, 0.75, 1.0], 0.25),
        Ok(()),
        "the first report is not compared to 0"
    );
    assert_eq!(check_seq(&[0.9, 1.0], 0.2), Ok(()));
    assert_eq!(check_seq(&[1.0], 0.0), Ok(()));
    assert_eq!(
        check_seq(&[0.1, 0.2, 0.9, 1.0], 0.3),
        Err(V::StepTooLarge { at: 2 })
    );
}

// ARQ-112
#[test]
fn arq_112_a_scan_fault_comes_before_not_completed_and_in_index_order() {
    assert_eq!(
        check_seq(&[0.0, 0.9], 0.5),
        Err(V::StepTooLarge { at: 1 }),
        "not 1.0 at the end too, the step is found first"
    );
    assert_eq!(
        check_seq(&[0.5, 0.4, 0.9, 0.1], 0.1),
        Err(V::Decreasing { at: 1 })
    );
    // Same index: early completion is judged before the step.
    assert_eq!(
        check_seq(&[0.0, 1.0, 1.0], 0.5),
        Err(V::EarlyCompletion { at: 1 })
    );
}

// ARQ-112
#[test]
fn arq_112_not_cancelled_when_the_operation_ignores_the_token() {
    for (bad, k) in [
        (Fake::bad(60, |k| k == 1, Mode::Ignore), 1),
        (Fake::bad(60, |k| k == 30, Mode::Ignore), 30),
        (Fake::bad(60, |k| k == 59, Mode::Ignore), 59),
    ] {
        assert_eq!(check(&bad, 0.02), Err(V::NotCancelled { k }));
    }
}

// ARQ-112
#[test]
fn arq_112_not_cancelled_when_the_cancel_never_fired() {
    // Fewer than k reports in the cancelled run, and it still says cancelled.
    let fake = Fake::bad(60, |_| true, Mode::Silent);
    assert_eq!(check(&fake, 0.02), Err(V::NotCancelled { k: 1 }));
}

// ARQ-112
#[test]
fn arq_112_output_after_cancel_when_it_stops_early_and_returns_success() {
    let fake = Fake::bad(60, |k| k == 30, Mode::StopOk);
    assert_eq!(check(&fake, 0.02), Err(V::OutputAfterCancel { k: 30 }));
}

// ARQ-112
#[test]
fn arq_112_reports_after_cancel_allows_one_and_names_extra() {
    let one: fn(usize, usize) -> Vec<f32> = |k, n| vec![k as f32 / n as f32];
    let two: fn(usize, usize) -> Vec<f32> = |k, n| vec![k as f32 / n as f32; 2];
    let three: fn(usize, usize) -> Vec<f32> = |k, n| vec![k as f32 / n as f32; 3];
    assert_eq!(
        check(&Fake::bad(60, |_| true, Mode::Extra(one)), 0.02),
        Ok(())
    );
    for (extra, f) in [(2, two), (3, three)] {
        for k in [1, 30, 59] {
            let fake = match k {
                1 => Fake::bad(60, |k| k == 1, Mode::Extra(f)),
                30 => Fake::bad(60, |k| k == 30, Mode::Extra(f)),
                _ => Fake::bad(60, |k| k == 59, Mode::Extra(f)),
            };
            assert_eq!(
                check(&fake, 0.02),
                Err(V::ReportsAfterCancel { k, extra }),
                "k = {k}"
            );
        }
    }
}

// ARQ-112
#[test]
fn arq_112_a_cancelled_run_is_scanned_for_range_order_and_completion_only() {
    let drop: fn(usize, usize) -> Vec<f32> = |k, n| vec![(k - 1) as f32 / n as f32];
    let range: fn(usize, usize) -> Vec<f32> = |_, _| vec![2.0];
    let nan: fn(usize, usize) -> Vec<f32> = |_, _| vec![f32::NAN];
    let one: fn(usize, usize) -> Vec<f32> = |_, _| vec![1.0];
    let jump: fn(usize, usize) -> Vec<f32> = |_, _| vec![0.9];
    let same: fn(usize, usize) -> Vec<f32> = |k, n| vec![k as f32 / n as f32];
    let at = |f| Fake::bad(60, |k| k == 30, Mode::Extra(f));
    assert_eq!(check(&at(drop), 0.02), Err(V::Decreasing { at: 30 }));
    assert_eq!(check(&at(range), 0.02), Err(V::OutOfRange { at: 30 }));
    assert_eq!(check(&at(nan), 0.02), Err(V::OutOfRange { at: 30 }));
    assert_eq!(check(&at(one), 0.02), Err(V::EarlyCompletion { at: 30 }));
    assert_eq!(
        check(&at(jump), 0.02),
        Ok(()),
        "no step limit after a cancel"
    );
    assert_eq!(
        check(&at(same), 0.02),
        Ok(()),
        "an equal fraction is not a drop"
    );
    // Not ending at 1.0 is no fault in a cancelled run (checked above: Ok).
}

// ARQ-112
#[test]
fn arq_112_a_cancelled_run_that_reaches_1_is_early_completion_even_as_last() {
    // Cancel at k = 59 of 60, then one extra report of 1.0: it is the last report.
    let one: fn(usize, usize) -> Vec<f32> = |_, _| vec![1.0];
    let fake = Fake::bad(60, |k| k == 59, Mode::Extra(one));
    assert_eq!(check(&fake, 0.02), Err(V::EarlyCompletion { at: 59 }));
}

// ARQ-112
#[test]
fn arq_112_an_err_of_op_is_returned_at_once_from_any_run() {
    let calls = AtomicUsize::new(0);
    let r = check_progress_contract(1.0, |_, _| -> Result<bool, &str> {
        calls.fetch_add(1, Ordering::SeqCst);
        Err("boom")
    });
    assert_eq!(r, Err("boom"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let calls = AtomicUsize::new(0);
    let r = check_progress_contract(1.0, |t, p| -> Result<bool, &str> {
        if calls.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err("second");
        }
        for i in 1..=4 {
            if checkpoint(t, p, i as f32 / 4.0).is_err() {
                return Ok(true);
            }
        }
        Ok(false)
    });
    assert_eq!(r, Err("second"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

// ARQ-112
#[test]
fn arq_112_the_first_violation_ends_the_check() {
    let fake = Fake::honest(60);
    let _ = check_progress_contract::<()>(1.0, |t, p| {
        fake.calls.fetch_add(1, Ordering::SeqCst);
        p.report(0.5);
        p.report(0.4);
        let _ = t;
        Ok(false)
    });
    assert_eq!(fake.calls(), 1);
}

// ARQ-112
#[test]
fn arq_112_even_steps_are_measured_in_f32_with_no_tolerance() {
    // `i / 50` in f32 rises above 0.02f32 at 19 places (design, measured).
    let seq: Vec<f32> = (1..=50).map(|i| i as f32 / 50.0).collect();
    assert!(matches!(check_seq(&seq, 0.02), Err(V::StepTooLarge { .. })));
    let seq: Vec<f32> = (1..=60).map(|i| i as f32 / 60.0).collect();
    assert_eq!(check_seq(&seq, 0.02), Ok(()));
}
