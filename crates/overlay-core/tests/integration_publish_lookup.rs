use overlay_core::REPOSITORY_STAGE;

#[test]
fn publish_lookup_smoke_tracks_current_stage_boundary() {
    assert_eq!(REPOSITORY_STAGE, "milestone-3-session-skeleton");
}
