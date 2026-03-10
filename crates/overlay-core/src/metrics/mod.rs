//! Bounded in-memory observability surfaces for Milestone 9.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::identity::NodeId;

pub const DEFAULT_MAX_LOG_ENTRIES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub max_log_entries: usize,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            max_log_entries: DEFAULT_MAX_LOG_ENTRIES,
        }
    }
}

impl ObservabilityConfig {
    pub fn validate(self) -> Result<Self, ObservabilityError> {
        if self.max_log_entries == 0 {
            return Err(ObservabilityError::ZeroLimit {
                field: "max_log_entries",
            });
        }

        Ok(self)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ObservabilityError {
    #[error("observability config limit {field} must be non-zero")]
    ZeroLimit { field: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogContext {
    pub timestamp_unix_ms: u64,
    pub node_id: NodeId,
    pub correlation_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogComponent {
    Identity,
    Runtime,
    Bootstrap,
    Peer,
    Session,
    Rendezvous,
    Relay,
    Routing,
    Service,
    Metrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredLogEntry {
    pub timestamp_unix_ms: u64,
    pub node_id: NodeId,
    pub correlation_id: u64,
    pub component: LogComponent,
    pub event: String,
    pub result: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub active_peers: u64,
    pub established_sessions: u64,
    pub publish_presence_total: u64,
    pub lookup_total: u64,
    pub lookup_success_total: u64,
    pub lookup_latency_ms: Option<u64>,
    pub relay_bind_total: u64,
    pub path_switch_total: u64,
    pub probe_rtt_ms: Option<u32>,
    pub probe_loss_ratio: Option<u32>,
    pub dropped_rate_limited_total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observability {
    metrics: MetricsSnapshot,
    logs: VecDeque<StructuredLogEntry>,
    max_log_entries: usize,
}

impl Default for Observability {
    fn default() -> Self {
        Self::new(ObservabilityConfig::default())
            .expect("default observability config should be valid")
    }
}

impl Observability {
    pub fn new(config: ObservabilityConfig) -> Result<Self, ObservabilityError> {
        let config = config.validate()?;
        Ok(Self {
            metrics: MetricsSnapshot::default(),
            logs: VecDeque::new(),
            max_log_entries: config.max_log_entries,
        })
    }

    pub fn metrics(&self) -> &MetricsSnapshot {
        &self.metrics
    }

    pub fn logs(&self) -> &VecDeque<StructuredLogEntry> {
        &self.logs
    }

    pub fn latest_log(&self) -> Option<&StructuredLogEntry> {
        self.logs.back()
    }

    pub fn set_active_peers(&mut self, active_peers: usize) {
        self.metrics.active_peers = active_peers as u64;
    }

    pub fn set_established_sessions(&mut self, established_sessions: usize) {
        self.metrics.established_sessions = established_sessions as u64;
    }

    pub fn note_publish_presence(&mut self) {
        self.metrics.publish_presence_total = self.metrics.publish_presence_total.saturating_add(1);
    }

    pub fn note_lookup(&mut self, success: bool, latency_ms: u64) {
        self.metrics.lookup_total = self.metrics.lookup_total.saturating_add(1);
        if success {
            self.metrics.lookup_success_total = self.metrics.lookup_success_total.saturating_add(1);
        }
        self.metrics.lookup_latency_ms = Some(latency_ms);
    }

    pub fn note_relay_bind(&mut self) {
        self.metrics.relay_bind_total = self.metrics.relay_bind_total.saturating_add(1);
    }

    pub fn note_path_switch(&mut self) {
        self.metrics.path_switch_total = self.metrics.path_switch_total.saturating_add(1);
    }

    pub fn observe_probe_feedback(&mut self, obs_rtt_ms: Option<u32>, loss_ppm: u32) {
        if let Some(obs_rtt_ms) = obs_rtt_ms {
            self.metrics.probe_rtt_ms = Some(obs_rtt_ms);
        }
        self.metrics.probe_loss_ratio = Some(loss_ppm);
    }

    pub fn note_rate_limited_drop(&mut self) {
        self.metrics.dropped_rate_limited_total =
            self.metrics.dropped_rate_limited_total.saturating_add(1);
    }

    pub fn push_log(
        &mut self,
        context: LogContext,
        component: LogComponent,
        event: impl Into<String>,
        result: impl Into<String>,
    ) {
        if self.logs.len() == self.max_log_entries {
            self.logs.pop_front();
        }
        self.logs.push_back(StructuredLogEntry {
            timestamp_unix_ms: context.timestamp_unix_ms,
            node_id: context.node_id,
            correlation_id: context.correlation_id,
            component,
            event: event.into(),
            result: result.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{LogComponent, LogContext, Observability, ObservabilityConfig, ObservabilityError};
    use crate::identity::NodeId;

    #[test]
    fn observability_config_rejects_zero_log_entries() {
        let error = Observability::new(ObservabilityConfig { max_log_entries: 0 })
            .expect_err("zero log capacity must be rejected");

        assert_eq!(
            error,
            ObservabilityError::ZeroLimit {
                field: "max_log_entries"
            }
        );
    }

    #[test]
    fn structured_log_store_is_bounded() {
        let node_id = NodeId::from_bytes([7_u8; 32]);
        let mut observability = Observability::new(ObservabilityConfig { max_log_entries: 2 })
            .expect("config should be valid");

        for correlation_id in 1..=3 {
            observability.push_log(
                LogContext {
                    timestamp_unix_ms: correlation_id,
                    node_id,
                    correlation_id,
                },
                LogComponent::Metrics,
                "tick",
                "ok",
            );
        }

        assert_eq!(observability.logs().len(), 2);
        assert_eq!(observability.logs()[0].correlation_id, 2);
        assert_eq!(observability.logs()[1].correlation_id, 3);
    }

    #[test]
    fn metrics_snapshot_tracks_counters_gauges_and_samples() {
        let node_id = NodeId::from_bytes([9_u8; 32]);
        let mut observability = Observability::default();

        observability.set_active_peers(5);
        observability.set_established_sessions(3);
        observability.note_publish_presence();
        observability.note_lookup(true, 42);
        observability.note_lookup(false, 77);
        observability.note_relay_bind();
        observability.note_path_switch();
        observability.observe_probe_feedback(Some(80), 500_000);
        observability.note_rate_limited_drop();
        observability.push_log(
            LogContext {
                timestamp_unix_ms: 1_700_000_000_123,
                node_id,
                correlation_id: 11,
            },
            LogComponent::Relay,
            "bind_tunnel",
            "opened",
        );

        assert_eq!(observability.metrics().active_peers, 5);
        assert_eq!(observability.metrics().established_sessions, 3);
        assert_eq!(observability.metrics().publish_presence_total, 1);
        assert_eq!(observability.metrics().lookup_total, 2);
        assert_eq!(observability.metrics().lookup_success_total, 1);
        assert_eq!(observability.metrics().lookup_latency_ms, Some(77));
        assert_eq!(observability.metrics().relay_bind_total, 1);
        assert_eq!(observability.metrics().path_switch_total, 1);
        assert_eq!(observability.metrics().probe_rtt_ms, Some(80));
        assert_eq!(observability.metrics().probe_loss_ratio, Some(500_000));
        assert_eq!(observability.metrics().dropped_rate_limited_total, 1);
        assert_eq!(
            observability
                .latest_log()
                .expect("log should be present")
                .component,
            LogComponent::Relay
        );
    }
}
