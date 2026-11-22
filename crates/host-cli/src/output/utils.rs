use super::messages;
use myrig_protocol::local_control::client::{ListReply, StatusReply};

pub fn format_status(status: &StatusReply) -> String {
    messages::status(status.port, &status.listener_state, status.test_mode)
}

pub fn format_list(list: &ListReply) -> String {
    if list.paired_devices.is_empty() {
        return messages::NO_DEVICES.into();
    }
    list.paired_devices
        .iter()
        .map(|device| messages::device(&device.device_id, &device.name))
        .collect::<Vec<_>>()
        .join("\n")
}
