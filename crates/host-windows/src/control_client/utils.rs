pub fn control_pipe_name() -> String {
    format!(
        r"\\.\pipe\{}",
        myrig_protocol::local_control::CONTROL_SOCKET_NAME
    )
}
