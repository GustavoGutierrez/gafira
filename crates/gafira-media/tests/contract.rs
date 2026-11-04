//! The contract the crate inherits from the layer-3 rules: the purity of
//! ARQ-02, the cancellation and the progress of ARQ-139 and the determinism
//! of ARQ-140.

mod support;

use std::path::Path;
use std::time::Duration;

use gafira_doc::contract::CancelToken;
use gafira_media::{Medium, Ports};
use gafira_ports::media::{MediaError, StreamIdx};
use gafira_testkit::ports::container::FakeContainer;
use gafira_testkit::progress::check_progress_contract;
use support::{FakeClock, FakeFileSystem, FakePool, MEDIUM};

// ARQ-139: the scan checks the cancel token once per packet, reports a
// fraction that never falls, stays in [0, 1] and reaches 1.0 only on
// success, and the fraction follows the units done over the units planned.
#[test]
fn arq_139_the_scan_follows_the_progress_contract() {
    let pool = FakePool::new(2);
    let clock = FakeClock::new(Duration::from_millis(10));

    let contract = check_progress_contract(0.02, |cancel, progress| {
        let file_system = FakeFileSystem::with_file(MEDIUM, support::reference_medium());
        let ports = Ports {
            file_system: &file_system,
            pool: &pool,
            clock: &clock,
        };
        match Medium::open(
            &ports,
            Path::new(MEDIUM),
            &FakeContainer,
            StreamIdx(0),
            cancel,
            progress,
        ) {
            Ok(_) => Ok(false),
            Err(MediaError::Cancelled(_)) => Ok(true),
            Err(other) => Err(other),
        }
    })
    .expect("the contract check itself does not fail");

    assert_eq!(contract, Ok(()));
}

// ARQ-140: the same input gives the same bytes, and the index does not
// depend on the size of the pool.
#[test]
fn arq_140_the_index_does_not_depend_on_the_pool() {
    let mut expected: Option<Vec<u8>> = None;

    for workers in [1_usize, 2, 4] {
        let pool = FakePool::new(workers);
        let clock = FakeClock::new(Duration::from_millis(10));

        for _ in 0..2 {
            let file_system = FakeFileSystem::with_file(MEDIUM, support::reference_medium());
            let ports = Ports {
                file_system: &file_system,
                pool: &pool,
                clock: &clock,
            };
            let token = CancelToken::new();
            let medium = Medium::open(
                &ports,
                Path::new(MEDIUM),
                &FakeContainer,
                StreamIdx(0),
                &token,
                &gafira_doc::contract::NoProgress,
            )
            .expect("the medium opens");

            assert_eq!(medium.index().len(), 5);
            // The same input is read the same way: one read, no write.
            assert_eq!(file_system.calls(), vec!["open".to_string()]);
            let bytes = medium.index().canonical_bytes();
            match &expected {
                None => expected = Some(bytes),
                Some(first) => assert_eq!(first, &bytes),
            }
        }
    }
}
