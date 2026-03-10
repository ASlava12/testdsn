//! Path metrics, deterministic scoring, and switch hysteresis baseline for Milestone 7.

use std::collections::{BTreeMap, VecDeque};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::{
    error::FrameError,
    wire::{Message, MessageType, MAX_FRAME_BODY_LEN},
};

pub const RTT_EWMA_ALPHA_NUMERATOR: u64 = 2;
pub const RTT_EWMA_ALPHA_DENOMINATOR: u64 = 10;
pub const LOSS_EWMA_ALPHA_NUMERATOR: u64 = 1;
pub const LOSS_EWMA_ALPHA_DENOMINATOR: u64 = 10;
pub const JITTER_EWMA_ALPHA_NUMERATOR: u64 = 1;
pub const JITTER_EWMA_ALPHA_DENOMINATOR: u64 = 10;

pub const DEFAULT_MIN_ABSOLUTE_IMPROVEMENT_MS: u64 = 10;
pub const DEFAULT_MIN_RELATIVE_IMPROVEMENT_PERCENT: u8 = 15;
pub const DEFAULT_MIN_DWELL_TIME_S: u64 = 30;
pub const DEFAULT_MAX_SWITCHES_PER_MINUTE: usize = 2;
pub const DEFAULT_PATH_PROBE_INTERVAL_MS: u64 = 5_000;
pub const DEFAULT_MAX_IN_FLIGHT_PATH_PROBES_PER_PATH: usize = 4;
pub const DEFAULT_PATH_PROBE_LOSS_WINDOW_SAMPLES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathProbeConfig {
    pub path_probe_interval_ms: u64,
}

impl Default for PathProbeConfig {
    fn default() -> Self {
        Self {
            path_probe_interval_ms: DEFAULT_PATH_PROBE_INTERVAL_MS,
        }
    }
}

impl PathProbeConfig {
    pub fn validate(self) -> Result<Self, RoutingError> {
        if self.path_probe_interval_ms == 0 {
            return Err(RoutingError::ZeroLimit {
                field: "path_probe_interval_ms",
            });
        }

        Ok(self)
    }
}

