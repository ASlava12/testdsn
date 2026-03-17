mod bootstrap_server;
mod devnet;
mod operator_client;
mod operator_state;
mod signal;

use std::{
    env,
    ffi::OsString,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use operator_state::{OperatorStateManager, RuntimeShutdownReason};
use overlay_core::{
    config::{ConfigTemplateProfile, OverlayConfig},
    identity::{derive_app_id, NodeId},
    records::{IntroTicket, PresenceRecord},
    relay::{IntroResponse, ResolveIntro},
    rendezvous::{LookupNotFound, LookupResult, PublishAck, PublishPresence},
    runtime::NodeRuntime,
    service::{
        GetServiceRecord, OpenAppSession, OpenAppSessionResult, ServiceRecordResponse,
        ServiceRecordResponseStatus,
    },
    wire::MessageType,
    REPOSITORY_STAGE,
};
use serde::Serialize;
use serde_json::Value;
use signal::{install_shutdown_handlers, pending_shutdown_signal, process_exists, ShutdownSignal};

fn main() -> ExitCode {
    let command = match parse_command(env::args_os()) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("overlay-cli: {error}");
            return ExitCode::from(1);
        }
    };

    if let Command::Doctor { config_path } = command {
        return doctor_command(config_path);
    }

    match execute_command(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("overlay-cli: {error}");
            ExitCode::from(1)
        }
    }
}

fn execute_command(command: Command) -> Result<(), String> {
    match command {
        Command::Stage => {
            println!("overlay-cli: {}", REPOSITORY_STAGE);
            Ok(())
        }
        Command::Help => {
            print_usage();
            Ok(())
        }
        Command::ConfigTemplate {
            output_path,
            profile,
        } => config_template_command(output_path, profile),
        Command::Status {
            config_path,
            summary_only,
        } => print_status_command(config_path, summary_only),
        Command::Run {
            config_path,
            tick_ms,
            max_ticks,
            status_every_ticks,
            dial_hints,
            local_services,
        } => run_command(
            config_path,
            tick_ms,
            max_ticks,
            status_every_ticks,
            dial_hints,
            local_services,
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
        Command::Publish {
            config_path,
            target,
            relay_refs,
            capability_requirements,
            transport_classes,
            expires_in_s,
        } => publish_command(
            config_path,
            &target,
            relay_refs,
            capability_requirements,
            transport_classes,
            expires_in_s,
        ),
        Command::Lookup {
            config_path,
            target,
            node_id,
        } => lookup_command(config_path, &target, node_id),
        Command::OpenService {
            config_path,
            target,
            target_node_id,
            app_namespace,
            service_name,
        } => open_service_command(
            config_path,
            &target,
            target_node_id,
            &app_namespace,
            &service_name,
        ),
        Command::RelayIntro {
            config_path,
            target,
            relay_node_id,
            requester_node_id,
            expires_in_s,
        } => relay_intro_command(
            config_path,
            &target,
            relay_node_id,
            requester_node_id,
            expires_in_s,
        ),
        Command::Doctor { .. } => unreachable!("doctor is handled before execute_command"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalServiceSpec {
    app_namespace: String,
    service_name: String,
    service_version: String,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Stage,
    Help,
    ConfigTemplate {
        output_path: Option<PathBuf>,
        profile: ConfigTemplateProfile,
    },
    Status {
        config_path: PathBuf,
        summary_only: bool,
    },
    Run {
        config_path: PathBuf,
        tick_ms: u64,
        max_ticks: Option<u64>,
        status_every_ticks: Option<u64>,
        dial_hints: Vec<String>,
        local_services: Vec<LocalServiceSpec>,
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
    Publish {
        config_path: PathBuf,
        target: String,
        relay_refs: Vec<Vec<u8>>,
        capability_requirements: Vec<String>,
        transport_classes: Vec<String>,
        expires_in_s: u64,
    },
    Lookup {
        config_path: PathBuf,
        target: String,
        node_id: NodeId,
    },
    OpenService {
        config_path: PathBuf,
        target: String,
        target_node_id: NodeId,
        app_namespace: String,
        service_name: String,
    },
    RelayIntro {
        config_path: PathBuf,
        target: String,
        relay_node_id: NodeId,
        requester_node_id: NodeId,
        expires_in_s: u64,
    },
    Doctor {
        config_path: PathBuf,
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
        "config-template" => parse_config_template_command(args),
        "status" => parse_status_command(args),
        "doctor" => parse_doctor_command(args),
        "run" => parse_run_command(args),
        "smoke" => parse_smoke_command(args),
        "bootstrap-serve" => parse_bootstrap_serve_command(args),
        "publish" => parse_publish_command(args),
        "lookup" => parse_lookup_command(args),
        "open-service" => parse_open_service_command(args),
        "relay-intro" => parse_relay_intro_command(args),
        other => Err(format!("unknown command '{other}'")),
    }
}

fn parse_config_template_command(
    args: impl IntoIterator<Item = OsString>,
) -> Result<Command, String> {
    let mut output_path = None;
    let mut profile = ConfigTemplateProfile::UserNode;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--output" => {
                let Some(value) = args.next() else {
                    return Err("--output requires a path".to_string());
                };
                output_path = Some(PathBuf::from(value));
            }
            "--profile" => {
                let Some(value) = args.next() else {
                    return Err(
                        "--profile requires one of: user-node, relay-capable, bootstrap-seed"
                            .to_string(),
                    );
                };
                profile = ConfigTemplateProfile::parse(&value.to_string_lossy()).ok_or_else(|| {
                    format!(
                        "unsupported config profile '{}'; use one of: user-node, relay-capable, bootstrap-seed",
                        value.to_string_lossy()
                    )
                })?;
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown config-template flag '{other}'")),
        }
    }

    Ok(Command::ConfigTemplate {
        output_path,
        profile,
    })
}

fn parse_status_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut summary_only = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--config" => {
                let Some(value) = args.next() else {
                    return Err("--config requires a path".to_string());
                };
                config_path = Some(PathBuf::from(value));
            }
            "--summary" => {
                summary_only = true;
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown status flag '{other}'")),
        }
    }

    let Some(config_path) = config_path else {
        return Err("status requires --config <path>".to_string());
    };

    Ok(Command::Status {
        config_path,
        summary_only,
    })
}

