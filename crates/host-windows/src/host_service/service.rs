use super::types::ServiceExit;
use crate::shared::consts::SERVICE_NAME;
use crate::{
    control_pipe::ControlPipe, power::WindowsPower, program_data::program_data,
    registry_control::RegistryControl, stop_progress,
};
use myrig_host_core::{
    file_storage::{FileStorage, load_security},
    listener,
    paired_devices::RegistryConfig,
    service_loop::{ServiceLoopError, join_endpoints},
};
use std::{
    ffi::OsString,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
use tokio::{runtime::Builder, sync::watch};
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
};

static EXIT_CODE: AtomicU32 = AtomicU32::new(0);
define_windows_service!(service_entry, service_main);

pub fn run() -> Result<(), &'static str> {
    // ! No process arguments allowed: no other config path, listener or power
    //   mode.
    if std::env::args_os().len() != 1 {
        return Err("Service arguments are unsupported.");
    }
    service_dispatcher::start(SERVICE_NAME, service_entry).map_err(|_| "SCM dispatcher failed.")?;
    if EXIT_CODE.load(Ordering::SeqCst) == 0 {
        Ok(())
    } else {
        Err("Service failed.")
    }
}

fn status(
    handle: ServiceStatusHandle,
    state: ServiceState,
    exit: Option<ServiceExit>,
    checkpoint: u32,
) -> Result<(), ServiceExit> {
    let pending = matches!(
        state,
        ServiceState::StartPending | ServiceState::StopPending
    );
    handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: if state == ServiceState::Running {
                ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
            } else {
                ServiceControlAccept::empty()
            },
            exit_code: exit.map_or(ServiceExitCode::Win32(0), |exit| {
                ServiceExitCode::ServiceSpecific(exit as u32)
            }),
            checkpoint: if pending { checkpoint } else { 0 },
            wait_hint: if pending {
                Duration::from_secs(5)
            } else {
                Duration::ZERO
            },
            process_id: None,
        })
        .map_err(|_| ServiceExit::Status)
}

fn service_main(arguments: Vec<OsString>) {
    let (send, mut receive) = watch::channel(false);
    let handle = match service_control_handler::register(SERVICE_NAME, move |event| match event {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = send.send(true);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    }) {
        Ok(handle) => handle,
        Err(_) => {
            EXIT_CODE.store(ServiceExit::Register as u32, Ordering::SeqCst);
            return;
        }
    };
    let result = (|| -> Result<(), ServiceExit> {
        status(handle, ServiceState::StartPending, None, 1)?;
        if arguments.len() > 1 {
            return Err(ServiceExit::Arguments);
        }
        let directory = program_data().map_err(|_| ServiceExit::Config)?;
        let host_directory = directory.join("MyRig");
        let store = FileStorage::open(&host_directory.join("host.json"))
            .map_err(|_| ServiceExit::Config)?;
        let bytes = store.read().map_err(|_| ServiceExit::Config)?;
        let config = RegistryConfig::from_json(&bytes).map_err(|_| ServiceExit::Config)?;
        // ! The identity lives in a separate admin-only folder, so permissions on
        //   the registry staging can't let the service replace its TLS identity.
        //   A missing or unsafe identity is refused before taking requests.
        let host_id = config.host_id();
        let identity = load_security(&directory.join("MyRigSecurity").join("Security"), host_id)
            .map_err(|_| ServiceExit::Identity)?
            .into_identity();
        let power: Box<dyn myrig_host_core::shutdown::Power> = Box::new(WindowsPower);
        let control = RegistryControl::from_loaded(store, bytes, config, power)
            .map_err(|_| ServiceExit::Config)?;
        let host = control.host();
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| ServiceExit::Runtime)?;
        runtime.block_on(async {
            let pipe = ControlPipe::bind().map_err(|_| ServiceExit::Control)?;
            status(handle, ServiceState::Running, None, 0)?;
            let (stop, endpoint_stop) = watch::channel(false);
            let http_stop = endpoint_stop.clone();
            let approval_identity = identity.clone();
            let (stopping_tx, stopping_rx) = watch::channel(false);
            let work = join_endpoints(
                host.clone(),
                async {
                    listener::run(
                        host,
                        identity,
                        crate::lan_socket::bind_exclusive,
                        http_stop,
                        crate::event_log::report_listener_error,
                    )
                    .await;
                    Ok(())
                },
                async {
                    control
                        .serve_secure(pipe, endpoint_stop, approval_identity)
                        .await
                        .map_err(|_| ())
                },
                async {
                    while !*receive.borrow_and_update() {
                        if receive.changed().await.is_err() {
                            break;
                        }
                    }
                },
                stop,
                || {
                    status(handle, ServiceState::StopPending, None, 1).map_err(|_| ())?;
                    let _ = stopping_tx.send(true);
                    Ok(())
                },
            );
            let (result, progress_failed) =
                stop_progress::until_done(work, stopping_rx, |checkpoint| {
                    status(handle, ServiceState::StopPending, None, checkpoint).map_err(|_| ())
                })
                .await;
            if progress_failed {
                return Err(ServiceExit::Status);
            }
            result.map_err(|error| match error {
                ServiceLoopError::Status => ServiceExit::Status,
                ServiceLoopError::Control => ServiceExit::Control,
                ServiceLoopError::Http | ServiceLoopError::UnexpectedExit => ServiceExit::Endpoint,
            })
        })
    })();
    let mut exit = result.err();
    if status(handle, ServiceState::Stopped, exit, 0).is_err() {
        exit = Some(ServiceExit::Status);
    }
    EXIT_CODE.store(exit.map_or(0, |exit| exit as u32), Ordering::SeqCst);
}
