//! Top-level node configuration validation and conservative subsystem projection.

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    peer::{PeerStoreConfig, PeerStoreError},
    relay::{RelayConfig, RelayError},
    rendezvous::{RendezvousConfig, RendezvousError},
    routing::{PathProbeConfig, RoutingError},
    service::{ServiceConfig, ServiceError},
    transport::{TransportBufferConfig, TransportBufferError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTemplateProfile {
    UserNode,
    RelayCapable,
    BootstrapSeed,
}

impl ConfigTemplateProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UserNode => "user-node",
            Self::RelayCapable => "relay-capable",
            Self::BootstrapSeed => "bootstrap-seed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "user-node" => Some(Self::UserNode),
            "relay-capable" => Some(Self::RelayCapable),
            "bootstrap-seed" => Some(Self::BootstrapSeed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverlayConfig {
    pub node_key_path: PathBuf,
    pub bootstrap_sources: Vec<String>,
    #[serde(default)]
    pub tcp_listener_addr: Option<String>,
    pub max_total_neighbors: usize,
    pub max_presence_records: usize,
    pub max_service_records: usize,
    pub presence_ttl_s: u64,
    pub epoch_duration_s: u64,
    pub path_probe_interval_ms: u64,
    pub max_transport_buffer_bytes: usize,
    pub relay_mode: bool,
    pub log_level: LogLevel,
}

impl OverlayConfig {
    pub fn template() -> Self {
        Self::template_for_profile(ConfigTemplateProfile::UserNode)
    }

    pub fn template_for_profile(profile: ConfigTemplateProfile) -> Self {
        let mut config = Self {
            node_key_path: PathBuf::from("./keys/node.key"),
            bootstrap_sources: vec!["./bootstrap/node-foundation.json".to_string()],
            tcp_listener_addr: Some("127.0.0.1:4101".to_string()),
            max_total_neighbors: 8,
            max_presence_records: 64,
            max_service_records: 16,
            presence_ttl_s: 120,
            epoch_duration_s: 60,
            path_probe_interval_ms: 5_000,
            max_transport_buffer_bytes: 65_536,
            relay_mode: false,
            log_level: LogLevel::Info,
        };
        if matches!(profile, ConfigTemplateProfile::RelayCapable) {
            config.relay_mode = true;
        }
        config
    }

    pub fn validate(self) -> Result<Self, ConfigError> {
        if path_is_empty(&self.node_key_path) {
            return Err(ConfigError::EmptyField {
                field: "node_key_path",
            });
        }
        if self.bootstrap_sources.is_empty() {
            return Err(ConfigError::EmptyField {
                field: "bootstrap_sources",
            });
        }
        for (index, source) in self.bootstrap_sources.iter().enumerate() {
            if source.trim().is_empty() {
                return Err(ConfigError::EmptyBootstrapSource { index });
            }
            if !bootstrap_source_supported(source) {
                return Err(ConfigError::UnsupportedBootstrapSource {
                    index,
                    value: source.trim().to_string(),
                });
            }
        }
        if self
            .tcp_listener_addr
            .as_ref()
            .is_some_and(|addr| addr.trim().is_empty())
        {
            return Err(ConfigError::EmptyField {
                field: "tcp_listener_addr",
            });
        }
        if let Some(addr) = self.tcp_listener_addr.as_deref() {
            addr.trim().parse::<SocketAddr>().map_err(|detail| {
                ConfigError::InvalidTcpListenerAddr {
                    detail: detail.to_string(),
                }
            })?;
        }

        for (field, value) in [
            ("max_total_neighbors", self.max_total_neighbors as u64),
            ("max_presence_records", self.max_presence_records as u64),
            ("max_service_records", self.max_service_records as u64),
            ("presence_ttl_s", self.presence_ttl_s),
            ("epoch_duration_s", self.epoch_duration_s),
            ("path_probe_interval_ms", self.path_probe_interval_ms),
            (
                "max_transport_buffer_bytes",
                self.max_transport_buffer_bytes as u64,
            ),
        ] {
            if value == 0 {
                return Err(ConfigError::ZeroLimit { field });
            }
        }

        self.peer_store_config().validate()?;
        self.rendezvous_config().validate()?;
        self.service_config().validate()?;
        self.path_probe_config().validate()?;
        self.transport_buffer_config().validate()?;
        self.relay_config().validate()?;

        Ok(self)
    }

    pub fn peer_store_config(&self) -> PeerStoreConfig {
        let defaults = PeerStoreConfig::default();
        PeerStoreConfig {
            max_neighbors: self.max_total_neighbors,
            max_relay_neighbors: defaults.max_relay_neighbors.min(self.max_total_neighbors),
            max_neighbors_per_transport: defaults
                .max_neighbors_per_transport
                .min(self.max_total_neighbors),
        }
    }

    pub fn rendezvous_config(&self) -> RendezvousConfig {
        RendezvousConfig {
            max_published_records: self.max_presence_records,
            ..RendezvousConfig::default()
        }
    }

    pub fn service_config(&self) -> ServiceConfig {
        ServiceConfig {
            max_registered_services: self.max_service_records,
            ..ServiceConfig::default()
        }
    }

    pub fn path_probe_config(&self) -> PathProbeConfig {
        PathProbeConfig {
            path_probe_interval_ms: self.path_probe_interval_ms,
        }
    }

    pub fn transport_buffer_config(&self) -> TransportBufferConfig {
        TransportBufferConfig {
            max_buffer_bytes: self.max_transport_buffer_bytes,
        }
    }

    pub fn relay_config(&self) -> RelayConfig {
        RelayConfig::default().with_relay_mode(self.relay_mode)
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config field {field} must not be empty; start from `overlay-cli config-template --profile user-node` or see docs/CONFIG_EXAMPLES.md")]
    EmptyField { field: &'static str },
    #[error("config bootstrap_sources[{index}] must not be empty; use a local .json path, file:<path>, or http://host[:port]/path with optional #sha256=<hex> and/or #ed25519=<hex> pins")]
    EmptyBootstrapSource { index: usize },
    #[error("config bootstrap_sources[{index}] is not a supported source: {value}; accepted forms are local .json paths, file:<path>, or http://host[:port]/path with optional #sha256=<hex> and/or #ed25519=<hex> pins")]
    UnsupportedBootstrapSource { index: usize, value: String },
    #[error("config tcp_listener_addr must be a host:port socket address such as 127.0.0.1:4101: {detail}")]
    InvalidTcpListenerAddr { detail: String },
    #[error("config limit {field} must be non-zero; see docs/CONFIG_EXAMPLES.md for the current bounded defaults")]
    ZeroLimit { field: &'static str },
    #[error(transparent)]
    PeerStore(#[from] PeerStoreError),
    #[error(transparent)]
    Rendezvous(#[from] RendezvousError),
    #[error(transparent)]
    Service(#[from] ServiceError),
    #[error(transparent)]
    Routing(#[from] RoutingError),
    #[error(transparent)]
    Transport(#[from] TransportBufferError),
    #[error(transparent)]
    Relay(#[from] RelayError),
}

fn path_is_empty(path: &Path) -> bool {
    path.as_os_str().is_empty()
}

fn bootstrap_source_supported(source: &str) -> bool {
    let trimmed = source.trim();
    let Some((trimmed, _pins)) = split_bootstrap_source_pins(trimmed) else {
        return false;
    };
    if let Some(path) = trimmed.strip_prefix("file:") {
        return !Path::new(path.trim()).as_os_str().is_empty();
    }

    if parse_http_bootstrap_source(trimmed).is_some() {
        return true;
    }

    let path = Path::new(trimmed);
    path.extension().and_then(|ext| ext.to_str()) == Some("json") && !trimmed.contains("://")
}

fn parse_http_bootstrap_source(source: &str) -> Option<()> {
    let (source, _pins) = split_bootstrap_source_pins(source)?;
    let remainder = source.strip_prefix("http://")?;
    if remainder.is_empty() {
        return None;
    }

    let authority = remainder
        .split_once('/')
        .map(|(authority, _)| authority)
        .unwrap_or(remainder)
        .trim();
    if authority.is_empty() {
        return None;
    }

    http_authority_to_socket_addr(authority)
}

fn http_authority_to_socket_addr(authority: &str) -> Option<()> {
    if authority.starts_with('[') {
        let close = authority.find(']')?;
        let suffix = authority.get(close + 1..)?;
        if suffix.is_empty() {
            return Some(());
        }
        if suffix.starts_with(':') && suffix.len() > 1 {
            return Some(());
        }
        return None;
    }

    if authority.contains(':') {
        return Some(());
    }

    Some(())
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BootstrapSourcePins {
    expected_sha256_hex: Option<String>,
    trusted_ed25519_public_key_hex: Option<String>,
}

fn split_bootstrap_source_pins(source: &str) -> Option<(&str, BootstrapSourcePins)> {
    let (base, fragment) = match source.split_once('#') {
        Some((base, fragment)) => (base.trim(), Some(fragment.trim())),
        None => (source.trim(), None),
    };
    if base.is_empty() {
        return None;
    }

    let mut pins = BootstrapSourcePins::default();
    if let Some(fragment) = fragment {
        if fragment.is_empty() {
            return None;
        }
        for part in fragment.split('&') {
            let (key, value) = part.split_once('=')?;
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            match key {
                "sha256" => {
                    if value.len() != 64 || pins.expected_sha256_hex.is_some() {
                        return None;
                    }
                    pins.expected_sha256_hex = Some(value.to_ascii_lowercase());
                }
                "ed25519" => {
                    if value.len() != 64 || pins.trusted_ed25519_public_key_hex.is_some() {
                        return None;
                    }
                    pins.trusted_ed25519_public_key_hex = Some(value.to_ascii_lowercase());
                }
                _ => return None,
            }
        }
    }

    Some((base, pins))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{ConfigError, ConfigTemplateProfile, LogLevel, OverlayConfig};

    fn sample_config() -> OverlayConfig {
        OverlayConfig {
            node_key_path: PathBuf::from("keys/node.pem"),
            bootstrap_sources: vec![
                "bootstrap.json".to_string(),
                "http://127.0.0.1:4201/bootstrap.json".to_string(),
            ],
            tcp_listener_addr: None,
            max_total_neighbors: 16,
            max_presence_records: 1_024,
            max_service_records: 256,
            presence_ttl_s: 1_800,
            epoch_duration_s: 900,
            path_probe_interval_ms: 5_000,
            max_transport_buffer_bytes: 65_536,
            relay_mode: false,
            log_level: LogLevel::Info,
        }
    }

    #[test]
    fn overlay_config_template_is_valid_with_current_defaults() {
        let config = OverlayConfig::template()
            .validate()
            .expect("template config should be valid");

        assert_eq!(config.node_key_path, PathBuf::from("./keys/node.key"));
        assert_eq!(
            config.bootstrap_sources,
            vec!["./bootstrap/node-foundation.json".to_string()]
        );
        assert_eq!(config.tcp_listener_addr.as_deref(), Some("127.0.0.1:4101"));
        assert_eq!(config.max_total_neighbors, 8);
        assert_eq!(config.max_presence_records, 64);
        assert_eq!(config.max_service_records, 16);
        assert_eq!(config.presence_ttl_s, 120);
        assert_eq!(config.epoch_duration_s, 60);
        assert_eq!(config.path_probe_interval_ms, 5_000);
        assert_eq!(config.max_transport_buffer_bytes, 65_536);
        assert!(!config.relay_mode);
        assert_eq!(config.log_level, LogLevel::Info);
    }

    #[test]
    fn template_profiles_apply_expected_profile_defaults() {
        let relay = OverlayConfig::template_for_profile(ConfigTemplateProfile::RelayCapable)
            .validate()
            .expect("relay-capable template should be valid");
        assert!(relay.relay_mode);

        let bootstrap_seed =
            OverlayConfig::template_for_profile(ConfigTemplateProfile::BootstrapSeed)
                .validate()
                .expect("bootstrap-seed template should be valid");
        assert!(!bootstrap_seed.relay_mode);
        assert_eq!(
            bootstrap_seed.bootstrap_sources,
            vec!["./bootstrap/node-foundation.json".to_string()]
        );
    }

    #[test]
    fn overlay_config_template_serializes_to_the_expected_json_shape() {
        let value = serde_json::to_value(OverlayConfig::template())
            .expect("template config should serialize");

        assert_eq!(
            value,
            json!({
                "node_key_path": "./keys/node.key",
                "bootstrap_sources": ["./bootstrap/node-foundation.json"],
                "tcp_listener_addr": "127.0.0.1:4101",
                "max_total_neighbors": 8,
                "max_presence_records": 64,
                "max_service_records": 16,
                "presence_ttl_s": 120,
                "epoch_duration_s": 60,
                "path_probe_interval_ms": 5000,
                "max_transport_buffer_bytes": 65536,
                "relay_mode": false,
                "log_level": "info"
            })
        );
    }

    #[test]
    fn overlay_config_validates_and_projects_subsystem_configs() {
        let config = sample_config().validate().expect("config should be valid");

        let peer = config.peer_store_config();
        assert_eq!(peer.max_neighbors, 16);
        assert_eq!(peer.max_relay_neighbors, 4);
        assert_eq!(peer.max_neighbors_per_transport, 8);

        let rendezvous = config.rendezvous_config();
        assert_eq!(rendezvous.max_published_records, 1_024);

        let service = config.service_config();
        assert_eq!(service.max_registered_services, 256);

        let routing = config.path_probe_config();
        assert_eq!(routing.path_probe_interval_ms, 5_000);

        let transport = config.transport_buffer_config();
        assert_eq!(transport.max_buffer_bytes, 65_536);

        let relay = config.relay_config();
        assert!(!relay.relay_mode);
    }

    #[test]
    fn overlay_config_rejects_empty_required_fields() {
        let mut missing_key_path = sample_config();
        missing_key_path.node_key_path = PathBuf::new();
        assert!(matches!(
            missing_key_path.validate(),
            Err(ConfigError::EmptyField {
                field: "node_key_path"
            })
        ));

        let mut missing_sources = sample_config();
        missing_sources.bootstrap_sources.clear();
        assert!(matches!(
            missing_sources.validate(),
            Err(ConfigError::EmptyField {
                field: "bootstrap_sources"
            })
        ));

        let mut blank_source = sample_config();
        blank_source.bootstrap_sources[1] = "   ".to_string();
        assert!(matches!(
            blank_source.validate(),
            Err(ConfigError::EmptyBootstrapSource { index: 1 })
        ));

        let mut unsupported_source = sample_config();
        unsupported_source.bootstrap_sources[0] =
            "https://bootstrap.example.net/bootstrap.json".to_string();
        assert!(matches!(
            unsupported_source.validate(),
            Err(ConfigError::UnsupportedBootstrapSource { index: 0, .. })
        ));

        let mut blank_listener = sample_config();
        blank_listener.tcp_listener_addr = Some("   ".to_string());
        assert!(matches!(
            blank_listener.validate(),
            Err(ConfigError::EmptyField {
                field: "tcp_listener_addr"
            })
        ));

        let mut invalid_listener = sample_config();
        invalid_listener.tcp_listener_addr = Some("localhost".to_string());
        assert!(matches!(
            invalid_listener.validate(),
            Err(ConfigError::InvalidTcpListenerAddr { .. })
        ));

        let mut invalid_pinned_http_source = sample_config();
        invalid_pinned_http_source.bootstrap_sources[1] =
            "http://127.0.0.1:4201/bootstrap.json#sha256=xyz".to_string();
        assert!(matches!(
            invalid_pinned_http_source.validate(),
            Err(ConfigError::UnsupportedBootstrapSource { index: 1, .. })
        ));

        let mut invalid_ed25519_source = sample_config();
        invalid_ed25519_source.bootstrap_sources[1] =
            "http://127.0.0.1:4201/bootstrap.json#ed25519=xyz".to_string();
        assert!(matches!(
            invalid_ed25519_source.validate(),
            Err(ConfigError::UnsupportedBootstrapSource { index: 1, .. })
        ));

        let mut duplicate_fragment_key = sample_config();
        duplicate_fragment_key.bootstrap_sources[1] = "http://127.0.0.1:4201/bootstrap.json#sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef&sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string();
        assert!(matches!(
            duplicate_fragment_key.validate(),
            Err(ConfigError::UnsupportedBootstrapSource { index: 1, .. })
        ));

        let mut unknown_fragment_key = sample_config();
        unknown_fragment_key.bootstrap_sources[1] =
            "http://127.0.0.1:4201/bootstrap.json#trust=on".to_string();
        assert!(matches!(
            unknown_fragment_key.validate(),
            Err(ConfigError::UnsupportedBootstrapSource { index: 1, .. })
        ));
    }

    #[test]
    fn overlay_config_rejects_zero_limits() {
        for field in [
            "max_total_neighbors",
            "max_presence_records",
            "max_service_records",
            "presence_ttl_s",
            "epoch_duration_s",
            "path_probe_interval_ms",
            "max_transport_buffer_bytes",
        ] {
            let mut config = sample_config();
            match field {
                "max_total_neighbors" => config.max_total_neighbors = 0,
                "max_presence_records" => config.max_presence_records = 0,
                "max_service_records" => config.max_service_records = 0,
                "presence_ttl_s" => config.presence_ttl_s = 0,
                "epoch_duration_s" => config.epoch_duration_s = 0,
                "path_probe_interval_ms" => config.path_probe_interval_ms = 0,
                "max_transport_buffer_bytes" => config.max_transport_buffer_bytes = 0,
                _ => unreachable!(),
            }

            assert!(matches!(
                config.validate(),
                Err(ConfigError::ZeroLimit { field: actual }) if actual == field
            ));
        }
    }

    #[test]
    fn peer_store_projection_caps_transport_and_relay_limits_to_total_neighbors() {
        let config = OverlayConfig {
            max_total_neighbors: 3,
            ..sample_config()
        }
        .validate()
        .expect("config should be valid");

        let peer = config.peer_store_config();
        assert_eq!(peer.max_neighbors, 3);
        assert_eq!(peer.max_relay_neighbors, 3);
        assert_eq!(peer.max_neighbors_per_transport, 3);
    }

    #[test]
    fn overlay_config_accepts_http_bootstrap_with_sha256_pin() {
        let mut config = sample_config();
        config.bootstrap_sources[1] = "http://127.0.0.1:4201/bootstrap.json#sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();

        let validated = config
            .validate()
            .expect("pinned http source should validate");
        assert_eq!(
            validated.bootstrap_sources[1],
            "http://127.0.0.1:4201/bootstrap.json#sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn overlay_config_accepts_http_bootstrap_with_ed25519_pin() {
        let mut config = sample_config();
        config.bootstrap_sources[1] = "http://127.0.0.1:4201/bootstrap.json#ed25519=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();

        let validated = config
            .validate()
            .expect("signed http source should validate");
        assert_eq!(
            validated.bootstrap_sources[1],
            "http://127.0.0.1:4201/bootstrap.json#ed25519=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn overlay_config_accepts_http_bootstrap_with_sha256_and_ed25519_pins() {
        let mut config = sample_config();
        config.bootstrap_sources[1] = "http://127.0.0.1:4201/bootstrap.json#sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef&ed25519=abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".to_string();

        let validated = config
            .validate()
            .expect("combined pinned http source should validate");
        assert_eq!(
            validated.bootstrap_sources[1],
            "http://127.0.0.1:4201/bootstrap.json#sha256=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef&ed25519=abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
        );
    }

    #[test]
    fn log_level_uses_lowercase_serde_names() {
        let json = serde_json::to_string(&LogLevel::Debug).expect("log level should serialize");
        assert_eq!(json, "\"debug\"");

        let parsed: LogLevel = serde_json::from_str("\"warn\"").expect("log level should parse");
        assert_eq!(parsed, LogLevel::Warn);
    }

    #[test]
    fn overlay_config_rejects_unknown_fields() {
        let error = serde_json::from_value::<OverlayConfig>(json!({
            "node_key_path": "keys/node.pem",
            "bootstrap_sources": ["bootstrap.json"],
            "max_total_neighbors": 8,
            "max_presence_records": 64,
            "max_service_records": 16,
            "presence_ttl_s": 120,
            "epoch_duration_s": 60,
            "path_probe_interval_ms": 5000,
            "max_transport_buffer_bytes": 65536,
            "relay_mode": false,
            "log_level": "info",
            "unexpected_operator_knob": true
        }))
        .expect_err("unknown config fields should be rejected");
        assert!(error.to_string().contains("unknown field"));
    }
}