fn parse_doctor_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
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
            other => return Err(format!("unknown doctor flag '{other}'")),
        }
    }

    let Some(config_path) = config_path else {
        return Err("doctor requires --config <path>".to_string());
    };

    Ok(Command::Doctor { config_path })
}

fn parse_run_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut tick_ms = 1_000_u64;
    let mut max_ticks = None;
    let mut status_every_ticks = None;
    let mut dial_hints = Vec::new();
    let mut local_services = Vec::new();
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
            "--service" => {
                let Some(value) = args.next() else {
                    return Err("--service requires namespace:name[:version]".to_string());
                };
                local_services.push(parse_local_service_spec(&value.to_string_lossy())?);
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
        local_services,
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

fn parse_publish_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut target = None;
    let mut relay_refs = Vec::new();
    let mut capability_requirements = Vec::new();
    let mut transport_classes = Vec::new();
    let mut expires_in_s = 300_u64;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--config" => {
                let Some(value) = args.next() else {
                    return Err("--config requires a path".to_string());
                };
                config_path = Some(PathBuf::from(value));
            }
            "--target" => {
                let Some(value) = args.next() else {
                    return Err("--target requires tcp://host:port".to_string());
                };
                target = Some(value.to_string_lossy().into_owned());
            }
            "--relay-ref" => {
                let Some(value) = args.next() else {
                    return Err("--relay-ref requires 64 hex characters".to_string());
                };
                relay_refs.push(parse_fixed_hex(&value.to_string_lossy(), NodeId::LEN)?);
            }
            "--capability" => {
                let Some(value) = args.next() else {
                    return Err("--capability requires a value".to_string());
                };
                capability_requirements.push(value.to_string_lossy().into_owned());
            }
            "--transport-class" => {
                let Some(value) = args.next() else {
                    return Err("--transport-class requires a value".to_string());
                };
                transport_classes.push(value.to_string_lossy().into_owned());
            }
            "--expires-in" => {
                let Some(value) = args.next() else {
                    return Err("--expires-in requires an integer value".to_string());
                };
                expires_in_s = parse_non_zero_u64_flag("--expires-in", &value)?;
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown publish flag '{other}'")),
        }
    }

    Ok(Command::Publish {
        config_path: config_path.ok_or_else(|| "publish requires --config <path>".to_string())?,
        target: target.ok_or_else(|| "publish requires --target <tcp://host:port>".to_string())?,
        relay_refs,
        capability_requirements,
        transport_classes,
        expires_in_s,
    })
}

fn parse_lookup_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut target = None;
    let mut node_id = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--config" => {
                let Some(value) = args.next() else {
                    return Err("--config requires a path".to_string());
                };
                config_path = Some(PathBuf::from(value));
            }
            "--target" => {
                let Some(value) = args.next() else {
                    return Err("--target requires tcp://host:port".to_string());
                };
                target = Some(value.to_string_lossy().into_owned());
            }
            "--node-id" => {
                let Some(value) = args.next() else {
                    return Err("--node-id requires 64 hex characters".to_string());
                };
                node_id = Some(parse_node_id_hex(&value.to_string_lossy())?);
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown lookup flag '{other}'")),
        }
    }

    Ok(Command::Lookup {
        config_path: config_path.ok_or_else(|| "lookup requires --config <path>".to_string())?,
        target: target.ok_or_else(|| "lookup requires --target <tcp://host:port>".to_string())?,
        node_id: node_id.ok_or_else(|| "lookup requires --node-id <hex>".to_string())?,
    })
}

fn parse_open_service_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut target = None;
    let mut target_node_id = None;
    let mut app_namespace = None;
    let mut service_name = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--config" => {
                let Some(value) = args.next() else {
                    return Err("--config requires a path".to_string());
                };
                config_path = Some(PathBuf::from(value));
            }
            "--target" => {
                let Some(value) = args.next() else {
                    return Err("--target requires tcp://host:port".to_string());
                };
                target = Some(value.to_string_lossy().into_owned());
            }
            "--target-node-id" => {
                let Some(value) = args.next() else {
                    return Err("--target-node-id requires 64 hex characters".to_string());
                };
                target_node_id = Some(parse_node_id_hex(&value.to_string_lossy())?);
            }
            "--service-namespace" => {
                let Some(value) = args.next() else {
                    return Err("--service-namespace requires a value".to_string());
                };
                app_namespace = Some(value.to_string_lossy().into_owned());
            }
            "--service-name" => {
                let Some(value) = args.next() else {
                    return Err("--service-name requires a value".to_string());
                };
                service_name = Some(value.to_string_lossy().into_owned());
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown open-service flag '{other}'")),
        }
    }

    Ok(Command::OpenService {
        config_path: config_path
            .ok_or_else(|| "open-service requires --config <path>".to_string())?,
        target: target
            .ok_or_else(|| "open-service requires --target <tcp://host:port>".to_string())?,
        target_node_id: target_node_id
            .ok_or_else(|| "open-service requires --target-node-id <hex>".to_string())?,
        app_namespace: app_namespace
            .ok_or_else(|| "open-service requires --service-namespace <value>".to_string())?,
        service_name: service_name
            .ok_or_else(|| "open-service requires --service-name <value>".to_string())?,
    })
}

