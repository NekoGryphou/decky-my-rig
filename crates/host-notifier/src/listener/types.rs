pub trait Notifier {
    fn notify(&self, title: &str, body: &str);
    fn unauthorized(&self);
    /// Only the notifier on the PC's own screen opens pairing codes. Each open
    /// replaces the last code, so there's one per request.
    fn shows_pairing_codes(&self) -> bool;
}
