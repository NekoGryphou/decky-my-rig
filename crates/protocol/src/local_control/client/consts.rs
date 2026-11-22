pub const UNAUTHORIZED_ERROR: &str = "unauthorized";
/// Error codes the service sends. Anything else is an invalid reply.
pub(super) const HOST_ERRORS: [&str; 7] = [
    "invalid_request",
    "wrong_host",
    "storage_failed",
    "storage_conflict",
    "recovery_required",
    "control_failed",
    "unavailable",
];
