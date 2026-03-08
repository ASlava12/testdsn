use overlay_core::REPOSITORY_STAGE;

#[test]
fn relay_fallback_smoke_tracks_current_stage_boundary() {
    assert_eq!(REPOSITORY_STAGE, "milestone-2-handshake");
}
