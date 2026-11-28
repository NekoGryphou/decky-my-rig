use super::{native, types::WindowsPower};
use crate::event_log::report_shutdown_failure;
use myrig_host_core::shutdown::{Power, PowerError, SHUTDOWN_GRACE};

/// HRESULT for an argument the shutdown call cannot take (E_INVALIDARG).
const INVALID_ARGUMENT: u32 = 0x8007_0057;

impl Power for WindowsPower {
    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        native::abort_shutdown().map_err(|error| {
            let error = PowerError {
                code: error.code().0 as u32,
            };
            // ? Logged for the admin; the caller ignores the result.
            report_shutdown_failure(error.code);
            error
        })
    }

    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        let result = u32::try_from(SHUTDOWN_GRACE.as_secs())
            .map_err(|_| PowerError {
                code: INVALID_ARGUMENT,
            })
            .and_then(|grace| {
                native::initiate_shutdown("", grace).map_err(|error| PowerError {
                    // * Bit-for-bit reinterpretation of the HRESULT.
                    code: error.code().0 as u32,
                })
            });
        // ? The HTTP reply stays generic, so the cause is only in the event log.
        if let Err(error) = &result {
            report_shutdown_failure(error.code);
        }
        result
    }
}
