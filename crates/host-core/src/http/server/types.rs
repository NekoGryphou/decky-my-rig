/// An authenticated API route.
#[derive(Clone, Copy)]
pub(super) enum Route {
    Status,
    /// `test` asks for a test stop to be recorded instead of a real shutdown.
    Shutdown {
        test: bool,
    },
    /// The calling device removes its own pairing.
    Unpair,
}
