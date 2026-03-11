mod bootstrap_server;
mod devnet;
mod operator_state;
mod signal;

use std::{
    env,
    ffi::OsString,
    path::PathBuf,
    process::ExitCode,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use operator_state::{OperatorStateManager, RuntimeShutdownReason};
use overlay_core::{runtime::NodeRuntime, REPOSITORY_STAGE};
use signal::{install_shutdown_handlers, pending_shutdown_signal, ShutdownSignal};

fn main() -> ExitCode {
    match try_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("overlay-cli: {error}");
            ExitCode::from(1)
        }
    }
}

fn try_main() -> Result<(), String> {
    match parse_command(env::args_os())? {
        Command::Stage => {
            println!("overlay-cli: {}", REPOSITORY_STAGE);
            Ok(())
        }
        Command::Help => {
            print_usage();
            Ok(())
        }
        Command::Status { config_path } => print_status_command(config_path),
        Command::Run {
            config_path,
            tick_ms,
            max_ticks,
            status_every_ticks,
            dial_hints,
        } => run_command(
            config_path,
            tick_ms,
            max_ticks,
            status_every_ticks,
            dial_hints,
        ),
        Command::Smoke {
            devnet_dir,
            soak_seconds,
            status_interval_seconds,
            fault,
        } => {
            if soak_seconds == 0
                && status_interval_seconds.is_none()
                && fault == devnet::SmokeFault::None
            {
                devnet::run_smoke(&devnet_dir)
            } else {
                devnet::run_smoke_with_options(
                    &devnet_dir,
                    devnet::SmokeOptions {
                        soak_seconds,
                        status_interval_seconds,
                        fault,
                    },
                )
            }
        }
        Command::BootstrapServe {
            bind_addr,
            bootstrap_file,
            max_requests,
        } => bootstrap_server::run(&bind_addr, &bootstrap_file, max_requests),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Stage,
    Help,
    Status {
        config_path: PathBuf,
    },
    Run {
        config_path: PathBuf,
        tick_ms: u64,
        max_ticks: Option<u64>,
        status_every_ticks: Option<u64>,
        dial_hints: Vec<String>,
    },
    Smoke {
        devnet_dir: PathBuf,
        soak_seconds: u64,
        status_interval_seconds: Option<u64>,
        fault: devnet::SmokeFault,
    },
    BootstrapServe {
        bind_addr: String,
        bootstrap_file: PathBuf,
        max_requests: Option<usize>,
    },
}

fn parse_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let _binary = args.next();
    let Some(command) = args.next() else {
        return Ok(Command::Stage);
    };

    match command.to_string_lossy().as_ref() {
        "-h" | "--help" => Ok(Command::Help),
        "status" => parse_status_command(args),
        "run" => parse_run_command(args),
        "smoke" => parse_smoke_command(args),
        "bootstrap-serve" => parse_bootstrap_serve_command(args),
        other => Err(format!("unknown command '{other}'")),
    }
}

fn parse_status_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--config" => {
                let Some(value) = args.next() else {
                    return Err("--config requires a path".to_string());
                };
                config_path = Some(PathBuf::from(value));
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown status flag '{other}'")),
        }
    }

    let Some(config_path) = config_path else {
        return Err("status requires --config <path>".to_string());
    };

    Ok(Command::Status { config_path })
}

fn parse_run_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut tick_ms = 1_000_u64;
    let mut max_ticks = None;
    let mut status_every_ticks = None;
    let mut dial_hints = Vec::new();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--config" => {
                let Some(value) = args.next() else {
                    return Err("--config requires a path".to_string());
                };
                config_path = Some(PathBuf::from(value));
            }
            "--tick-ms" => {
                let Some(value) = args.next() else {
                    return Err("--tick-ms requires an integer value".to_string());
                };
                tick_ms = parse_u64_flag("--tick-ms", &value)?;
            }
            "--max-ticks" => {
                let Some(value) = args.next() else {
                    return Err("--max-ticks requires an integer value".to_string());
                };
                max_ticks = Some(parse_u64_flag("--max-ticks", &value)?);
            }
            "--status-every" => {
                let Some(value) = args.next() else {
                    return Err("--status-every requires an integer value".to_string());
                };
                status_every_ticks = Some(parse_non_zero_u64_flag("--status-every", &value)?);
            }
            "--dial" => {
                let Some(value) = args.next() else {
                    return Err("--dial requires a tcp://host:port hint".to_string());
                };
                dial_hints.push(value.to_string_lossy().into_owned());
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown run flag '{other}'")),
        }
    }

    let Some(config_path) = config_path else {
        return Err("run requires --config <path>".to_string());
    };

    Ok(Command::Run {
        config_path,
        tick_ms,
        max_ticks,
        status_every_ticks,
        dial_hints,
    })
}

