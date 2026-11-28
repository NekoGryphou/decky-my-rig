use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};
use uuid::Uuid;
struct Recorder(Arc<AtomicUsize>, bool);
const DENIED: u32 = 0x8007_0005;
impl Power for Recorder {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if self.1 {
            Err(PowerError { code: DENIED })
        } else {
            Ok(())
        }
    }

    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        Ok(())
    }
}
#[test]
fn duplicates_concurrency_and_deadline() {
    let calls = Arc::new(AtomicUsize::new(0));
    let core = Arc::new(Core::new(Box::new(Recorder(calls.clone(), false))));
    let now = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..16 {
            let core = &core;
            scope.spawn(move || assert_eq!(core.shutdown(Uuid::nil(), now), Ok(())));
        }
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        core.shutdown(Uuid::max(), now),
        Err(ShutdownError::InProgress)
    );
    assert!(core.shutting_down(now + Duration::from_secs(119)).unwrap());
    assert!(!core.shutting_down(now + PENDING_TIMEOUT).unwrap());
    assert_eq!(core.shutdown(Uuid::max(), now + PENDING_TIMEOUT), Ok(()));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
#[test]
fn refusal_does_not_leave_pending() {
    let calls = Arc::new(AtomicUsize::new(0));
    let core = Core::new(Box::new(Recorder(calls.clone(), true)));
    for _ in 0..2 {
        assert_eq!(
            core.shutdown(Uuid::nil(), Instant::now()),
            Err(ShutdownError::Failed(PowerError { code: DENIED }))
        );
    }
    assert!(!core.shutting_down(Instant::now()).unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
struct Blocking {
    entered: std::sync::mpsc::SyncSender<()>,
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}
impl Power for Blocking {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        let _ = self.entered.send(());
        let _ = self.release.lock().unwrap().recv();
        Ok(())
    }

    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        Ok(())
    }
}
#[test]
fn status_does_not_wait_for_the_power_call() {
    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let core = Arc::new(Core::new(Box::new(Blocking {
        entered: entered_tx,
        release: std::sync::Mutex::new(release_rx),
    })));
    let now = Instant::now();
    let shutdown = {
        let core = core.clone();
        std::thread::spawn(move || core.shutdown(Uuid::nil(), now))
    };
    entered_rx.recv().unwrap();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    {
        let core = core.clone();
        std::thread::spawn(move || {
            let _ = done_tx.send((core.shutting_down(now), core.shutdown(Uuid::max(), now)));
        });
    }
    let answered = done_rx.recv_timeout(Duration::from_secs(2));
    release_tx.send(()).unwrap();
    assert_eq!(shutdown.join().unwrap(), Ok(()));
    let (status, other) = answered.expect("status blocked on the power call");
    assert_eq!(status, Ok(true));
    assert_eq!(other, Err(ShutdownError::InProgress));
    assert!(core.shutting_down(now).unwrap());
}
struct Panicking;
impl Power for Panicking {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        panic!("power backend panicked");
    }

    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        Ok(())
    }
}
#[test]
fn a_panicking_power_call_does_not_block_later_requests() {
    let core = Core::new(Box::new(Panicking));
    let now = Instant::now();
    let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        core.shutdown(Uuid::nil(), now)
    }));
    assert!(first.is_err());
    assert_eq!(core.shutting_down(now), Ok(false));
}

#[derive(Default)]
struct Cancellable {
    schedules: AtomicUsize,
    cancels: AtomicUsize,
}

impl Power for Arc<Cancellable> {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        self.schedules.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        self.cancels.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn an_expired_shutdown_is_cancelled_so_windows_does_not_power_off_later() {
    // ! The OS shutdown keeps its own timer. Once the host stops tracking the request, the timer
    // ! must be cancelled, or the PC can still power off after the host has reported that it stayed
    // ! up.
    let power = Arc::new(Cancellable::default());
    let core = Core::new(Box::new(power.clone()));
    let now = Instant::now();
    core.shutdown(Uuid::nil(), now).unwrap();
    assert!(!core.shutting_down(now + PENDING_TIMEOUT).unwrap());
    assert_eq!(power.cancels.load(Ordering::SeqCst), 1);
}

#[test]
fn a_shutdown_that_is_still_pending_is_not_cancelled() {
    let power = Arc::new(Cancellable::default());
    let core = Core::new(Box::new(power.clone()));
    let now = Instant::now();
    core.shutdown(Uuid::nil(), now).unwrap();
    assert!(core.shutting_down(now + Duration::from_secs(119)).unwrap());
    assert_eq!(power.cancels.load(Ordering::SeqCst), 0);
}
