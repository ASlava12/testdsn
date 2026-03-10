//! Path metrics, deterministic scoring, and switch hysteresis baseline for Milestone 7.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
        self.metrics.obs_rtt_ms = ewma_u32(
            self.metrics.obs_rtt_ms,
            observation.obs_rtt_ms,
            RTT_EWMA_ALPHA_NUMERATOR,
            RTT_EWMA_ALPHA_DENOMINATOR,
        );
        self.metrics.loss_ppm = ewma_u32(
            self.metrics.loss_ppm,
            observation.loss_ppm,
            LOSS_EWMA_ALPHA_NUMERATOR,
            LOSS_EWMA_ALPHA_DENOMINATOR,
        );
        self.metrics.jitter_ms = ewma_u32(
            self.metrics.jitter_ms,
            observation.jitter_ms,
            JITTER_EWMA_ALPHA_NUMERATOR,
            JITTER_EWMA_ALPHA_DENOMINATOR,
        );
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

#[cfg(test)]
mod tests {
    use super::{
        HysteresisConfig, PathMetrics, PathObservation, PathState, RouteDecision, RouteSelector,
        RoutingError,
    };

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
}
