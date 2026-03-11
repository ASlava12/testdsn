use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process,
};

use overlay_core::{runtime::NodeRuntime, REPOSITORY_STAGE};
use serde::Serialize;
use serde_json::{json, Value};

use crate::signal::{process_exists, ShutdownSignal};

const STATUS_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeShutdownReason {
    NaturalEnd,
    SignalInterrupt,
    SignalTerminate,
}

impl From<ShutdownSignal> for RuntimeShutdownReason {
    fn from(value: ShutdownSignal) -> Self {
        match value {
            ShutdownSignal::Interrupt => Self::SignalInterrupt,
            ShutdownSignal::Terminate => Self::SignalTerminate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeLifecycleStatus {
    pub config_path: PathBuf,
    pub state_dir: PathBuf,
    pub status_file: PathBuf,
    pub lock_file: PathBuf,
    pub node_id: String,
    pub pid: u32,
    pub startup_count: u64,
    pub recovered_from_unclean_shutdown: bool,
    pub stale_lock_recovered: bool,
    pub previous_shutdown_clean: Option<bool>,
    pub clean_shutdown: bool,
    pub shutdown_requested: bool,
    pub shutdown_reason: Option<RuntimeShutdownReason>,
    pub last_start_unix_ms: u64,
    pub last_update_unix_ms: u64,
    pub last_shutdown_unix_ms: Option<u64>,
    pub last_shutdown_reason: Option<RuntimeShutdownReason>,
}

#[derive(Debug, Clone)]
struct OperatorStatePaths {
    config_path: PathBuf,
    state_dir: PathBuf,
    status_file: PathBuf,
    lock_file: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct PreviousStatus {
    node_id: Option<String>,
    startup_count: u64,
    clean_shutdown: Option<bool>,
    last_shutdown_unix_ms: Option<u64>,
    last_shutdown_reason: Option<RuntimeShutdownReason>,
}

#[derive(Debug)]
pub struct OperatorStateManager {
    lifecycle: RuntimeLifecycleStatus,
    lock_held: bool,
}

impl OperatorStateManager {
    pub fn acquire(
        config_path: &Path,
        node_id: String,
        timestamp_unix_ms: u64,
    ) -> Result<Self, String> {
        let paths = OperatorStatePaths::resolve(config_path)?;
        fs::create_dir_all(&paths.state_dir).map_err(|error| {
            format!(
                "failed to create operator state directory {}: {error}",
                paths.state_dir.display()
            )
        })?;

        let previous_status = read_previous_status(&paths.status_file)?;
        if let Some(previous_node_id) = previous_status.node_id.as_deref() {
            if previous_node_id != node_id {
                return Err(format!(
                    "operator state {} belongs to node_id {previous_node_id}, current config resolves to {node_id}",
                    paths.state_dir.display()
                ));
            }
        }

        let stale_lock_recovered = recover_or_reject_existing_lock(&paths.lock_file)?;
        write_lock_file(&paths, &node_id, timestamp_unix_ms)?;

        let previous_shutdown_clean = previous_status.clean_shutdown;
        let recovered_from_unclean_shutdown =
            stale_lock_recovered || previous_shutdown_clean == Some(false);

        Ok(Self {
            lifecycle: RuntimeLifecycleStatus {
                config_path: paths.config_path,
                state_dir: paths.state_dir,
                status_file: paths.status_file,
                lock_file: paths.lock_file,
                node_id,
                pid: process::id(),
                startup_count: previous_status.startup_count.saturating_add(1),
                recovered_from_unclean_shutdown,
                stale_lock_recovered,
                previous_shutdown_clean,
                clean_shutdown: false,
                shutdown_requested: false,
                shutdown_reason: None,
                last_start_unix_ms: timestamp_unix_ms,
                last_update_unix_ms: timestamp_unix_ms,
                last_shutdown_unix_ms: previous_status.last_shutdown_unix_ms,
                last_shutdown_reason: previous_status.last_shutdown_reason,
            },
            lock_held: true,
        })
    }

    pub fn lifecycle(&self) -> &RuntimeLifecycleStatus {
        &self.lifecycle
    }

    pub fn write_status(
        &mut self,
        runtime: &NodeRuntime,
        ticks_run: u64,
        timestamp_unix_ms: u64,
    ) -> Result<(), String> {
        self.lifecycle.last_update_unix_ms = timestamp_unix_ms;
        let payload = json!({
            "kind": "runtime_status",
            "version": STATUS_VERSION,
            "stage": REPOSITORY_STAGE,
            "updated_at_unix_ms": timestamp_unix_ms,
            "ticks_run": ticks_run,
            "lifecycle": &self.lifecycle,
            "health": runtime.health_snapshot(),
        });
        write_json_atomically(&self.lifecycle.status_file, &payload)
    }

    pub fn begin_shutdown(&mut self, reason: RuntimeShutdownReason, timestamp_unix_ms: u64) {
        self.lifecycle.shutdown_requested = true;
        self.lifecycle.shutdown_reason = Some(reason);
        self.lifecycle.last_update_unix_ms = timestamp_unix_ms;
    }

    pub fn finalize_clean_shutdown(
        &mut self,
        runtime: &NodeRuntime,
        ticks_run: u64,
        reason: RuntimeShutdownReason,
        timestamp_unix_ms: u64,
    ) -> Result<(), String> {
        self.begin_shutdown(reason, timestamp_unix_ms);
        self.lifecycle.clean_shutdown = true;
        self.lifecycle.last_shutdown_unix_ms = Some(timestamp_unix_ms);
        self.lifecycle.last_shutdown_reason = Some(reason);
        self.write_status(runtime, ticks_run, timestamp_unix_ms)?;
        self.release_lock()
    }

    pub fn status_file_path(config_path: &Path) -> Result<PathBuf, String> {
        Ok(OperatorStatePaths::resolve(config_path)?.status_file)
    }

    pub fn read_status_file(config_path: &Path) -> Result<String, String> {
        let status_file = Self::status_file_path(config_path)?;
        fs::read_to_string(&status_file).map_err(|error| {
            format!(
                "failed to read operator status file {}: {error}",
                status_file.display()
            )
        })
    }

    fn release_lock(&mut self) -> Result<(), String> {
        if !self.lock_held {
            return Ok(());
        }
        fs::remove_file(&self.lifecycle.lock_file).map_err(|error| {
            format!(
                "failed to remove operator lock file {}: {error}",
                self.lifecycle.lock_file.display()
            )
        })?;
        self.lock_held = false;
        Ok(())
    }
}

impl OperatorStatePaths {
    fn resolve(config_path: &Path) -> Result<Self, String> {
        let config_path = fs::canonicalize(config_path).map_err(|error| {
            format!(
                "failed to resolve config path {}: {error}",
                config_path.display()
            )
        })?;
        let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        let config_stem = config_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                format!(
                    "config path {} does not have a usable file stem",
                    config_path.display()
                )
            })?;
        let state_dir = config_dir.join(".overlay-runtime").join(config_stem);
        Ok(Self {
            status_file: state_dir.join("runtime-status.json"),
            lock_file: state_dir.join("runtime.lock"),
            config_path,
            state_dir,
        })
    }
}

fn read_previous_status(status_file: &Path) -> Result<PreviousStatus, String> {
    let bytes = match fs::read(status_file) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PreviousStatus::default())
        }
        Err(error) => {
            return Err(format!(
                "failed to read operator status file {}: {error}",
                status_file.display()
            ))
        }
    };
    let value = serde_json::from_slice::<Value>(&bytes).map_err(|error| {
        format!(
            "failed to parse operator status file {}: {error}",
            status_file.display()
        )
    })?;

    Ok(PreviousStatus {
        node_id: value
            .pointer("/lifecycle/node_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        startup_count: value
            .pointer("/lifecycle/startup_count")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        clean_shutdown: value
            .pointer("/lifecycle/clean_shutdown")
            .and_then(Value::as_bool),
        last_shutdown_unix_ms: value
            .pointer("/lifecycle/last_shutdown_unix_ms")
            .and_then(Value::as_u64),
        last_shutdown_reason: value
            .pointer("/lifecycle/last_shutdown_reason")
            .and_then(Value::as_str)
            .and_then(parse_shutdown_reason),
    })
}

