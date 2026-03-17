use overlay_core::{
    routing::{HysteresisConfig, PathMetrics, PathState, RouteDecision, RouteSelector},
    REPOSITORY_STAGE,
};

const REPOSITORY_STAGE_FILE: &str = include_str!("../../../REPOSITORY_STAGE");

#[test]
fn routing_selector_tracks_current_stage_boundary() {
    let mut selector =
        RouteSelector::new(HysteresisConfig::default()).expect("config should be valid");
    let direct_path = sample_path(1, 50, 100, 10, 0);
    let better_path = sample_path(2, 30, 60, 8, 1);

    assert_eq!(REPOSITORY_STAGE, REPOSITORY_STAGE_FILE.trim());
    assert!(matches!(
        selector.evaluate(1_700_000_000, &[direct_path]),
        RouteDecision::SelectedInitial { path_id: 1, .. }
    ));
    assert!(matches!(
        selector.evaluate(1_700_000_031, &[direct_path, better_path]),
        RouteDecision::Switched {
            from_path_id: 1,
            to_path_id: 2,
            ..
        }
    ));
    assert_eq!(selector.current_path_id(), Some(2));
}

fn sample_path(
    path_id: u64,
    est_rtt_ms: u32,
    obs_rtt_ms: u32,
    jitter_ms: u32,
    diversity_bonus: u8,
) -> PathState {
    PathState {
        path_id,
        metrics: PathMetrics {
            est_rtt_ms,
            obs_rtt_ms,
            jitter_ms,
            loss_ppm: 0,
            relay_hops: 0,
            censorship_risk_level: 0,
            diversity_bonus,
        },
    }
}
