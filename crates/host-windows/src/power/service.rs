use super::types::WindowsPower;
use myrig_host_core::shutdown::{Power, PowerError, SHUTDOWN_GRACE};

impl Power for WindowsPower {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        let grace = u32::try_from(SHUTDOWN_GRACE.as_secs()).map_err(|_| PowerError)?;
        system_shutdown::shutdown_with_message("", grace, false).map_err(|_| PowerError)
    }
}
