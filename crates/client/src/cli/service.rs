use super::{
    consts::{OK, REFUSED, SHUTDOWN_BOUND, UNREACHABLE, USAGE},
    messages::{
        CANCELLED, CODE_PROMPT, IDENTITY_CHANGED, INCOMPATIBLE, MAC_PROMPT, NO_PCS, NOT_PAIRED,
        OFFLINE, ONLINE, SHUTTING_DOWN, UNREACHABLE_PREFIX, USAGE_TEXT,
    },
    types::{Command, Failure, Terminal},
    utils::{parse, refused, resolve, stop_runtime},
};
use crate::{
    agent::serve,
    host_client::{HostClient, HostError},
    mac_detection::detect_mac,
    network::wake,
    pairing::{Paired, device_name, pair, request_code, revoke},
    platform::Platform,
    shared::{PublicError, normalize_host, normalize_mac, normalize_port},
    store::{PcChanges, PcConfig, PcInput, Store},
};
use myrig_protocol::{api::PcIssue, lan::HostState};
use std::{ffi::OsString, io::Write, net::Ipv4Addr, path::Path, process::ExitCode, sync::Arc};
use uuid::Uuid;

const NOT_TEXT: &str = "An argument is not valid text.";

/// The arguments as text, or `None` if any is not valid UTF-8.
pub(super) fn text_arguments(arguments: impl Iterator<Item = OsString>) -> Option<Vec<String>> {
    arguments
        .map(|argument| argument.into_string().ok())
        .collect()
}

/// Entry point of `myrig`: parses the command line and runs it.
pub fn main(platform: Arc<dyn Platform>) -> ExitCode {
    // * A non-UTF-8 argument is a usage error, not a panic or an empty name.
    let Some(arguments) = text_arguments(std::env::args_os().skip(1)) else {
        eprintln!("{NOT_TEXT}\n{USAGE_TEXT}");
        return ExitCode::from(USAGE);
    };
    // ? One thread is enough: the work is network I/O, and Argon2 runs on the blocking pool.
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return ExitCode::from(REFUSED);
    };
    if let Some(Command::Agent { store }) = parse(&arguments) {
        let input = tokio::io::BufReader::new(tokio::io::stdin());
        let code = match runtime.block_on(serve(platform, &store, input, tokio::io::stdout())) {
            Ok(()) => OK,
            Err(error) => {
                eprintln!("{error}");
                REFUSED
            }
        };
        return finish(runtime, code);
    }
    let store = match platform.settings_directory() {
        Ok(store) => store,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(REFUSED);
        }
    };
    let (stdin, mut stdout, mut stderr) = (std::io::stdin(), std::io::stdout(), std::io::stderr());
    let terminal = Terminal {
        input: &mut stdin.lock(),
        output: &mut stdout,
        errors: &mut stderr,
    };
    let code = runtime.block_on(run(platform, &arguments, &store, terminal));
    finish(runtime, code)
}

/// Stops the runtime within `SHUTDOWN_BOUND`; a save still stuck then turns a
/// success into a refusal, since it may not have been written.
fn finish(runtime: tokio::runtime::Runtime, code: u8) -> ExitCode {
    match stop_runtime(runtime, SHUTDOWN_BOUND) {
        Ok(()) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(if code == OK { REFUSED } else { code })
        }
    }
}

/// Runs one command against the saved PCs in `store` and returns the exit code
/// (0 done, 1 refused or failed, 2 usage error, 3 PC unreachable).
pub async fn run(
    platform: Arc<dyn Platform>,
    arguments: &[String],
    store: &Path,
    terminal: Terminal<'_>,
) -> u8 {
    let Some(command) =
        parse(arguments).filter(|command| !matches!(command, Command::Agent { .. }))
    else {
        let _ = writeln!(terminal.errors, "{USAGE_TEXT}");
        return USAGE;
    };
    let errors = &mut *terminal.errors;
    let mut session = Session {
        platform: platform.clone(),
        input: terminal.input,
        output: terminal.output,
    };
    let result = match Store::open(store, platform) {
        Ok(store) => session.command(command, &store).await,
        Err(error) => Err(refused(error.0)),
    };
    match result {
        Ok(()) => OK,
        Err(failure) => {
            let _ = writeln!(errors, "{}", failure.message);
            failure.code
        }
    }
}

