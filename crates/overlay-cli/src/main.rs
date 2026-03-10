mod devnet;

use std::{env, ffi::OsString, path::PathBuf, process::ExitCode, thread, time::Duration};

use overlay_core::{runtime::NodeRuntime, REPOSITORY_STAGE};

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
        Command::Run {
            config_path,
            tick_ms,
            max_ticks,
        } => run_command(config_path, tick_ms, max_ticks),
        Command::Smoke { devnet_dir } => devnet::run_smoke(&devnet_dir),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Stage,
    Help,
    Run {
        config_path: PathBuf,
        tick_ms: u64,
        max_ticks: Option<u64>,
    },
    Smoke {
        devnet_dir: PathBuf,
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
        "run" => parse_run_command(args),
        "smoke" => parse_smoke_command(args),
        other => Err(format!("unknown command '{other}'")),
    }
}

fn parse_run_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut tick_ms = 1_000_u64;
    let mut max_ticks = None;
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
    })
}

fn parse_smoke_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut devnet_dir = PathBuf::from("devnet");
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--devnet-dir" => {
                let Some(value) = args.next() else {
                    return Err("--devnet-dir requires a path".to_string());
                };
                devnet_dir = PathBuf::from(value);
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown smoke flag '{other}'")),
        }
    }

    Ok(Command::Smoke { devnet_dir })
}

fn parse_u64_flag(flag: &str, value: &OsString) -> Result<u64, String> {
    value
        .to_string_lossy()
        .parse::<u64>()
        .map_err(|error| format!("{flag} must be an unsigned integer: {error}"))
}

fn run_command(config_path: PathBuf, tick_ms: u64, max_ticks: Option<u64>) -> Result<(), String> {
    let mut runtime =
        NodeRuntime::from_config_path(&config_path).map_err(|error| error.to_string())?;
    runtime.startup_now().map_err(|error| error.to_string())?;

    let mut emitted_logs = 0usize;
    print_new_logs(&runtime, &mut emitted_logs)?;

    let mut ticks_run = 0_u64;
    while max_ticks.map(|limit| ticks_run < limit).unwrap_or(true) {
        thread::sleep(Duration::from_millis(tick_ms));
        runtime.tick_now().map_err(|error| error.to_string())?;
        print_new_logs(&runtime, &mut emitted_logs)?;
        ticks_run = ticks_run.saturating_add(1);
    }

    runtime.shutdown_now().map_err(|error| error.to_string())?;
    print_new_logs(&runtime, &mut emitted_logs)?;
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

fn print_usage() {
    println!("overlay-cli: {}", REPOSITORY_STAGE);
    println!("usage:");
    println!("  overlay-cli");
    println!("  overlay-cli run --config <path> [--tick-ms <ms>] [--max-ticks <count>]");
    println!("  overlay-cli smoke [--devnet-dir <path>]");
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

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
            ])
            .unwrap(),
            Command::Smoke {
                devnet_dir: PathBuf::from("fixtures/devnet"),
            }
        );
    }
}