fn parse_relay_intro_command(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut config_path = None;
    let mut target = None;
    let mut relay_node_id = None;
    let mut requester_node_id = None;
    let mut expires_in_s = 300_u64;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--config" => {
                let Some(value) = args.next() else {
                    return Err("--config requires a path".to_string());
                };
                config_path = Some(PathBuf::from(value));
            }
            "--target" => {
                let Some(value) = args.next() else {
                    return Err("--target requires tcp://host:port".to_string());
                };
                target = Some(value.to_string_lossy().into_owned());
            }
            "--relay-node-id" => {
                let Some(value) = args.next() else {
                    return Err("--relay-node-id requires 64 hex characters".to_string());
                };
                relay_node_id = Some(parse_node_id_hex(&value.to_string_lossy())?);
            }
            "--requester-node-id" => {
                let Some(value) = args.next() else {
                    return Err("--requester-node-id requires 64 hex characters".to_string());
                };
                requester_node_id = Some(parse_node_id_hex(&value.to_string_lossy())?);
            }
            "--expires-in" => {
                let Some(value) = args.next() else {
                    return Err("--expires-in requires an integer value".to_string());
                };
                expires_in_s = parse_non_zero_u64_flag("--expires-in", &value)?;
            }
            "-h" | "--help" => return Ok(Command::Help),
            other => return Err(format!("unknown relay-intro flag '{other}'")),
        }
    }

    Ok(Command::RelayIntro {
        config_path: config_path
            .ok_or_else(|| "relay-intro requires --config <path>".to_string())?,
        target: target
            .ok_or_else(|| "relay-intro requires --target <tcp://host:port>".to_string())?,
        relay_node_id: relay_node_id
            .ok_or_else(|| "relay-intro requires --relay-node-id <hex>".to_string())?,
        requester_node_id: requester_node_id
            .ok_or_else(|| "relay-intro requires --requester-node-id <hex>".to_string())?,
        expires_in_s,
    })
}

fn parse_local_service_spec(value: &str) -> Result<LocalServiceSpec, String> {
    let mut parts = value.split(':');
    let app_namespace = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "--service requires namespace:name[:version]".to_string())?;
    let service_name = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "--service requires namespace:name[:version]".to_string())?;
    let service_version = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("1.0.0");
    if parts.next().is_some() {
        return Err("--service requires namespace:name[:version]".to_string());
    }
    Ok(LocalServiceSpec {
        app_namespace: app_namespace.to_string(),
        service_name: service_name.to_string(),
        service_version: service_version.to_string(),
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

fn config_template_command(
    output_path: Option<PathBuf>,
    profile: ConfigTemplateProfile,
) -> Result<(), String> {
    let rendered = render_config_template(profile)?;
    if let Some(path) = output_path {
        write_config_template_file(&path, rendered.as_bytes())?;
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn render_config_template(profile: ConfigTemplateProfile) -> Result<String, String> {
    let template = OverlayConfig::template_for_profile(profile);
    template
        .clone()
        .validate()
        .map_err(|error| format!("internal config template is invalid: {error}"))?;
    serde_json::to_string_pretty(&template)
        .map_err(|error| format!("failed to serialize config template: {error}"))
}

fn write_config_template_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!("refusing to overwrite existing file {}", path.display())
            } else {
                format!(
                    "failed to create config template file {}: {error}",
                    path.display()
                )
            }
        })?;
    file.write_all(bytes).map_err(|error| {
        format!(
            "failed to write config template file {}: {error}",
            path.display()
        )
    })?;
    file.write_all(b"\n").map_err(|error| {
        format!(
            "failed to finalize config template file {}: {error}",
            path.display()
        )
    })?;
    Ok(())
}