fn parse_smoke_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut devnet_dir = PathBuf::from("devnet");
    let mut soak_seconds = 0_u64;
    let mut status_interval_seconds = None;
    let mut fault = devnet::SmokeFault::None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--devnet-dir" => {
                let Some(value) = args.next() else {
                    return Err("--devnet-dir requires a path".to_string());
                };
                devnet_dir = PathBuf::from(value);
            }
            "--soak-seconds" => {
                let Some(value) = args.next() else {
                    return Err("--soak-seconds requires an integer value".to_string());
                };
                soak_seconds = parse_u64_flag("--soak-seconds", &value)?;
            }
            "--status-interval-seconds" => {
                let Some(value) = args.next() else {
                    return Err("--status-interval-seconds requires an integer value".to_string());
                };
                status_interval_seconds = Some(parse_non_zero_u64_flag(
                    "--status-interval-seconds",
                    &value,
                )?);
            }
            "--fault" => {
                let Some(value) = args.next() else {
                    return Err(
                        "--fault requires one of: none, node-c-down, relay-unavailable".to_string(),
                    );
                };
                fault = devnet::SmokeFault::parse(&value.to_string_lossy())?;
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown smoke flag '{other}'")),
        }
    }

    Ok(Command::Smoke {
        devnet_dir,
        soak_seconds,
        status_interval_seconds,
        fault,
    })
}

fn parse_bootstrap_serve_command(
    args: impl IntoIterator<Item = OsString>,
) -> Result<Command, String> {
    let mut bind_addr = None;
    let mut bootstrap_file = None;
    let mut max_requests = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--bind" => {
                let Some(value) = args.next() else {
                    return Err("--bind requires an address".to_string());
                };
                bind_addr = Some(value.to_string_lossy().into_owned());
            }
            "--bootstrap-file" => {
                let Some(value) = args.next() else {
                    return Err("--bootstrap-file requires a path".to_string());
                };
                bootstrap_file = Some(PathBuf::from(value));
            }
            "--max-requests" => {
                let Some(value) = args.next() else {
                    return Err("--max-requests requires an integer value".to_string());
                };
                max_requests = Some(parse_usize_flag("--max-requests", &value)?);
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown bootstrap-serve flag '{other}'")),
        }
    }

    let Some(bind_addr) = bind_addr else {
        return Err("bootstrap-serve requires --bind <addr>".to_string());
    };
    let Some(bootstrap_file) = bootstrap_file else {
        return Err("bootstrap-serve requires --bootstrap-file <path>".to_string());
    };

    Ok(Command::BootstrapServe {
        bind_addr,
        bootstrap_file,
        max_requests,
    })
}

fn parse_u64_flag(flag: &str, value: &OsString) -> Result<u64, String> {
    value
        .to_string_lossy()
        .parse::<u64>()
        .map_err(|error| format!("{flag} must be an unsigned integer: {error}"))
}

