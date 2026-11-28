use super::{native, types::WindowsPower};
use myrig_host_core::shutdown::Power;

// ! Schedules a shutdown ten minutes out and cancels it at once, so the machine is never shut down.
// ! It schedules for real, so it runs only on a test PC, not in default test runs: `cargo test --
// ! --ignored`.
#[test]
#[ignore = "schedules a real shutdown and cancels it; run only on a disposable machine"]
fn a_scheduled_shutdown_is_withdrawn() {
    native::initiate_shutdown("", 600).expect("schedule a shutdown to cancel");
    WindowsPower
        .cancel_shutdown()
        .expect("the scheduled shutdown is withdrawn");
    // Cancelling again has nothing to withdraw, so Windows reports an error.
    assert!(WindowsPower.cancel_shutdown().is_err());
}