fn recover_or_reject_existing_lock(lock_file: &Path) -> Result<bool, String> {
    let bytes = match fs::read(lock_file) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "failed to read operator lock file {}: {error}",
                lock_file.display()
            ))
        }
    };
    let value = serde_json::from_slice::<Value>(&bytes).map_err(|error| {
        format!(
            "failed to parse operator lock file {}: {error}",
            lock_file.display()
        )
    })?;
    let pid = value.get("pid").and_then(Value::as_u64).ok_or_else(|| {
        format!(
            "operator lock file {} did not contain a numeric pid",
            lock_file.display()
        )
    })?;

    if pid > u32::MAX as u64 {
        return Err(format!(
            "operator lock file {} contained pid {} that exceeds u32",
            lock_file.display(),
            pid
        ));
    }

    if process_exists(pid as u32) {
        return Err(format!(
            "runtime for this config already appears active with pid {} (lock file {})",
            pid,
            lock_file.display()
        ));
    }

    fs::remove_file(lock_file).map_err(|error| {
        format!(
            "failed to remove stale operator lock file {}: {error}",
            lock_file.display()
        )
    })?;
    Ok(true)
}

fn write_lock_file(
    paths: &OperatorStatePaths,
    node_id: &str,
    timestamp_unix_ms: u64,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&paths.lock_file)
        .map_err(|error| {
            format!(
                "failed to create operator lock file {}: {error}",
                paths.lock_file.display()
            )
        })?;

    let payload = json!({
        "version": STATUS_VERSION,
        "stage": REPOSITORY_STAGE,
        "config_path": paths.config_path,
        "node_id": node_id,
        "pid": process::id(),
        "started_unix_ms": timestamp_unix_ms,
    });
    let bytes = serde_json::to_vec(&payload)
        .map_err(|error| format!("failed to encode lock file: {error}"))?;
    file.write_all(&bytes).map_err(|error| {
        format!(
            "failed to write operator lock file {}: {error}",
            paths.lock_file.display()
        )
    })
}