fn publish_command(
    config_path: PathBuf,
    target: &str,
    relay_refs: Vec<Vec<u8>>,
    capability_requirements: Vec<String>,
    transport_classes: Vec<String>,
    expires_in_s: u64,
) -> Result<(), String> {
    let runtime = NodeRuntime::from_config_path(&config_path).map_err(|error| error.to_string())?;
    let signing_key = runtime.context().signing_key().clone();
    let listener_addr = runtime
        .context()
        .config()
        .tcp_listener_addr
        .clone()
        .ok_or_else(|| {
            format!(
                "publish requires tcp_listener_addr in {}",
                config_path.display()
            )
        })?;
    let now_unix_ms = current_unix_ms()?;
    let now_unix_s = now_unix_ms / 1_000;
    let transport_classes = if transport_classes.is_empty() {
        vec!["tcp".to_string()]
    } else {
        transport_classes
    };
    let mut record = PresenceRecord {
        version: 1,
        node_id: runtime.context().node_id(),
        epoch: now_unix_s / runtime.context().config().epoch_duration_s,
        expires_at_unix_s: now_unix_s.saturating_add(expires_in_s),
        sequence: now_unix_ms,
        transport_classes,
        reachability_mode: if relay_refs.is_empty() {
            "direct".to_string()
        } else {
            "hybrid".to_string()
        },
        locator_commitment: listener_addr.as_bytes().to_vec(),
        encrypted_contact_blobs: vec![listener_addr.as_bytes().to_vec()],
        relay_hint_refs: relay_refs,
        intro_policy: "allow".to_string(),
        capability_requirements,
        signature: Vec::new(),
    };
    let body = record
        .canonical_body_bytes()
        .map_err(|error| error.to_string())?;
    record.signature = signing_key.sign(&body).as_bytes().to_vec();

    let mut client = operator_client::OperatorSessionClient::connect(&config_path, target)?;
    let ack: PublishAck =
        client.request_typed(&PublishPresence { record }, MessageType::PublishAck)?;
    let _ = client.close();
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "step": "publish_presence",
            "target": target,
            "node_id": ack.node_id.to_string(),
            "placement_key": hex_encode(ack.placement_key.as_bytes()),
            "disposition": ack.disposition,
            "accepted_epoch": ack.accepted_epoch,
            "accepted_sequence": ack.accepted_sequence,
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

fn lookup_command(config_path: PathBuf, target: &str, node_id: NodeId) -> Result<(), String> {
    let mut client = operator_client::OperatorSessionClient::connect(&config_path, target)?;
    let lookup_started = std::time::Instant::now();
    let (message_type, _, body) =
        client.request(&overlay_core::rendezvous::LookupNode { node_id })?;
    let lookup_latency_ms = lookup_started
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let _ = client.close();

    match message_type {
        MessageType::LookupResult => {
            let result: LookupResult =
                serde_json::from_slice(&body).map_err(|error| error.to_string())?;
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "step": "lookup_node",
                    "target": target,
                    "result": "found",
                    "node_id": result.node_id.to_string(),
                    "remaining_budget": result.remaining_budget,
                    "lookup_latency_ms": lookup_latency_ms,
                    "record": result.record,
                }))
                .map_err(|error| error.to_string())?
            );
            Ok(())
        }
        MessageType::LookupNotFound => {
            let not_found: LookupNotFound =
                serde_json::from_slice(&body).map_err(|error| error.to_string())?;
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "step": "lookup_node",
                    "target": target,
                    "result": "not_found",
                    "node_id": not_found.node_id.to_string(),
                    "reason": not_found.reason,
                    "remaining_budget": not_found.remaining_budget,
                    "lookup_latency_ms": lookup_latency_ms,
                }))
                .map_err(|error| error.to_string())?
            );
            Err(format!(
                "lookup for {} returned {:?}",
                not_found.node_id, not_found.reason
            ))
        }
        other => Err(format!("lookup expected a lookup response, got {other:?}")),
    }
}

fn open_service_command(
    config_path: PathBuf,
    target: &str,
    target_node_id: NodeId,
    app_namespace: &str,
    service_name: &str,
) -> Result<(), String> {
    let app_id = derive_app_id(&target_node_id, app_namespace, service_name);
    let mut client = operator_client::OperatorSessionClient::connect(&config_path, target)?;
    let response: ServiceRecordResponse = client.request_typed(
        &GetServiceRecord { app_id },
        MessageType::ServiceRecordResponse,
    )?;
    if response.status != ServiceRecordResponseStatus::Found {
        let _ = client.close();
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "step": "open_service",
                "target": target,
                "app_id": app_id.to_string(),
                "resolve_status": response.status,
            }))
            .map_err(|error| error.to_string())?
        );
        return Err(format!("service {app_id} was not found on {target}"));
    }
    let record = response
        .record
        .ok_or_else(|| format!("service {app_id} was found without a record payload"))?;
    let opened: OpenAppSessionResult = client.request_typed(
        &OpenAppSession {
            app_id,
            reachability_ref: record.reachability_ref.clone(),
        },
        MessageType::OpenAppSessionResult,
    )?;
    let _ = client.close();
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "step": "open_service",
            "target": target,
            "app_id": app_id.to_string(),
            "resolve_status": response.status,
            "open_status": opened.status,
            "session_id": opened.session_id,
            "service_name": service_name,
        }))
        .map_err(|error| error.to_string())?
    );
    if opened.session_id.is_some() {
        Ok(())
    } else {
        Err(format!(
            "open_service for {app_id} returned {:?}",
            opened.status
        ))
    }
}

fn relay_intro_command(
    config_path: PathBuf,
    target: &str,
    relay_node_id: NodeId,
    requester_node_id: NodeId,
    expires_in_s: u64,
) -> Result<(), String> {
    let runtime = NodeRuntime::from_config_path(&config_path).map_err(|error| error.to_string())?;
    let signing_key = runtime.context().signing_key().clone();
    let now_unix_ms = current_unix_ms()?;
    let now_unix_s = now_unix_ms / 1_000;
    let mut ticket = IntroTicket {
        ticket_id: format!("relay-intro-{}", now_unix_ms).into_bytes(),
        target_node_id: runtime.context().node_id(),
        requester_binding: requester_node_id.as_bytes().to_vec(),
        scope: "relay-intro".to_string(),
        issued_at_unix_s: now_unix_s,
        expires_at_unix_s: now_unix_s.saturating_add(expires_in_s),
        nonce: now_unix_ms.to_be_bytes().to_vec(),
        signature: Vec::new(),
    };
    let body = ticket
        .canonical_body_bytes()
        .map_err(|error| error.to_string())?;
    ticket.signature = signing_key.sign(&body).as_bytes().to_vec();

    let mut client = operator_client::OperatorSessionClient::connect(&config_path, target)?;
    let response: IntroResponse = client.request_typed(
        &ResolveIntro {
            relay_node_id,
            intro_ticket: ticket,
        },
        MessageType::IntroResponse,
    )?;
    let _ = client.close();
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "step": if response.status == overlay_core::relay::IntroResponseStatus::Forwarded {
                "relay_fallback_bound"
            } else {
                "relay_intro"
            },
            "target": target,
            "relay_node_id": response.relay_node_id.to_string(),
            "target_node_id": response.target_node_id.to_string(),
            "ticket_id_hex": hex_encode(&response.ticket_id),
            "status": response.status,
            "requester_node_id": requester_node_id.to_string(),
        }))
        .map_err(|error| error.to_string())?
    );
    if response.status == overlay_core::relay::IntroResponseStatus::Forwarded {
        Ok(())
    } else {
        Err(format!("relay intro returned {:?}", response.status))
    }
}

