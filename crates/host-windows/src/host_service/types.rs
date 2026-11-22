/// Why the service stopped. The SCM gets it as the service-specific exit code.
#[derive(Clone, Copy)]
pub(super) enum ServiceExit {
    /// The protected config couldn't be read or used.
    Config = 1,
    Runtime = 3,
    /// The HTTPS listener or the service loop ended unexpectedly.
    Endpoint = 5,
    /// The SCM didn't accept a status report.
    Status = 6,
    Register = 7,
    Arguments = 8,
    /// The TLS identity is missing, unsafe or for another host.
    Identity = 9,
    /// The local control pipe couldn't be set up, or failed.
    Control = 10,
}
