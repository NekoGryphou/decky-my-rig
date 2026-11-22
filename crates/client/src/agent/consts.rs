use std::time::Duration;

/// Used while the Quick Access panel or the settings page is open.
pub(super) const STEADY_POLL: Duration = Duration::from_secs(15);
/// Used when nothing from the plugin is on screen, like during a game.
pub(super) const IDLE_POLL: Duration = Duration::from_secs(60);
pub(super) const TRANSITION_POLL: Duration = Duration::from_secs(2);
/// How long requests still running at EOF get before we drop them.
pub(super) const DRAIN_TIMEOUT: Duration = Duration::from_secs(4);
/// Longest request line we accept. Real requests are much smaller.
pub(super) const MAX_LINE: usize = 1024 * 1024;
