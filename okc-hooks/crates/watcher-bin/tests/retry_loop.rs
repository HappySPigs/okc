//! Regression coverage for autonomous retry and interruptible backoff.

mod common;

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use auth_consent::ConsentDecision;
use change_detect::{Availability, GuardVerdict};
use foundation::{Manifest, TransportError, TransportErrorClass};
use sync_state::{BackoffConfig, JitterMode};
use upload_client::{CommitOutcome, CycleReport, UploadError};
use watcher_bin::adapters::CycleDriver;
use watcher_bin::coordinator::{CycleTrigger, ShutdownFlag};

use common::{Scenario, build_custom, nonempty_manifest};

struct RetryDriver {
    calls: usize,
    observed: mpsc::Sender<Instant>,
    shutdown: ShutdownFlag,
}

impl CycleDriver for RetryDriver {
    fn execute_cycle_traced(&mut self, _: &Manifest) -> CycleReport {
        self.calls += 1;
        self.observed
            .send(Instant::now())
            .expect("observer connected");
        let outcome = if self.calls == 1 {
            Err(UploadError::Transport(TransportError {
                class: TransportErrorClass::Network,
                http_status: None,
                detail: "offline once".into(),
            }))
        } else {
            self.shutdown.set();
            Ok(CommitOutcome {
                server_vault_content_id: None,
                committed: true,
            })
        };
        CycleReport {
            outcome,
            phases: Vec::new(),
        }
    }
}

type LoopHarness = (
    mpsc::Sender<CycleTrigger>,
    mpsc::Receiver<Instant>,
    ShutdownFlag,
    mpsc::Receiver<()>,
    thread::JoinHandle<()>,
);

fn spawn_loop(delay: Duration) -> LoopHarness {
    let (tx, rx) = mpsc::channel();
    let (observed, calls) = mpsc::channel();
    let (done_tx, done) = mpsc::channel();
    let shutdown = ShutdownFlag::new();
    let flag = shutdown.clone();
    let worker = thread::spawn(move || {
        // Build inside the consumer thread: the injected driver intentionally has no Send bound.
        let (mut coordinator, _) = build_custom(
            Scenario {
                paused: false,
                availability: Availability::Reachable,
                verdict: GuardVerdict::Proceed,
                last: None,
                current: nonempty_manifest(),
                consent: ConsentDecision::Permitted,
                driver_succeeds: true,
            },
            Some(Box::new(RetryDriver {
                calls: 0,
                observed,
                shutdown: flag.clone(),
            })),
            BackoffConfig {
                initial_delay: delay,
                multiplier: 2.0,
                cap: delay,
                jitter: JitterMode::None,
            },
        );
        coordinator.run_loop(rx, &flag);
        done_tx.send(()).expect("completion connected");
    });
    (tx, calls, shutdown, done, worker)
}

#[test]
fn transient_failure_retries_without_another_event() {
    let (tx, calls, shutdown, done, worker) = spawn_loop(Duration::from_millis(25));
    tx.send(CycleTrigger::SyncNow).unwrap();
    let first = calls.recv_timeout(Duration::from_secs(2)).unwrap();
    let second = calls.recv_timeout(Duration::from_secs(2));
    shutdown.set();
    done.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.join().unwrap();
    assert!(second.unwrap().duration_since(first) >= Duration::from_millis(25));
}

#[test]
fn edits_during_backoff_do_not_bypass_its_deadline() {
    let delay = Duration::from_millis(120);
    let (tx, calls, shutdown, done, worker) = spawn_loop(delay);
    tx.send(CycleTrigger::SyncNow).unwrap();
    let first = calls.recv_timeout(Duration::from_secs(2)).unwrap();
    for _ in 0..20 {
        tx.send(CycleTrigger::SyncNow).unwrap();
    }
    let second = calls.recv_timeout(Duration::from_secs(2));
    shutdown.set();
    done.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.join().unwrap();
    assert!(second.unwrap().duration_since(first) >= delay);
    assert!(calls.try_recv().is_err());
}

#[test]
fn shutdown_interrupts_a_long_backoff() {
    let (tx, calls, shutdown, done, worker) = spawn_loop(Duration::from_secs(30));
    tx.send(CycleTrigger::SyncNow).unwrap();
    calls.recv_timeout(Duration::from_secs(2)).unwrap();
    shutdown.set();
    done.recv_timeout(Duration::from_secs(1))
        .expect("shutdown must not wait for the 30s backoff");
    worker.join().unwrap();
    assert!(calls.try_recv().is_err());
}

#[cfg(feature = "proptest-support")]
mod properties {
    use super::*;
    use proptest::prelude::*;
    use std::collections::VecDeque;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct ScriptDriver {
        failures: VecDeque<TransportErrorClass>,
        calls: Arc<AtomicUsize>,
        shutdown: ShutdownFlag,
    }

    impl CycleDriver for ScriptDriver {
        fn execute_cycle_traced(&mut self, _: &Manifest) -> CycleReport {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let outcome = if let Some(class) = self.failures.pop_front() {
                Err(UploadError::Transport(TransportError {
                    class,
                    http_status: None,
                    detail: "generated failure".into(),
                }))
            } else {
                self.shutdown.set();
                Ok(CommitOutcome {
                    server_vault_content_id: None,
                    committed: true,
                })
            };
            CycleReport {
                outcome,
                phases: Vec::new(),
            }
        }
    }

    proptest! {
        #[test]
        fn retryable_failure_sequences_reach_success_without_external_events(
            failures in proptest::collection::vec(prop_oneof![
                Just(TransportErrorClass::Network), Just(TransportErrorClass::Timeout),
                Just(TransportErrorClass::Backpressure), Just(TransportErrorClass::ServerError),
            ], 0..16),
        ) {
            let expected = failures.len() + 1;
            let calls = Arc::new(AtomicUsize::new(0));
            let shutdown = ShutdownFlag::new();
            let (mut coordinator, _) = build_custom(Scenario {
                paused: false, availability: Availability::Reachable, verdict: GuardVerdict::Proceed,
                last: None, current: nonempty_manifest(), consent: ConsentDecision::Permitted, driver_succeeds: true,
            }, Some(Box::new(ScriptDriver { failures: failures.into(), calls: calls.clone(), shutdown: shutdown.clone() })),
            BackoffConfig { initial_delay: Duration::from_nanos(1), multiplier: 1.0, cap: Duration::from_nanos(1), jitter: JitterMode::None });
            let (tx, rx) = mpsc::channel();
            tx.send(CycleTrigger::SyncNow).unwrap();
            coordinator.run_loop(rx, &shutdown);
            prop_assert_eq!(calls.load(Ordering::SeqCst), expected);
        }
    }
}