#[derive(Debug, Error)]
pub enum RoutingMessageError {
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Frame(#[from] FrameError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathProbe {
    pub path_id: u64,
    pub probe_id: u64,
    pub sent_at_unix_ms: u64,
}

impl PathProbe {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RoutingMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RoutingMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for PathProbe {
    const TYPE: MessageType = MessageType::PathProbe;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathProbeResult {
    pub path_id: u64,
    pub probe_id: u64,
}

impl PathProbeResult {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, RoutingMessageError> {
        canonical_message_bytes(self)
    }

    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RoutingMessageError> {
        parse_message_bytes(bytes)
    }
}

impl Message for PathProbeResult {
    const TYPE: MessageType = MessageType::PathProbeResult;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathMetrics {
    pub est_rtt_ms: u32,
    pub obs_rtt_ms: u32,
    pub jitter_ms: u32,
    pub loss_ppm: u32,
    pub relay_hops: u8,
    pub censorship_risk_level: u8,
    pub diversity_bonus: u8,
}

impl PathMetrics {
    pub fn score(&self) -> u64 {
        let relay_hops = u64::from(self.relay_hops);
        let censorship_risk_level = u64::from(self.censorship_risk_level);
        let diversity_bonus = u64::from(self.diversity_bonus);

        (8 * u64::from(self.obs_rtt_ms))
            + (2 * u64::from(self.est_rtt_ms))
            + u64::from(self.jitter_ms)
            + ((u64::from(self.loss_ppm) / 1_000) * 25)
            + (25 * relay_hops)
            + (100 * censorship_risk_level)
            - (20 * diversity_bonus)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathObservation {
    pub obs_rtt_ms: u32,
    pub loss_ppm: u32,
    pub jitter_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathProbeFeedback {
    pub obs_rtt_ms: Option<u32>,
    pub loss_ppm: u32,
    pub jitter_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathState {
    pub path_id: u64,
    pub metrics: PathMetrics,
}

impl PathState {
    pub fn score(&self) -> u64 {
        self.metrics.score()
    }

    pub fn observe(&mut self, observation: PathObservation) {
        self.observe_probe_feedback(PathProbeFeedback {
            obs_rtt_ms: Some(observation.obs_rtt_ms),
            loss_ppm: observation.loss_ppm,
            jitter_ms: Some(observation.jitter_ms),
        });
    }

    pub fn observe_probe_feedback(&mut self, feedback: PathProbeFeedback) {
        if let Some(obs_rtt_ms) = feedback.obs_rtt_ms {
            self.metrics.obs_rtt_ms = ewma_u32(
                self.metrics.obs_rtt_ms,
                obs_rtt_ms,
                RTT_EWMA_ALPHA_NUMERATOR,
                RTT_EWMA_ALPHA_DENOMINATOR,
            );
        }
        self.metrics.loss_ppm = ewma_u32(
            self.metrics.loss_ppm,
            feedback.loss_ppm,
            LOSS_EWMA_ALPHA_NUMERATOR,
            LOSS_EWMA_ALPHA_DENOMINATOR,
        );
        if let Some(jitter_ms) = feedback.jitter_ms {
            self.metrics.jitter_ms = ewma_u32(
                self.metrics.jitter_ms,
                jitter_ms,
                JITTER_EWMA_ALPHA_NUMERATOR,
                JITTER_EWMA_ALPHA_DENOMINATOR,
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HysteresisConfig {
    pub min_absolute_improvement_ms: u64,
    pub min_relative_improvement_percent: u8,
    pub min_dwell_time_s: u64,
    pub max_switches_per_minute: usize,
}

impl Default for HysteresisConfig {
    fn default() -> Self {
        Self {
            min_absolute_improvement_ms: DEFAULT_MIN_ABSOLUTE_IMPROVEMENT_MS,
            min_relative_improvement_percent: DEFAULT_MIN_RELATIVE_IMPROVEMENT_PERCENT,
            min_dwell_time_s: DEFAULT_MIN_DWELL_TIME_S,
            max_switches_per_minute: DEFAULT_MAX_SWITCHES_PER_MINUTE,
        }
    }
}

impl HysteresisConfig {
    pub fn validate(self) -> Result<Self, RoutingError> {
        for (field, value) in [
            (
                "min_absolute_improvement_ms",
                self.min_absolute_improvement_ms,
            ),
            (
                "min_relative_improvement_percent",
                self.min_relative_improvement_percent as u64,
            ),
            ("min_dwell_time_s", self.min_dwell_time_s),
            (
                "max_switches_per_minute",
                self.max_switches_per_minute as u64,
            ),
        ] {
            if value == 0 {
                return Err(RoutingError::ZeroLimit { field });
            }
        }

        Ok(self)
    }
}

#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("routing config limit {field} must be non-zero")]
    ZeroLimit { field: &'static str },
    #[error(
        "path {path_id} already has max_in_flight_path_probes_per_path ({max_in_flight_path_probes_per_path}) probes in flight"
    )]
    TooManyInFlightPathProbes {
        path_id: u64,
        max_in_flight_path_probes_per_path: usize,
    },
    #[error("unknown in-flight path probe {probe_id} for path {path_id}")]
    UnknownPathProbe { path_id: u64, probe_id: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteDecision {
    NoCandidates,
    SelectedInitial {
        path_id: u64,
        score: u64,
    },
    Stayed {
        path_id: u64,
        score: u64,
    },
    Switched {
        from_path_id: u64,
        to_path_id: u64,
        from_score: u64,
        to_score: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSelector {
    config: HysteresisConfig,
    current_path_id: Option<u64>,
    current_since_unix_s: Option<u64>,
    switch_history_unix_s: VecDeque<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathProbeTracker {
    config: PathProbeConfig,
    next_probe_id: u64,
    paths: BTreeMap<u64, PathProbeState>,
}

impl PathProbeTracker {
    pub fn new(config: PathProbeConfig) -> Result<Self, RoutingError> {
        Ok(Self {
            config: config.validate()?,
            next_probe_id: 0,
            paths: BTreeMap::new(),
        })
    }

    pub const fn config(&self) -> PathProbeConfig {
        self.config
    }

    pub fn begin_probe(
        &mut self,
        path_id: u64,
        now_unix_ms: u64,
    ) -> Result<Option<PathProbe>, RoutingError> {
        let state = self.paths.entry(path_id).or_default();
        if let Some(last_probe_sent_at_unix_ms) = state.last_probe_sent_at_unix_ms {
            if now_unix_ms.saturating_sub(last_probe_sent_at_unix_ms)
                < self.config.path_probe_interval_ms
            {
                return Ok(None);
            }
        }

        if state.in_flight.len() == DEFAULT_MAX_IN_FLIGHT_PATH_PROBES_PER_PATH {
            return Err(RoutingError::TooManyInFlightPathProbes {
                path_id,
                max_in_flight_path_probes_per_path: DEFAULT_MAX_IN_FLIGHT_PATH_PROBES_PER_PATH,
            });
        }

        let probe_id = self.next_probe_id;
        self.next_probe_id = self.next_probe_id.wrapping_add(1);
        state.last_probe_sent_at_unix_ms = Some(now_unix_ms);
        state.in_flight.push_back(InFlightPathProbe {
            probe_id,
            sent_at_unix_ms: now_unix_ms,
        });
        Ok(Some(PathProbe {
            path_id,
            probe_id,
            sent_at_unix_ms: now_unix_ms,
        }))
    }

    pub fn complete_probe(
        &mut self,
        result: PathProbeResult,
        received_at_unix_ms: u64,
    ) -> Result<PathProbeFeedback, RoutingError> {
        let state = self
            .paths
            .get_mut(&result.path_id)
            .ok_or(RoutingError::UnknownPathProbe {
                path_id: result.path_id,
                probe_id: result.probe_id,
            })?;
        let probe = remove_in_flight_probe(state, result.probe_id).ok_or(
            RoutingError::UnknownPathProbe {
                path_id: result.path_id,
                probe_id: result.probe_id,
            },
        )?;
        let obs_rtt_ms =
            saturating_millis_to_u32(received_at_unix_ms.saturating_sub(probe.sent_at_unix_ms));
        let jitter_ms = state
            .last_success_rtt_ms
            .map(|last_success_rtt_ms| last_success_rtt_ms.abs_diff(obs_rtt_ms))
            .unwrap_or(0);
        state.last_success_rtt_ms = Some(obs_rtt_ms);
        note_probe_outcome(&mut state.recent_outcomes, true);

        Ok(PathProbeFeedback {
            obs_rtt_ms: Some(obs_rtt_ms),
            loss_ppm: loss_ppm(&state.recent_outcomes),
            jitter_ms: Some(jitter_ms),
        })
    }

    pub fn mark_probe_lost(
        &mut self,
        path_id: u64,
        probe_id: u64,
    ) -> Result<PathProbeFeedback, RoutingError> {
        let state = self
            .paths
            .get_mut(&path_id)
            .ok_or(RoutingError::UnknownPathProbe { path_id, probe_id })?;
        remove_in_flight_probe(state, probe_id)
            .ok_or(RoutingError::UnknownPathProbe { path_id, probe_id })?;
        note_probe_outcome(&mut state.recent_outcomes, false);

        Ok(PathProbeFeedback {
            obs_rtt_ms: None,
            loss_ppm: loss_ppm(&state.recent_outcomes),
            jitter_ms: None,
        })
    }

    pub fn in_flight_probe_count(&self, path_id: u64) -> usize {
        self.paths
            .get(&path_id)
            .map(|state| state.in_flight.len())
            .unwrap_or(0)
    }
}

impl RouteSelector {
    pub fn new(config: HysteresisConfig) -> Result<Self, RoutingError> {
        Ok(Self {
            config: config.validate()?,
            current_path_id: None,
            current_since_unix_s: None,
            switch_history_unix_s: VecDeque::new(),
        })
    }

    pub fn current_path_id(&self) -> Option<u64> {
        self.current_path_id
    }

    pub const fn config(&self) -> HysteresisConfig {
        self.config
    }

    pub fn evaluate(&mut self, now_unix_s: u64, candidates: &[PathState]) -> RouteDecision {
        let Some(best_candidate) = best_candidate(candidates) else {
            return RouteDecision::NoCandidates;
        };
        let best_score = best_candidate.score();

        let Some(current_path_id) = self.current_path_id else {
            self.current_path_id = Some(best_candidate.path_id);
            self.current_since_unix_s = Some(now_unix_s);
            return RouteDecision::SelectedInitial {
                path_id: best_candidate.path_id,
                score: best_score,
            };
        };

        let Some(current_candidate) = candidates
            .iter()
            .find(|path| path.path_id == current_path_id)
        else {
            self.current_path_id = Some(best_candidate.path_id);
            self.current_since_unix_s = Some(now_unix_s);
            self.note_switch(now_unix_s);
            return RouteDecision::Switched {
                from_path_id: current_path_id,
                to_path_id: best_candidate.path_id,
                from_score: best_score,
                to_score: best_score,
            };
        };

        let current_score = current_candidate.score();
        if best_candidate.path_id == current_candidate.path_id {
            return RouteDecision::Stayed {
                path_id: current_candidate.path_id,
                score: current_score,
            };
        }

        if !self.should_switch(now_unix_s, current_score, best_score) {
            return RouteDecision::Stayed {
                path_id: current_candidate.path_id,
                score: current_score,
            };
        }

        let from_path_id = current_candidate.path_id;
        let from_score = current_score;
        self.current_path_id = Some(best_candidate.path_id);
        self.current_since_unix_s = Some(now_unix_s);
        self.note_switch(now_unix_s);
        RouteDecision::Switched {
            from_path_id,
            to_path_id: best_candidate.path_id,
            from_score,
            to_score: best_score,
        }
    }

    fn should_switch(&mut self, now_unix_s: u64, current_score: u64, best_score: u64) -> bool {
        if best_score >= current_score {
            return false;
        }

        let absolute_improvement = current_score - best_score;
        if absolute_improvement < self.config.min_absolute_improvement_ms {
            return false;
        }

        let relative_improvement_percent = if current_score == 0 {
            100
        } else {
            ((absolute_improvement * 100) / current_score) as u8
        };
        if relative_improvement_percent < self.config.min_relative_improvement_percent {
            return false;
        }

        let Some(current_since_unix_s) = self.current_since_unix_s else {
            return true;
        };
        if now_unix_s.saturating_sub(current_since_unix_s) < self.config.min_dwell_time_s {
            return false;
        }

        self.prune_switch_history(now_unix_s);
        self.switch_history_unix_s.len() < self.config.max_switches_per_minute
    }

    fn note_switch(&mut self, now_unix_s: u64) {
        self.prune_switch_history(now_unix_s);
        self.switch_history_unix_s.push_back(now_unix_s);
    }

    fn prune_switch_history(&mut self, now_unix_s: u64) {
        while let Some(switch_unix_s) = self.switch_history_unix_s.front().copied() {
            if now_unix_s.saturating_sub(switch_unix_s) < 60 {
                break;
            }
            self.switch_history_unix_s.pop_front();
        }
    }
}

fn best_candidate(candidates: &[PathState]) -> Option<PathState> {
    candidates.iter().copied().min_by(|left, right| {
        left.score()
            .cmp(&right.score())
            .then_with(|| left.path_id.cmp(&right.path_id))
    })
}

fn ewma_u32(current: u32, sample: u32, alpha_numerator: u64, alpha_denominator: u64) -> u32 {
    let current = u64::from(current);
    let sample = u64::from(sample);
    let weighted = (current * (alpha_denominator - alpha_numerator)) + (sample * alpha_numerator);
    ((weighted + (alpha_denominator / 2)) / alpha_denominator) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InFlightPathProbe {
    probe_id: u64,
    sent_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PathProbeState {
    last_probe_sent_at_unix_ms: Option<u64>,
    in_flight: VecDeque<InFlightPathProbe>,
    recent_outcomes: VecDeque<bool>,
    last_success_rtt_ms: Option<u32>,
}

fn remove_in_flight_probe(state: &mut PathProbeState, probe_id: u64) -> Option<InFlightPathProbe> {
    let index = state
        .in_flight
        .iter()
        .position(|probe| probe.probe_id == probe_id)?;
    state.in_flight.remove(index)
}

fn note_probe_outcome(window: &mut VecDeque<bool>, success: bool) {
    if window.len() == DEFAULT_PATH_PROBE_LOSS_WINDOW_SAMPLES {
        window.pop_front();
    }
    window.push_back(success);
}

fn loss_ppm(window: &VecDeque<bool>) -> u32 {
    if window.is_empty() {
        return 0;
    }

    let losses = window.iter().filter(|success| !**success).count() as u64;
    let total = window.len() as u64;
    (((losses * 1_000_000) + (total / 2)) / total) as u32
}

fn saturating_millis_to_u32(value: u64) -> u32 {
    value.min(u64::from(u32::MAX)) as u32
}

fn canonical_message_bytes<T>(message: &T) -> Result<Vec<u8>, RoutingMessageError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(message)?;
    validate_message_body_len(bytes.len())?;
    Ok(bytes)
}

fn parse_message_bytes<T>(bytes: &[u8]) -> Result<T, RoutingMessageError>
where
    T: DeserializeOwned,
{
    validate_message_body_len(bytes.len())?;
    serde_json::from_slice(bytes).map_err(Into::into)
}

fn validate_message_body_len(body_len: usize) -> Result<(), RoutingMessageError> {
    let body_len = u32::try_from(body_len).unwrap_or(u32::MAX);
    if body_len > MAX_FRAME_BODY_LEN {
        return Err(FrameError::BodyTooLarge {
            body_len,
            max_body_len: MAX_FRAME_BODY_LEN,
        }
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde::Deserialize;

    use super::{
        HysteresisConfig, PathMetrics, PathObservation, PathProbe, PathProbeConfig,
        PathProbeFeedback, PathProbeResult, PathProbeTracker, PathState, RouteDecision,
        RouteSelector, RoutingError, DEFAULT_MAX_IN_FLIGHT_PATH_PROBES_PER_PATH,
        DEFAULT_PATH_PROBE_INTERVAL_MS,
    };
    use crate::wire::{Message, MessageType};

    #[test]
    fn path_score_matches_open_question_formula() {
        let metrics = PathMetrics {
            est_rtt_ms: 40,
            obs_rtt_ms: 50,
            jitter_ms: 7,
            loss_ppm: 12_000,
            relay_hops: 2,
            censorship_risk_level: 1,
            diversity_bonus: 3,
        };

        assert_eq!(
            metrics.score(),
            8 * 50 + 2 * 40 + 7 + 25 * 12 + 25 * 2 + 100 - 20 * 3
        );
    }

    #[test]
    fn path_observation_uses_integer_ewma_defaults() {
        let mut path = PathState {
            path_id: 1,
            metrics: PathMetrics {
                est_rtt_ms: 50,
                obs_rtt_ms: 100,
                jitter_ms: 10,
                loss_ppm: 1_000,
                relay_hops: 0,
                censorship_risk_level: 0,
                diversity_bonus: 0,
            },
        };

        path.observe(PathObservation {
            obs_rtt_ms: 50,
            loss_ppm: 0,
            jitter_ms: 20,
        });

        assert_eq!(path.metrics.obs_rtt_ms, 90);
        assert_eq!(path.metrics.loss_ppm, 900);
        assert_eq!(path.metrics.jitter_ms, 11);
    }

    #[test]
    fn path_probe_config_rejects_zero_interval() {
        let error = PathProbeConfig {
            path_probe_interval_ms: 0,
        }
        .validate()
        .expect_err("zero probe interval must be rejected");

        assert!(matches!(
            error,
            RoutingError::ZeroLimit {
                field: "path_probe_interval_ms"
            }
        ));
    }

    #[test]
    fn hysteresis_config_rejects_zero_limits() {
        let error = HysteresisConfig {
            min_absolute_improvement_ms: 0,
            ..HysteresisConfig::default()
        }
        .validate()
        .expect_err("zero hysteresis limits must be rejected");

        assert!(matches!(
            error,
            RoutingError::ZeroLimit {
                field: "min_absolute_improvement_ms"
            }
        ));
    }

    #[test]
    fn route_selector_requires_absolute_relative_and_dwell_thresholds() {
        let mut selector =
            RouteSelector::new(HysteresisConfig::default()).expect("config should be valid");
        let current = sample_path(1, 50, 100, 10);
        let slightly_better = sample_path(2, 50, 99, 10);
        let clearly_better = sample_path(2, 40, 80, 10);

        assert!(matches!(
            selector.evaluate(1_700_000_000, &[current]),
            RouteDecision::SelectedInitial { path_id: 1, .. }
        ));
        assert!(matches!(
            selector.evaluate(1_700_000_010, &[current, slightly_better]),
            RouteDecision::Stayed { path_id: 1, .. }
        ));
        assert!(matches!(
            selector.evaluate(1_700_000_020, &[current, clearly_better]),
            RouteDecision::Stayed { path_id: 1, .. }
        ));
        assert!(matches!(
            selector.evaluate(1_700_000_031, &[current, clearly_better]),
            RouteDecision::Switched {
                from_path_id: 1,
                to_path_id: 2,
                ..
            }
        ));
    }

    #[test]
    fn route_selector_caps_switch_rate_to_avoid_flapping() {
        let mut selector =
            RouteSelector::new(HysteresisConfig::default()).expect("config should be valid");
        let path_a = sample_path(1, 60, 100, 10);
        let path_b = sample_path(2, 30, 60, 10);
        let path_c = sample_path(3, 20, 40, 10);
        let path_d = sample_path(4, 10, 20, 10);

        let _ = selector.evaluate(1_700_000_000, &[path_a]);
        let _ = selector.evaluate(1_700_000_031, &[path_a, path_b]);
        let _ = selector.evaluate(1_700_000_062, &[path_b, path_c]);

        let decision = selector.evaluate(1_700_000_090, &[path_c, path_d]);
        assert!(matches!(decision, RouteDecision::Stayed { path_id: 3, .. }));
        assert_eq!(selector.current_path_id(), Some(3));
    }

    #[test]
    fn route_selector_keeps_current_path_under_small_jitter_improvements() {
        let mut selector =
            RouteSelector::new(HysteresisConfig::default()).expect("config should be valid");
        let stable = sample_path(1, 45, 90, 10);

        let _ = selector.evaluate(1_700_000_000, &[stable]);
        for (offset, candidate) in [
            sample_path(2, 45, 89, 10),
            sample_path(2, 44, 89, 11),
            sample_path(2, 45, 88, 12),
        ]
        .into_iter()
        .enumerate()
        {
            let decision = selector.evaluate(1_700_000_031 + offset as u64, &[stable, candidate]);
            assert!(matches!(decision, RouteDecision::Stayed { path_id: 1, .. }));
        }

        assert_eq!(selector.current_path_id(), Some(1));
    }

    #[test]
    fn path_probe_messages_expose_expected_wire_types_and_round_trip() {
        let probe = PathProbe {
            path_id: 7,
            probe_id: 99,
            sent_at_unix_ms: 1_700_000_000_123,
        };
        assert_eq!(PathProbe::TYPE, MessageType::PathProbe);
        assert_eq!(
            PathProbe::from_canonical_bytes(&probe.canonical_bytes().expect("probe should encode"))
                .expect("probe should decode"),
            probe
        );

        let result = PathProbeResult {
            path_id: probe.path_id,
            probe_id: probe.probe_id,
        };
        assert_eq!(PathProbeResult::TYPE, MessageType::PathProbeResult);
        assert_eq!(
            PathProbeResult::from_canonical_bytes(
                &result
                    .canonical_bytes()
                    .expect("probe result should encode")
            )
            .expect("probe result should decode"),
            result
        );
    }

    #[test]
    fn path_probe_message_vectors_match_fixture() {
        let fixture = read_path_probe_message_vector();
        let probe = PathProbe {
            path_id: fixture.path_id,
            probe_id: fixture.probe_id,
            sent_at_unix_ms: fixture.sent_at_unix_ms,
        };
        let result = PathProbeResult {
            path_id: fixture.path_id,
            probe_id: fixture.probe_id,
        };

        assert_eq!(
            encode_hex(&probe.canonical_bytes().expect("probe should encode")),
            fixture.path_probe_hex
        );
        assert_eq!(
            PathProbe::from_canonical_bytes(&decode_hex(&fixture.path_probe_hex))
                .expect("probe should decode"),
            probe
        );
        assert_eq!(
            encode_hex(
                &result
                    .canonical_bytes()
                    .expect("probe result should encode")
            ),
            fixture.path_probe_result_hex
        );
        assert_eq!(
            PathProbeResult::from_canonical_bytes(&decode_hex(&fixture.path_probe_result_hex))
                .expect("probe result should decode"),
            result
        );
    }

    #[test]
    fn path_probe_tracker_respects_interval_and_in_flight_limits() {
        let mut tracker = PathProbeTracker::new(PathProbeConfig {
            path_probe_interval_ms: 1,
        })
        .expect("config should be valid");

        let first = tracker
            .begin_probe(7, 100)
            .expect("first probe should succeed")
            .expect("first probe should be due");
        assert_eq!(tracker.in_flight_probe_count(7), 1);
        assert_eq!(
            tracker
                .begin_probe(7, 100)
                .expect("same-tick scheduling should not error"),
            None
        );

        let _ = tracker
            .begin_probe(7, 101)
            .expect("second probe should succeed")
            .expect("second probe should be due");
        let _ = tracker
            .begin_probe(7, 102)
            .expect("third probe should succeed")
            .expect("third probe should be due");
        let _ = tracker
            .begin_probe(7, 103)
            .expect("fourth probe should succeed")
            .expect("fourth probe should be due");

        let error = tracker
            .begin_probe(7, 104)
            .expect_err("fifth probe should exceed the in-flight cap");
        assert!(matches!(
            error,
            RoutingError::TooManyInFlightPathProbes {
                path_id: 7,
                max_in_flight_path_probes_per_path: DEFAULT_MAX_IN_FLIGHT_PATH_PROBES_PER_PATH,
            }
        ));

        assert_eq!(first.probe_id, 0);
    }

    #[test]
    fn path_probe_tracker_builds_feedback_from_success_and_loss() {
        let mut tracker = PathProbeTracker::new(PathProbeConfig {
            path_probe_interval_ms: DEFAULT_PATH_PROBE_INTERVAL_MS,
        })
        .expect("config should be valid");
        let first = tracker
            .begin_probe(9, 1_000)
            .expect("first probe should succeed")
            .expect("first probe should be due");
        let first_feedback = tracker
            .complete_probe(
                PathProbeResult {
                    path_id: first.path_id,
                    probe_id: first.probe_id,
                },
                1_080,
            )
            .expect("probe completion should succeed");
        assert_eq!(
            first_feedback,
            PathProbeFeedback {
                obs_rtt_ms: Some(80),
                loss_ppm: 0,
                jitter_ms: Some(0),
            }
        );

        let second = tracker
            .begin_probe(9, 6_000)
            .expect("second probe should succeed")
            .expect("second probe should be due");
        let lost_feedback = tracker
            .mark_probe_lost(second.path_id, second.probe_id)
            .expect("probe loss should succeed");
        assert_eq!(
            lost_feedback,
            PathProbeFeedback {
                obs_rtt_ms: None,
                loss_ppm: 500_000,
                jitter_ms: None,
            }
        );
    }

    #[test]
    fn path_probe_feedback_updates_loss_without_overwriting_last_rtt() {
        let mut path = PathState {
            path_id: 1,
            metrics: PathMetrics {
                est_rtt_ms: 50,
                obs_rtt_ms: 100,
                jitter_ms: 10,
                loss_ppm: 0,
                relay_hops: 0,
                censorship_risk_level: 0,
                diversity_bonus: 0,
            },
        };

        path.observe_probe_feedback(PathProbeFeedback {
            obs_rtt_ms: Some(80),
            loss_ppm: 0,
            jitter_ms: Some(20),
        });
        path.observe_probe_feedback(PathProbeFeedback {
            obs_rtt_ms: None,
            loss_ppm: 500_000,
            jitter_ms: None,
        });

        assert_eq!(path.metrics.obs_rtt_ms, 96);
        assert_eq!(path.metrics.jitter_ms, 11);
        assert_eq!(path.metrics.loss_ppm, 50_000);
    }

    fn sample_path(path_id: u64, est_rtt_ms: u32, obs_rtt_ms: u32, jitter_ms: u32) -> PathState {
        PathState {
            path_id,
            metrics: PathMetrics {
                est_rtt_ms,
                obs_rtt_ms,
                jitter_ms,
                loss_ppm: 0,
                relay_hops: 0,
                censorship_risk_level: 0,
                diversity_bonus: 0,
            },
        }
    }

    #[derive(Debug, Deserialize)]
    struct PathProbeMessageVector {
        path_id: u64,
        probe_id: u64,
        sent_at_unix_ms: u64,
        path_probe_hex: String,
        path_probe_result_hex: String,
    }

    fn path_probe_message_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("path_probe_messages.json")
    }

    fn read_path_probe_message_vector() -> PathProbeMessageVector {
        let bytes = fs::read(path_probe_message_vector_path())
            .expect("path probe message vector file should exist");
        serde_json::from_slice(&bytes).expect("path probe message vector file should parse")
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        assert_eq!(value.len() % 2, 0, "hex input should have even length");

        value
            .as_bytes()
            .chunks_exact(2)
            .map(|chunk| {
                let hex = std::str::from_utf8(chunk).expect("hex bytes should be utf-8");
                u8::from_str_radix(hex, 16).expect("hex digits should parse")
            })
            .collect()
    }

    fn encode_hex(bytes: &[u8]) -> String {
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}").expect("hex encoding should succeed");
        }

        encoded
    }
}