fn parse_node_id_hex(value: &str) -> Result<NodeId, String> {
    let bytes = parse_fixed_hex(value, NodeId::LEN)?;
    let array: [u8; NodeId::LEN] = bytes
        .try_into()
        .map_err(|_| "internal node-id conversion failed".to_string())?;
    Ok(NodeId::from_bytes(array))
}

fn parse_fixed_hex(value: &str, expected_len: usize) -> Result<Vec<u8>, String> {
    let trimmed = value.trim();
    if trimmed.len() != expected_len * 2 {
        return Err(format!(
            "expected {} hex characters, got {}",
            expected_len * 2,
            trimmed.len()
        ));
    }
    let mut bytes = Vec::with_capacity(expected_len);
    for chunk in trimmed.as_bytes().chunks_exact(2) {
        let high = hex_nibble(chunk[0]).ok_or_else(|| format!("invalid hex value '{trimmed}'"))?;
        let low = hex_nibble(chunk[1]).ok_or_else(|| format!("invalid hex value '{trimmed}'"))?;
        bytes.push((high << 4) | low);
    }
    Ok(bytes)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("hex encoding should succeed");
    }
    encoded
}

fn run_command(
    config_path: PathBuf,
    tick_ms: u64,
    max_ticks: Option<u64>,
    status_every_ticks: Option<u64>,
    dial_hints: Vec<String>,
    local_services: Vec<LocalServiceSpec>,
) -> Result<(), String> {
    install_shutdown_handlers()?;

    let mut runtime =
        NodeRuntime::from_config_path(&config_path).map_err(|error| error.to_string())?;
    let recovery_state = OperatorStateManager::read_recovery_state(&config_path)?;
    let startup_timestamp = current_unix_ms()?;
    let mut operator_state = OperatorStateManager::acquire(
        &config_path,
        runtime.context().node_id().to_string(),
        startup_timestamp,
    )?;

    if let Err(error) =
        runtime.startup_with_recovery_state(startup_timestamp, recovery_state.as_ref())
    {
        let _ = operator_state.write_status(&runtime, 0, startup_timestamp);
        return Err(error.to_string());
    }
    for service in &local_services {
        if let Err(error) = runtime.register_local_service(
            &service.app_namespace,
            &service.service_name,
            &service.service_version,
            startup_timestamp,
        ) {
            let _ = operator_state.write_status(&runtime, 0, startup_timestamp);
            return Err(error.to_string());
        }
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

fn print_status_command(config_path: PathBuf, summary_only: bool) -> Result<(), String> {
    let status = OperatorStateManager::read_status_value(&config_path)?;
    let output = if summary_only {
        status
            .get("summary")
            .cloned()
            .ok_or_else(|| "persisted runtime status did not include a summary".to_string())?
    } else {
        status
    };
    println!(
        "{}",
        serde_json::to_string(&output).map_err(|error| error.to_string())?
    );
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

const DOCTOR_EXIT_WARN: u8 = 2;
const DOCTOR_EXIT_FAIL: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DoctorResult {
    Ok,
    Warn,
    Fail,
}

impl DoctorResult {
    const fn exit_code(self) -> u8 {
        match self {
            Self::Ok => 0,
            Self::Warn => DOCTOR_EXIT_WARN,
            Self::Fail => DOCTOR_EXIT_FAIL,
        }
    }

    fn escalate(&mut self, other: Self) {
        let current = self.exit_code();
        let next = other.exit_code();
        if next > current {
            *self = other;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct DoctorCheck {
    name: &'static str,
    result: DoctorResult,
    detail: String,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorReport {
    kind: &'static str,
    stage: &'static str,
    config_path: String,
    status_file: String,
    result: DoctorResult,
    exit_code: u8,
    checks: Vec<DoctorCheck>,
    summary: Option<Value>,
}

fn doctor_command(config_path: PathBuf) -> ExitCode {
    let (report, exit_code) = build_doctor_report(&config_path);
    match serde_json::to_string(&report) {
        Ok(encoded) => println!("{encoded}"),
        Err(error) => {
            eprintln!("overlay-cli: failed to encode doctor report: {error}");
            return ExitCode::from(1);
        }
    }
    ExitCode::from(exit_code)
}

fn build_doctor_report(config_path: &Path) -> (DoctorReport, u8) {
    let status_file = OperatorStateManager::status_file_path(config_path)
        .unwrap_or_else(|_| config_path.to_path_buf());
    let mut result = DoctorResult::Ok;
    let mut checks = Vec::new();

    let expected_node_id = match NodeRuntime::from_config_path(config_path) {
        Ok(runtime) => {
            checks.push(DoctorCheck {
                name: "config",
                result: DoctorResult::Ok,
                detail: format!(
                    "config loaded successfully for node_id {}",
                    runtime.context().node_id()
                ),
            });
            Some(runtime.context().node_id().to_string())
        }
        Err(error) => {
            result.escalate(DoctorResult::Fail);
            checks.push(DoctorCheck {
                name: "config",
                result: DoctorResult::Fail,
                detail: error.to_string(),
            });
            None
        }
    };

    let status_value = match OperatorStateManager::read_status_value(config_path) {
        Ok(value) => {
            checks.push(DoctorCheck {
                name: "status_file",
                result: DoctorResult::Ok,
                detail: format!(
                    "persisted runtime status loaded from {}",
                    status_file.display()
                ),
            });
            Some(value)
        }
        Err(error) => {
            result.escalate(DoctorResult::Fail);
            checks.push(DoctorCheck {
                name: "status_file",
                result: DoctorResult::Fail,
                detail: error,
            });
            None
        }
    };

    if let Some(status) = status_value.as_ref() {
        if let (Some(expected_node_id), Some(status_node_id)) = (
            expected_node_id.as_deref(),
            status.pointer("/lifecycle/node_id").and_then(Value::as_str),
        ) {
            if expected_node_id == status_node_id {
                checks.push(DoctorCheck {
                    name: "node_id",
                    result: DoctorResult::Ok,
                    detail: format!(
                        "config and persisted status agree on node_id {status_node_id}"
                    ),
                });
            } else {
                result.escalate(DoctorResult::Fail);
                checks.push(DoctorCheck {
                    name: "node_id",
                    result: DoctorResult::Fail,
                    detail: format!(
                        "config resolved node_id {expected_node_id}, but persisted status belongs to {status_node_id}"
                    ),
                });
            }
        }

        let clean_shutdown = status
            .pointer("/lifecycle/clean_shutdown")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let pid = status.pointer("/lifecycle/pid").and_then(Value::as_u64);
        let process_running = pid
            .filter(|pid| *pid <= u32::MAX as u64)
            .map(|pid| process_exists(pid as u32))
            .unwrap_or(false);
        let runtime_state = status
            .pointer("/health/runtime/state")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let active_peers = status
            .pointer("/health/runtime/active_peers")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let accepted_sources = status
            .pointer("/health/bootstrap/last_accepted_sources")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let restored_from_peer_cache = status
            .pointer("/health/recovery/restored_from_peer_cache")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let recent_failures = status
            .pointer("/summary/recent_failures")
            .and_then(Value::as_array)
            .map(|failures| failures.len())
            .unwrap_or(0);
        let recovered_from_unclean_shutdown = status
            .pointer("/lifecycle/recovered_from_unclean_shutdown")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let process_check = if process_running {
            DoctorCheck {
                name: "process",
                result: DoctorResult::Ok,
                detail: "runtime appears active".to_string(),
            }
        } else if clean_shutdown {
            result.escalate(DoctorResult::Warn);
            DoctorCheck {
                name: "process",
                result: DoctorResult::Warn,
                detail: "runtime is not currently active; last shutdown was clean".to_string(),
            }
        } else {
            result.escalate(DoctorResult::Fail);
            DoctorCheck {
                name: "process",
                result: DoctorResult::Fail,
                detail:
                    "runtime is not active and the last persisted status was not a clean shutdown"
                        .to_string(),
            }
        };
        checks.push(process_check);

        let runtime_check = match runtime_state {
            "running" if active_peers > 0 => DoctorCheck {
                name: "runtime_state",
                result: DoctorResult::Ok,
                detail: format!("runtime is running with {active_peers} active peers"),
            },
            "degraded" => {
                result.escalate(DoctorResult::Fail);
                DoctorCheck {
                    name: "runtime_state",
                    result: DoctorResult::Fail,
                    detail: "runtime is degraded and currently has no active peers".to_string(),
                }
            }
            "shutting_down" if clean_shutdown => {
                result.escalate(DoctorResult::Warn);
                DoctorCheck {
                    name: "runtime_state",
                    result: DoctorResult::Warn,
                    detail: "runtime status reflects a clean shutdown; restart the node for live service".to_string(),
                }
            }
            other => {
                result.escalate(DoctorResult::Warn);
                DoctorCheck {
                    name: "runtime_state",
                    result: DoctorResult::Warn,
                    detail: format!("runtime last reported state {other}"),
                }
            }
        };
        checks.push(runtime_check);

        let bootstrap_check = if accepted_sources > 0 {
            DoctorCheck {
                name: "bootstrap",
                result: DoctorResult::Ok,
                detail: format!(
                    "latest bootstrap attempt accepted {accepted_sources} configured source(s)"
                ),
            }
        } else if restored_from_peer_cache && active_peers > 0 {
            result.escalate(DoctorResult::Warn);
            DoctorCheck {
                name: "bootstrap",
                result: DoctorResult::Warn,
                detail: "live bootstrap is currently unavailable; runtime recovered from the persisted peer cache and will keep retrying bootstrap".to_string(),
            }
        } else {
            result.escalate(DoctorResult::Fail);
            DoctorCheck {
                name: "bootstrap",
                result: DoctorResult::Fail,
                detail: "no bootstrap source was accepted and no recoverable active peers were available".to_string(),
            }
        };
        checks.push(bootstrap_check);

        if recovered_from_unclean_shutdown {
            result.escalate(DoctorResult::Warn);
            checks.push(DoctorCheck {
                name: "shutdown_recovery",
                result: DoctorResult::Warn,
                detail: "runtime recovered after a previous unclean shutdown; inspect recent failures before regular use".to_string(),
            });
        }

        if recent_failures > 0 {
            result.escalate(DoctorResult::Warn);
            checks.push(DoctorCheck {
                name: "recent_failures",
                result: DoctorResult::Warn,
                detail: format!(
                    "status summary includes {recent_failures} recent failure record(s)"
                ),
            });
        } else {
            checks.push(DoctorCheck {
                name: "recent_failures",
                result: DoctorResult::Ok,
                detail: "no recent failure records were captured in the persisted summary"
                    .to_string(),
            });
        }
    }

    let exit_code = result.exit_code();
    (
        DoctorReport {
            kind: "runtime_doctor",
            stage: REPOSITORY_STAGE,
            config_path: config_path.display().to_string(),
            status_file: status_file.display().to_string(),
            result,
            exit_code,
            checks,
            summary: status_value.and_then(|status| status.get("summary").cloned()),
        },
        exit_code,
    )
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
    println!(
        "  overlay-cli config-template [--output <path>] [--profile <user-node|relay-capable|bootstrap-seed>]"
    );
    println!("  overlay-cli status --config <path> [--summary]");
    println!("  overlay-cli doctor --config <path>");
    println!(
        "  overlay-cli run --config <path> [--tick-ms <ms>] [--max-ticks <count>] [--status-every <ticks>] [--dial <tcp://host:port> ...] [--service <namespace:name[:version]> ...]"
    );
    println!(
        "  overlay-cli smoke [--devnet-dir <path>] [--soak-seconds <seconds>] [--status-interval-seconds <seconds>] [--fault <none|node-c-down|relay-unavailable>]"
    );
    println!(
        "  overlay-cli bootstrap-serve --bind <addr> --bootstrap-file <path> [--max-requests <count>]"
    );
    println!(
        "  overlay-cli publish --config <path> --target <tcp://host:port> [--relay-ref <node-id-hex> ...] [--capability <value> ...] [--transport-class <value> ...] [--expires-in <seconds>]"
    );
    println!("  overlay-cli lookup --config <path> --target <tcp://host:port> --node-id <hex>");
    println!(
        "  overlay-cli open-service --config <path> --target <tcp://host:port> --target-node-id <hex> --service-namespace <ns> --service-name <name>"
    );
    println!(
        "  overlay-cli relay-intro --config <path> --target <tcp://host:port> --relay-node-id <hex> --requester-node-id <hex> [--expires-in <seconds>]"
    );
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::devnet::SmokeFault;

    use super::{
        parse_command, render_config_template, write_config_template_file, Command,
        LocalServiceSpec,
    };
    use overlay_core::{
        config::{ConfigTemplateProfile, OverlayConfig},
        identity::NodeId,
    };

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
                local_services: Vec::new(),
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
                summary_only: false,
            }
        );
    }

    #[test]
    fn parse_command_parses_config_template_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("config-template"),
                OsString::from("--output"),
                OsString::from("configs/node.json"),
            ])
            .unwrap(),
            Command::ConfigTemplate {
                output_path: Some(PathBuf::from("configs/node.json")),
                profile: ConfigTemplateProfile::UserNode,
            }
        );
    }

    #[test]
    fn render_config_template_matches_overlay_core_template() {
        let rendered = render_config_template(ConfigTemplateProfile::UserNode)
            .expect("config template should render");
        let parsed: OverlayConfig =
            serde_json::from_str(&rendered).expect("rendered template should parse");
        assert_eq!(parsed, OverlayConfig::template());
        parsed
            .validate()
            .expect("rendered template should remain valid");
    }

    #[test]
    fn parse_command_parses_config_template_profile_flag() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("config-template"),
                OsString::from("--profile"),
                OsString::from("relay-capable"),
            ])
            .unwrap(),
            Command::ConfigTemplate {
                output_path: None,
                profile: ConfigTemplateProfile::RelayCapable,
            }
        );
    }

    #[test]
    fn parse_command_parses_status_summary_flag() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("status"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-a.json"),
                OsString::from("--summary"),
            ])
            .unwrap(),
            Command::Status {
                config_path: PathBuf::from("devnet/configs/node-a.json"),
                summary_only: true,
            }
        );
    }

    #[test]
    fn parse_command_parses_doctor_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("doctor"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-a.json"),
            ])
            .unwrap(),
            Command::Doctor {
                config_path: PathBuf::from("devnet/configs/node-a.json"),
            }
        );
    }

    #[test]
    fn write_config_template_file_creates_new_file_and_rejects_overwrite() {
        let output_dir = unique_temp_dir("config-template");
        fs::create_dir_all(&output_dir).expect("temp output dir should be created");
        let output_path = output_dir.join("node.json");

        write_config_template_file(&output_path, b"{\"node_key_path\":\"./keys/node.key\"}")
            .expect("template file should be written");
        assert_eq!(
            fs::read_to_string(&output_path).expect("template file should be readable"),
            "{\"node_key_path\":\"./keys/node.key\"}\n"
        );

        let error = write_config_template_file(&output_path, b"{}")
            .expect_err("existing template path should be rejected");
        assert!(error.contains("refusing to overwrite existing file"));

        let _ = fs::remove_dir_all(output_dir);
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
                local_services: Vec::new(),
            }
        );
    }

    #[test]
    fn parse_command_parses_run_service_flag() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("run"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-b.json"),
                OsString::from("--service"),
                OsString::from("devnet:terminal:1.2.3"),
            ])
            .unwrap(),
            Command::Run {
                config_path: PathBuf::from("devnet/configs/node-b.json"),
                tick_ms: 1_000,
                max_ticks: None,
                status_every_ticks: None,
                dial_hints: Vec::new(),
                local_services: vec![LocalServiceSpec {
                    app_namespace: "devnet".to_string(),
                    service_name: "terminal".to_string(),
                    service_version: "1.2.3".to_string(),
                }],
            }
        );
    }

    #[test]
    fn parse_command_parses_publish_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("publish"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-b.json"),
                OsString::from("--target"),
                OsString::from("tcp://127.0.0.1:4111"),
                OsString::from("--relay-ref"),
                OsString::from("16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d"),
                OsString::from("--capability"),
                OsString::from("service-host"),
            ])
            .unwrap(),
            Command::Publish {
                config_path: PathBuf::from("devnet/configs/node-b.json"),
                target: "tcp://127.0.0.1:4111".to_string(),
                relay_refs: vec![vec![
                    0x16, 0xf5, 0x2d, 0x6f, 0xea, 0x63, 0xef, 0x08, 0x64, 0x05, 0xaa, 0x71, 0xb5,
                    0x37, 0xdd, 0x48, 0x33, 0xbd, 0x0b, 0x36, 0xff, 0xe0, 0x54, 0xbe, 0x0f, 0xd0,
                    0x7f, 0xb5, 0x25, 0xaf, 0x15, 0x7d,
                ]],
                capability_requirements: vec!["service-host".to_string()],
                transport_classes: Vec::new(),
                expires_in_s: 300,
            }
        );
    }

    #[test]
    fn parse_command_parses_lookup_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("lookup"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-a.json"),
                OsString::from("--target"),
                OsString::from("tcp://127.0.0.1:4111"),
                OsString::from("--node-id"),
                OsString::from("1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b"),
            ])
            .unwrap(),
            Command::Lookup {
                config_path: PathBuf::from("devnet/configs/node-a.json"),
                target: "tcp://127.0.0.1:4111".to_string(),
                node_id: NodeId::from_bytes([
                    0x1e, 0xed, 0x29, 0xb1, 0x65, 0x4f, 0xbc, 0xa9, 0x46, 0x17, 0x00, 0x4d, 0x79,
                    0x69, 0xdf, 0xc4, 0x65, 0x2b, 0x1f, 0x30, 0xa7, 0xa8, 0xb7, 0x71, 0xc3, 0x48,
                    0x00, 0x15, 0x54, 0x83, 0x38, 0x0b,
                ]),
            }
        );
    }

    #[test]
    fn parse_command_parses_open_service_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("open-service"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-a.json"),
                OsString::from("--target"),
                OsString::from("tcp://127.0.0.1:4112"),
                OsString::from("--target-node-id"),
                OsString::from("1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b"),
                OsString::from("--service-namespace"),
                OsString::from("devnet"),
                OsString::from("--service-name"),
                OsString::from("terminal"),
            ])
            .unwrap(),
            Command::OpenService {
                config_path: PathBuf::from("devnet/configs/node-a.json"),
                target: "tcp://127.0.0.1:4112".to_string(),
                target_node_id: NodeId::from_bytes([
                    0x1e, 0xed, 0x29, 0xb1, 0x65, 0x4f, 0xbc, 0xa9, 0x46, 0x17, 0x00, 0x4d, 0x79,
                    0x69, 0xdf, 0xc4, 0x65, 0x2b, 0x1f, 0x30, 0xa7, 0xa8, 0xb7, 0x71, 0xc3, 0x48,
                    0x00, 0x15, 0x54, 0x83, 0x38, 0x0b,
                ]),
                app_namespace: "devnet".to_string(),
                service_name: "terminal".to_string(),
            }
        );
    }

    #[test]
    fn parse_command_parses_relay_intro_flags() {
        assert_eq!(
            parse_command([
                OsString::from("overlay-cli"),
                OsString::from("relay-intro"),
                OsString::from("--config"),
                OsString::from("devnet/configs/node-b.json"),
                OsString::from("--target"),
                OsString::from("tcp://127.0.0.1:4198"),
                OsString::from("--relay-node-id"),
                OsString::from("16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d"),
                OsString::from("--requester-node-id"),
                OsString::from("83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062"),
            ])
            .unwrap(),
            Command::RelayIntro {
                config_path: PathBuf::from("devnet/configs/node-b.json"),
                target: "tcp://127.0.0.1:4198".to_string(),
                relay_node_id: NodeId::from_bytes([
                    0x16, 0xf5, 0x2d, 0x6f, 0xea, 0x63, 0xef, 0x08, 0x64, 0x05, 0xaa, 0x71, 0xb5,
                    0x37, 0xdd, 0x48, 0x33, 0xbd, 0x0b, 0x36, 0xff, 0xe0, 0x54, 0xbe, 0x0f, 0xd0,
                    0x7f, 0xb5, 0x25, 0xaf, 0x15, 0x7d,
                ]),
                requester_node_id: NodeId::from_bytes([
                    0x83, 0x56, 0x1a, 0xdb, 0x39, 0x8f, 0xd8, 0x7f, 0x8e, 0x7e, 0xd8, 0x33, 0x1b,
                    0xff, 0x2f, 0xcb, 0x94, 0x57, 0x33, 0xcc, 0x30, 0x12, 0x87, 0x9c, 0xb9, 0xfa,
                    0xb0, 0x79, 0x28, 0x66, 0x70, 0x62,
                ]),
                expires_in_s: 300,
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

    fn unique_temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("overlay-cli-{label}-{suffix}"))
    }
}