struct Session<'a> {
    platform: Arc<dyn Platform>,
    input: &'a mut dyn std::io::BufRead,
    output: &'a mut dyn Write,
}

impl Session<'_> {
    async fn command(&mut self, command: Command, store: &Store) -> Result<(), Failure> {
        let pcs = store.configs();
        match command {
            Command::List => self.list(&pcs),
            Command::Status(name) => {
                let targets = match name {
                    Some(name) => vec![resolve(&pcs, &name)?],
                    None => pcs,
                };
                for pc in targets {
                    let state = status(&pc).await;
                    self.say(&format!("{}: {state}", pc.name));
                }
                Ok(())
            }
            Command::Wake(name) => {
                let pc = resolve(&pcs, &name)?;
                wake(self.platform.as_ref(), &pc.mac, pc.host)
                    .await
                    .map_err(|error| refused(format!("Wake-on-LAN failed: {error}")))?;
                self.say(&format!("Wake-on-LAN packet sent to {}.", pc.name));
                Ok(())
            }
            Command::Stop { name, yes } => {
                let pc = resolve(&pcs, &name)?;
                self.confirm(&format!("Shut down {}?", pc.name), yes)?;
                match HostClient::for_pc(&pc).shutdown(Uuid::new_v4()).await {
                    Ok(result) if result.test => {
                        self.say(&format!(
                            "Test shutdown received by {}; test mode keeps the PC on.",
                            pc.name
                        ));
                    }
                    Ok(_) => self.say(&format!("{} is shutting down.", pc.name)),
                    Err(error @ (HostError::Connection | HostError::Timeout)) => {
                        return Err(Failure {
                            code: UNREACHABLE,
                            message: format!("{}: {error}", pc.name),
                        });
                    }
                    Err(error) => {
                        return Err(refused(format!(
                            "{}: {}",
                            pc.name,
                            describe(Err(error), &pc)
                        )));
                    }
                }
                Ok(())
            }
            Command::Pair { host, port } => self.pair(store, &host, &port).await,
            Command::Rename { name, new_name } => {
                let pc = resolve(&pcs, &name)?;
                let changes = PcChanges {
                    name: Some(new_name),
                    ..PcChanges::default()
                };
                let renamed = store
                    .update(pc.id, changes)
                    .map_err(|error| refused(error.0))?;
                self.say(&format!("Renamed {} to {}.", pc.name, renamed.name));
                Ok(())
            }
            Command::Remove { name, yes } => {
                let pc = resolve(&pcs, &name)?;
                self.confirm(&format!("Remove {}?", pc.name), yes)?;
                store.delete(pc.id).map_err(|error| refused(error.0))?;
                let forgot = match HostClient::for_pc(&pc).unpair().await {
                    Ok(()) => "The PC forgot this device.",
                    Err(_) => "The PC did not answer; remove this device in My Rig on the PC.",
                };
                self.say(&format!("Removed {}. {forgot}", pc.name));
                Ok(())
            }
            Command::Agent { .. } => Err(Failure {
                code: USAGE,
                message: USAGE_TEXT.to_owned(),
            }),
        }
    }

    fn list(&mut self, pcs: &[PcConfig]) -> Result<(), Failure> {
        if pcs.is_empty() {
            self.say(NO_PCS);
        }
        for pc in pcs {
            self.say(&format!(
                "{}\t{}:{}\t{}\t{}",
                pc.name, pc.host, pc.port, pc.mac, pc.id
            ));
        }
        Ok(())
    }

    async fn pair(&mut self, store: &Store, host: &str, port: &str) -> Result<(), Failure> {
        let host = normalize_host(host).map_err(|error| refused(error.0))?;
        let port = normalize_port(port).map_err(|error| refused(error.0))?;
        request_code(host, port, &device_name(self.platform.as_ref()))
            .await
            .map_err(pairing_failure)?;
        let code = self.ask_secret(CODE_PROMPT);
        let paired = pair(host, port, &code, None)
            .await
            .map_err(pairing_failure)?;
        let name = paired.computer_name.clone();
        let saved = self.keep(store, host, port, &paired);
        if saved.is_err() {
            revoke(host, port, paired.authentication).await;
        }
        saved?;
        self.say(&format!("Paired with {name}."));
        Ok(())
    }

    /// Saves a pairing the PC already has, asking for the MAC address if it
    /// can't be detected.
    fn keep(
        &mut self,
        store: &Store,
        host: Ipv4Addr,
        port: u16,
        paired: &Paired,
    ) -> Result<(), Failure> {
        let mac = match detect_mac(self.platform.as_ref(), host) {
            Ok(mac) => mac,
            // * The PC has already saved this device, so keep asking until the MAC is valid. We
            // * only give up, and revoke the pairing, when input ends.
            Err(_) => loop {
                let answer = self.prompt(MAC_PROMPT);
                match normalize_mac(answer.as_deref().unwrap_or_default()) {
                    Ok(mac) => break mac,
                    Err(error) if answer.is_some() => self.say(error.0),
                    Err(error) => return Err(refused(error.0)),
                }
            },
        };
        let input = PcInput {
            name: paired.computer_name.clone(),
            host: host.to_string(),
            port: Some(port.to_string()),
            mac,
        };
        store
            .add(input, paired.authentication.clone())
            .map_err(|error| refused(error.0))?;
        Ok(())
    }

    fn confirm(&mut self, question: &str, yes: bool) -> Result<(), Failure> {
        if yes {
            return Ok(());
        }
        let answer = self.ask(&format!("{question} [y/N] "));
        if matches!(answer.to_ascii_lowercase().as_str(), "y" | "yes") {
            Ok(())
        } else {
            Err(refused(CANCELLED))
        }
    }

    fn ask(&mut self, prompt: &str) -> String {
        self.prompt(prompt).unwrap_or_default()
    }

    /// Reads an answer the PC shows, with echo off, so it isn't printed to the
    /// terminal's history or a screen share.
    fn ask_secret(&mut self, prompt: &str) -> String {
        let hidden = self.platform.hide_input();
        let answer = self.ask(prompt);
        if hidden.is_some() {
            // ? With echo off the newline the user typed isn't shown, so end the line here.
            self.say("");
        }
        drop(hidden);
        answer
    }

    /// The trimmed answer, or `None` once input has ended.
    fn prompt(&mut self, prompt: &str) -> Option<String> {
        let _ = write!(self.output, "{prompt}");
        let _ = self.output.flush();
        let mut answer = String::new();
        match self.input.read_line(&mut answer) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(answer.trim().to_owned()),
        }
    }

    fn say(&mut self, line: &str) {
        let _ = writeln!(self.output, "{line}");
    }
}

async fn status(pc: &PcConfig) -> &'static str {
    describe(
        HostClient::for_pc(pc)
            .status()
            .await
            .map(|status| (status.host_id, status.state)),
        pc,
    )
}

fn describe(status: Result<(Uuid, HostState), HostError>, pc: &PcConfig) -> &'static str {
    match status {
        Ok((host_id, _)) if host_id != pc.authentication.host_id => IDENTITY_CHANGED,
        Ok((_, HostState::Online)) => ONLINE,
        Ok((_, HostState::ShuttingDown)) => SHUTTING_DOWN,
        Err(HostError::Connection | HostError::Timeout) => OFFLINE,
        // ? An invalid reply is a compatibility problem, as in the plugin.
        Err(error) => match error.issue().unwrap_or(PcIssue::Compatibility) {
            PcIssue::Identity => IDENTITY_CHANGED,
            PcIssue::Authentication => NOT_PAIRED,
            PcIssue::Compatibility => INCOMPATIBLE,
        },
    }
}

fn pairing_failure(error: PublicError) -> Failure {
    let code = if error.0.starts_with(UNREACHABLE_PREFIX) {
        UNREACHABLE
    } else {
        REFUSED
    };
    Failure {
        code,
        message: error.0.to_owned(),
    }
}
