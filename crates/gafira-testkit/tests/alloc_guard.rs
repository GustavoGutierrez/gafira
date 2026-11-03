//! ARQ-23 negative control: an allocation inside `assert_no_alloc` aborts the
//! process, so the check runs in a subprocess (this same test binary).

use std::hint::black_box;
use std::process::Command;

#[cfg(debug_assertions)]
use assert_no_alloc::AllocDisabler;
use assert_no_alloc::assert_no_alloc;

#[cfg(debug_assertions)]
#[global_allocator]
static GUARD: AllocDisabler = AllocDisabler;

const CASE: &str = "GAFIRA_ALLOC_GUARD_CASE";
const NAME: &str = "arq_23_an_allocation_inside_the_guard_aborts_the_process";

fn child(case: &str) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", NAME, "--test-threads=1"])
        .env(CASE, case)
        .output()
        .unwrap()
        .status
}

#[cfg(debug_assertions)]
#[test]
fn arq_23_an_allocation_inside_the_guard_aborts_the_process() {
    match std::env::var(CASE).as_deref() {
        Ok("alloc") => {
            assert_no_alloc(|| black_box(vec![1_u8; 64]));
            // Unreachable when the guard works: the process has aborted.
            std::process::exit(0);
        }
        Ok("clean") => {
            assert_no_alloc(|| black_box(2 + 2));
        }
        _ => {
            // Control of the control: a clean body in the same subprocess
            // succeeds, so the failure below is the guard and not the setup.
            assert!(child("clean").success(), "the clean child must succeed");
            let status = child("alloc");
            assert!(!status.success(), "an allocation must abort the child");
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                assert_eq!(status.signal(), Some(6), "it must be an abort");
            }
        }
    }
}
