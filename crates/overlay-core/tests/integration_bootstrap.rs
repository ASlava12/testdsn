use overlay_core::REPOSITORY_STAGE;

#[test]
fn bootstrap_smoke_tracks_current_stage_boundary() {
    assert_eq!(REPOSITORY_STAGE, "milestone-2-handshake");
}
