//! Per-user process that shows host notifications.
// ? Started at login: a console app would keep a window open all session.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod windows_notifier {
    use auto_launch::{AutoLaunchBuilder, WindowsEnableMode};
    use myrig_host_windows::{
        control_client::control_pipe_name,
        event_log::report_notifier_failure,
        notification_identity::{
            NOTIFIER_APP_ID, NOTIFIER_DISPLAY_NAME, NOTIFIER_INSTANCE, register, unregister,
        },
        server_identity::connect_verified,
        session::is_console_session,
        single_instance,
    };
    use myrig_notifier::{
        autostart::{APP_NAME, CANNOT_CONFIGURE, CANNOT_UPDATE, Command, NO_ACTION, launch_path},
        listener::{Notifier, run},
    };
    use notify_rust::Notification;
    use std::{io, process::ExitCode};

    /// This user's login entry for the notifier.
    fn login_entry() -> Result<auto_launch::AutoLaunch, &'static str> {
        let path = launch_path()?;
        AutoLaunchBuilder::new()
            .set_app_name(APP_NAME)
            .set_app_path(&path)
            .set_windows_enable_mode(WindowsEnableMode::CurrentUser)
            .build()
            .map_err(|_| CANNOT_CONFIGURE)
    }

    /// Adds or removes this user's login entry for the notifier.
    fn configure(command: Command) -> Result<(), &'static str> {
        let auto = login_entry()?;
        match command {
            Command::Enable => auto.enable(),
            Command::Disable => auto.disable(),
            Command::Run => return Err(NO_ACTION),
        }
        .map_err(|_| CANNOT_UPDATE)
    }

    /// A connected pipe keeps its server process, so checking once at connect
    /// covers every event and pairing code read from it.
    async fn connect() -> io::Result<Option<tokio::net::windows::named_pipe::NamedPipeClient>> {
        connect_verified(&control_pipe_name()).map(|(pipe, _)| Some(pipe))
    }

    struct SystemNotifier;
    impl Notifier for SystemNotifier {
        fn shows_pairing_codes(&self) -> bool {
            is_console_session()
        }

        // ? The notifier has no console, so the refusal is shown as a notification.
        fn unauthorized(&self) {
            self.notify(
                myrig_notifier::listener::UNAUTHORIZED_TITLE,
                myrig_notifier::listener::UNAUTHORIZED,
            );
        }

        fn notify(&self, title: &str, body: &str) {
            let mut notice = Notification::new();
            notice.summary(title).body(body).app_id(NOTIFIER_APP_ID);
            if let Err(error) = notice.show() {
                report_notifier_failure("show a notification", &error.to_string());
            }
        }
    }

    /// Notifications from an unpackaged program need a registered identity. The
    /// login flags manage it along with the login entry.
    fn notification_identity(command: &Command) -> Result<(), &'static str> {
        match command {
            Command::Enable => register(NOTIFIER_APP_ID, NOTIFIER_DISPLAY_NAME),
            Command::Disable => unregister(NOTIFIER_APP_ID),
            Command::Run => Ok(()),
        }
    }

    pub async fn main() -> ExitCode {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        match Command::parse(&args) {
            Ok(Command::Run) => {
                let _instance = match single_instance::acquire(NOTIFIER_INSTANCE) {
                    Ok(Some(instance)) => instance,
                    // - Another copy is already running in this session.
                    Ok(None) => return ExitCode::SUCCESS,
                    Err(_) => return ExitCode::FAILURE,
                };
                // ! Windows shows a toast only for a registered app identity. The login entry can
                // ! be added without it, so every start registers it. The login entry is left
                // ! alone: only the flags change it, so a disabled entry stays disabled.
                if let Err(error) = register(NOTIFIER_APP_ID, NOTIFIER_DISPLAY_NAME) {
                    report_notifier_failure("register its notifications", error);
                }
                run(connect, &SystemNotifier, tokio::time::sleep).await;
                ExitCode::SUCCESS
            }
            Ok(command) => {
                match notification_identity(&command).and_then(|()| configure(command)) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(error) => {
                        report_notifier_failure("change its login entry", error);
                        ExitCode::FAILURE
                    }
                }
            }
            Err(error) => {
                report_notifier_failure("read its command line", error);
                ExitCode::FAILURE
            }
        }
    }
}

#[cfg(windows)]
#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    windows_notifier::main().await
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("The My Rig notifier runs on Windows.");
    std::process::ExitCode::FAILURE
}
