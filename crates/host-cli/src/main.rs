#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("{}", myrig_host::output::messages::NOT_WINDOWS);
    std::process::ExitCode::from(2)
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    use myrig_host::{command::parse_command, output::messages, shared::CliError};
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let command = match parse_command(&arguments) {
        Some(command) => command,
        None => {
            eprintln!("{}", messages::USAGE);
            return std::process::ExitCode::from(2);
        }
    };
    let result = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(myrig_host::client::execute_command(&command, send)),
        Err(_) => Err(CliError::Unavailable),
    };
    match result {
        Ok(message) => {
            println!("{message}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", error.message());
            std::process::ExitCode::from(error.exit_code())
        }
    }
}

#[cfg(windows)]
async fn send(request: Vec<u8>) -> Result<Vec<u8>, myrig_host::shared::CliError> {
    use myrig_host::shared::CliError;
    use myrig_host_windows::{
        control_client::control_pipe_name, server_identity::connect_verified,
    };
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let (mut pipe, identity) =
            connect_verified(&control_pipe_name()).map_err(myrig_host::client::connection_error)?;
        let reply = myrig_protocol::local_control::client::exchange(&mut pipe, &request)
            .await
            .map_err(|_| CliError::Unavailable)?;
        identity.recheck(&pipe).map_err(|_| CliError::Unavailable)?;
        Ok(reply)
    })
    .await
    .map_err(|_| CliError::Unavailable)?
}
