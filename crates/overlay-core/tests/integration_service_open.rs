use overlay_core::REPOSITORY_STAGE;

#[test]
fn service_open_smoke_tracks_current_stage_boundary() {
    assert_eq!(REPOSITORY_STAGE, "milestone-5-presence-lookup");
}
