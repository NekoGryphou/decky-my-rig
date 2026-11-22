pub const USAGE: &str = "Usage: myrig-host status | set-port <1024..65535> | test-mode on|off | pair <name> | list | remove <id> | rename <id> <name>";
pub const NOT_WINDOWS: &str = "myrig-host runs on Windows.";
pub const NO_DEVICES: &str = "No paired devices.";

pub(crate) fn status(port: u16, listener: &str, test: bool) -> String {
    format!(
        "Service: running\nPort: {port}\nPort status: {listener}\nTest mode: {}",
        on_off(test)
    )
}
pub(crate) fn port(port: u16) -> String {
    format!("Port: {port}")
}
pub(crate) fn test_mode(test: bool) -> String {
    format!("Test mode: {}", on_off(test))
}
pub(crate) fn pairing(code: &str, seconds: u64) -> String {
    format!("Pairing code: {code}\nExpires in: {seconds} s")
}
pub(crate) fn device(id: &str, name: &str) -> String {
    format!("{id}  {name}")
}
pub(crate) fn removed(name: &str) -> String {
    format!("Removed: {name}")
}
pub(crate) fn renamed(name: &str) -> String {
    format!("Renamed: {name}")
}
fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}
