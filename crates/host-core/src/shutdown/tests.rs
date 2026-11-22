use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};
use uuid::Uuid;
struct Recorder(Arc<AtomicUsize>, bool);
impl Power for Recorder {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if self.1 { Err(PowerError) } else { Ok(()) }
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
            Err(ShutdownError::Failed)
        );
    }
    assert!(!core.shutting_down(Instant::now()).unwrap());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