fn parse_non_zero_u64_flag(flag: &str, value: &OsString) -> Result<u64, String> {
    let parsed = parse_u64_flag(flag, value)?;
    if parsed == 0 {
        return Err(format!("{flag} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_usize_flag(flag: &str, value: &OsString) -> Result<usize, String> {
    value
        .to_string_lossy()
        .parse::<usize>()
        .map_err(|error| format!("{flag} must be an unsigned integer: {error}"))
}

fn run_command(
    config_path: PathBuf,
    tick_ms: u64,
    max_ticks: Option<u64>,
    status_every_ticks: Option<u64>,
    dial_hints: Vec<String>,
) -> Result<(), String> {
    install_shutdown_handlers()?;

    let mut runtime =
        NodeRuntime::from_config_path(&config_path).map_err(|error| error.to_string())?;
    let startup_timestamp = current_unix_ms()?;
    let mut operator_state = OperatorStateManager::acquire(
        &config_path,
        runtime.context().node_id().to_string(),
        startup_timestamp,
    )?;

    if let Err(error) = runtime.startup(startup_timestamp) {
        let _ = operator_state.write_status(&runtime, 0, startup_timestamp);
        return Err(error.to_string());
    }
    for dial_hint in dial_hints {
        let timestamp_unix_ms = current_unix_ms()?;
        if let Err(error) = runtime.open_tcp_session(&dial_hint, timestamp_unix_ms) {
            let _ = operator_state.write_status(&runtime, 0, timestamp_unix_ms);
            return Err(error.to_string());
        }
    }

    operator_state.write_status(&runtime, 0, startup_timestamp)?;
    let mut emitted_logs = 0usize;
    print_new_logs(&runtime, &mut emitted_logs)?;
    print_status_snapshot(&runtime, operator_state.lifecycle(), status_every_ticks, 0)?;

    let mut ticks_run = 0_u64;
    let mut shutdown_reason = RuntimeShutdownReason::NaturalEnd;
    while max_ticks.map(|limit| ticks_run < limit).unwrap_or(true) {
        if let Some(signal) = pending_shutdown_signal() {
            shutdown_reason = signal.into();
            emit_shutdown_signal(signal)?;
            break;
        }
        if let Some(signal) = sleep_until_next_tick(tick_ms) {
            shutdown_reason = signal.into();
            emit_shutdown_signal(signal)?;
            break;
        }
        if let Err(error) = runtime.tick_now() {
            let timestamp_unix_ms = current_unix_ms()?;
            let _ = operator_state.write_status(&runtime, ticks_run, timestamp_unix_ms);
            return Err(error.to_string());
        }
        print_new_logs(&runtime, &mut emitted_logs)?;
        ticks_run = ticks_run.saturating_add(1);
        let timestamp_unix_ms = current_unix_ms()?;
        operator_state.write_status(&runtime, ticks_run, timestamp_unix_ms)?;
        print_status_snapshot(
            &runtime,
            operator_state.lifecycle(),
            status_every_ticks,
            ticks_run,
        )?;
    }

    let shutdown_timestamp = current_unix_ms()?;
    operator_state.begin_shutdown(shutdown_reason, shutdown_timestamp);
    if let Err(error) = runtime.shutdown(shutdown_timestamp) {
        let _ = operator_state.write_status(&runtime, ticks_run, shutdown_timestamp);
        return Err(error.to_string());
    }
    print_new_logs(&runtime, &mut emitted_logs)?;
    operator_state.finalize_clean_shutdown(
        &runtime,
        ticks_run,
        shutdown_reason,
        shutdown_timestamp,
    )?;
    print_status_snapshot(
        &runtime,
        operator_state.lifecycle(),
        status_every_ticks,
        ticks_run,
    )?;
    Ok(())
}

fn print_status_command(config_path: PathBuf) -> Result<(), String> {
    let status = OperatorStateManager::read_status_file(&config_path)?;
    println!("{status}");
    Ok(())
}

fn print_new_logs(runtime: &NodeRuntime, emitted_logs: &mut usize) -> Result<(), String> {
    let logs = runtime.context().observability().logs();
    for entry in logs.iter().skip(*emitted_logs) {
        println!(
            "{}",
            serde_json::to_string(entry).map_err(|error| error.to_string())?
        );
    }
    *emitted_logs = logs.len();
    Ok(())
}

fn print_status_snapshot(
    runtime: &NodeRuntime,
    lifecycle: &operator_state::RuntimeLifecycleStatus,
    status_every_ticks: Option<u64>,
    ticks_run: u64,
) -> Result<(), String> {
    let Some(interval) = status_every_ticks else {
        return Ok(());
    };
    if ticks_run != 0 && !ticks_run.is_multiple_of(interval) {
        return Ok(());
    }

    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "kind": "runtime_status",
            "ticks_run": ticks_run,
            "lifecycle": lifecycle,
            "health": runtime.health_snapshot(),
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn current_unix_ms() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as u64)
}

fn sleep_until_next_tick(tick_ms: u64) -> Option<ShutdownSignal> {
    let mut remaining_ms = tick_ms;
    while remaining_ms > 0 {
        let sleep_ms = remaining_ms.min(100);
        thread::sleep(Duration::from_millis(sleep_ms));
        if let Some(signal) = pending_shutdown_signal() {
            return Some(signal);
        }
        remaining_ms -= sleep_ms;
    }
    None
}

fn emit_shutdown_signal(signal: ShutdownSignal) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "kind": "runtime_control",
            "event": "shutdown_signal_received",
            "signal": signal.as_str(),
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn print_usage() {
    println!("overlay-cli: {}", REPOSITORY_STAGE);
    println!("usage:");
    println!("  overlay-cli");
    println!("  overlay-cli status --config <path>");
    println!(
        "  overlay-cli run --config <path> [--tick-ms <ms>] [--max-ticks <count>] [--status-every <ticks>] [--dial <tcp://host:port> ...]"
    );
    println!(
        "  overlay-cli smoke [--devnet-dir <path>] [--soak-seconds <seconds>] [--status-interval-seconds <seconds>] [--fault <none|node-c-down|relay-unavailable>]"
    );
    println!(
        "  overlay-cli bootstrap-serve --bind <addr> --bootstrap-file <path> [--max-requests <count>]"
    );
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use crate::devnet::SmokeFault;

    use super::{parse_command, Command};

    #[test]
    fn parse_command_defaults_to_stage() {
        assert_eq!(
            parse_command([OsString::from("overlay-cli")]).unwrap(),
            Command::Stage
        );
    }

    #[test]
    fn parse_command_parses_run_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("run"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-a.json"),
                OsString::from("--tick-ms"),
                OsString::from("250"),
                OsString::from("--max-ticks"),
                OsString::from("3"),
            ])
            .unwrap(),
            Command::Run {
                config_path: PathBuf::from("devnet/configs/node-a.json"),
                tick_ms: 250,
                max_ticks: Some(3),
                status_every_ticks: None,
                dial_hints: Vec::new(),
            }
        );
    }

    #[test]
    fn parse_command_parses_status_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("status"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-a.json"),
            ])
            .unwrap(),
            Command::Status {
                config_path: PathBuf::from("devnet/configs/node-a.json"),
            }
        );
    }

    #[test]
    fn parse_command_parses_smoke_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("smoke"),
                OsString::from("--devnet-dir"),
                OsString::from("fixtures/devnet"),
                OsString::from("--soak-seconds"),
                OsString::from("600"),
                OsString::from("--status-interval-seconds"),
                OsString::from("120"),
            ])
            .unwrap(),
            Command::Smoke {
                devnet_dir: PathBuf::from("fixtures/devnet"),
                soak_seconds: 600,
                status_interval_seconds: Some(120),
                fault: SmokeFault::None,
            }
        );
    }

    #[test]
    fn parse_command_parses_smoke_fault_flag() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("smoke"),
                OsString::from("--fault"),
                OsString::from("relay-unavailable"),
            ])
            .unwrap(),
            Command::Smoke {
                devnet_dir: PathBuf::from("devnet"),
                soak_seconds: 0,
                status_interval_seconds: None,
                fault: SmokeFault::RelayUnavailable,
            }
        );
    }

    #[test]
    fn parse_command_parses_status_flag() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("run"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-a.json"),
                OsString::from("--status-every"),
                OsString::from("5"),
                OsString::from("--dial"),
                OsString::from("tcp://127.0.0.1:4102"),
            ])
            .unwrap(),
            Command::Run {
                config_path: PathBuf::from("devnet/configs/node-a.json"),
                tick_ms: 1_000,
                max_ticks: None,
                status_every_ticks: Some(5),
                dial_hints: vec!["tcp://127.0.0.1:4102".to_string()],
            }
        );
    }

    #[test]
    fn parse_command_parses_bootstrap_serve_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("bootstrap-serve"),
                OsString::from("--bind"),
                OsString::from("127.0.0.1:4201"),
                OsString::from("--bootstrap-file"),
                OsString::from("devnet/bootstrap/node-foundation.json"),
                OsString::from("--max-requests"),
                OsString::from("2"),
            ])
            .unwrap(),
            Command::BootstrapServe {
                bind_addr: "127.0.0.1:4201".to_string(),
                bootstrap_file: PathBuf::from("devnet/bootstrap/node-foundation.json"),
                max_requests: Some(2),
            }
        );
    }
}