fn write_json_atomically(path: &Path, value: &Value) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "status file {} did not have a parent directory",
            path.display()
        ));
    };
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create operator status parent directory {}: {error}",
            parent.display()
        )
    })?;

    let temp_path = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("runtime-status"),
        process::id()
    ));
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("failed to encode operator status JSON: {error}"))?;
    fs::write(&temp_path, bytes).map_err(|error| {
        format!(
            "failed to write temporary operator status file {}: {error}",
            temp_path.display()
        )
    })?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| {
            format!(
                "failed to replace operator status file {}: {error}",
                path.display()
            )
        })?;
    }
    fs::rename(&temp_path, path).map_err(|error| {
        format!(
            "failed to publish operator status file {}: {error}",
            path.display()
        )
    })
}

fn parse_shutdown_reason(value: &str) -> Option<RuntimeShutdownReason> {
    match value {
        "natural_end" => Some(RuntimeShutdownReason::NaturalEnd),
        "signal_interrupt" => Some(RuntimeShutdownReason::SignalInterrupt),
        "signal_terminate" => Some(RuntimeShutdownReason::SignalTerminate),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde_json::json;

    use super::{
        parse_shutdown_reason, OperatorStateManager, RuntimeShutdownReason, STATUS_VERSION,
    };

    fn unique_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "overlay-cli-operator-state-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    #[test]
    fn status_file_path_is_derived_from_config_path() {
        let dir = unique_test_dir("path");
        let config_path = dir.join("node-a.json");
        fs::write(&config_path, b"{}").expect("config file should be written");

        let status_path = OperatorStateManager::status_file_path(&config_path)
            .expect("status path should resolve");
        assert!(status_path.ends_with(".overlay-runtime/node-a/runtime-status.json"));
    }

    #[test]
    fn status_reader_reports_missing_file() {
        let dir = unique_test_dir("missing");
        let config_path = dir.join("node-a.json");
        fs::write(&config_path, b"{}").expect("config file should be written");

        let error = OperatorStateManager::read_status_file(&config_path)
            .expect_err("missing status file should fail");
        assert!(error.contains("runtime-status.json"));
    }

    #[test]
    fn parse_shutdown_reason_accepts_known_values() {
        assert_eq!(
            parse_shutdown_reason("natural_end"),
            Some(RuntimeShutdownReason::NaturalEnd)
        );
        assert_eq!(
            parse_shutdown_reason("signal_interrupt"),
            Some(RuntimeShutdownReason::SignalInterrupt)
        );
        assert_eq!(
            parse_shutdown_reason("signal_terminate"),
            Some(RuntimeShutdownReason::SignalTerminate)
        );
        assert_eq!(parse_shutdown_reason("unexpected"), None);
    }

    #[test]
    fn acquire_rejects_live_lock_file() {
        let dir = unique_test_dir("live-lock");
        let config_path = dir.join("node-a.json");
        fs::write(&config_path, b"{}").expect("config file should be written");

        let state_dir = dir.join(".overlay-runtime").join("node-a");
        fs::create_dir_all(&state_dir).expect("state dir should be created");
        let lock_file = state_dir.join("runtime.lock");
        fs::write(
            &lock_file,
            serde_json::to_vec(&json!({
                "version": STATUS_VERSION,
                "pid": std::process::id(),
            }))
            .expect("lock payload should encode"),
        )
        .expect("lock file should be written");

        let error = OperatorStateManager::acquire(&config_path, "node-id".to_string(), 1)
            .expect_err("live lock should be rejected");
        assert!(error.contains("already appears active"));
    }

    #[test]
    fn acquire_recovers_stale_lock_file() {
        let dir = unique_test_dir("stale-lock");
        let config_path = dir.join("node-a.json");
        fs::write(&config_path, b"{}").expect("config file should be written");

        let state_dir = dir.join(".overlay-runtime").join("node-a");
        fs::create_dir_all(&state_dir).expect("state dir should be created");
        let lock_file = state_dir.join("runtime.lock");
        fs::write(
            &lock_file,
            serde_json::to_vec(&json!({
                "version": STATUS_VERSION,
                "pid": u32::MAX,
            }))
            .expect("lock payload should encode"),
        )
        .expect("stale lock file should be written");

        let manager = OperatorStateManager::acquire(&config_path, "node-id".to_string(), 7)
            .expect("stale lock should be recovered");
        assert!(manager.lifecycle().stale_lock_recovered);
        let lock = fs::read_to_string(&lock_file).expect("replacement lock file should exist");
        drop(manager);
        assert!(lock.contains(&std::process::id().to_string()));
    }
}
