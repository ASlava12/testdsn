pub mod bootstrap;
pub mod config;
pub mod crypto;
pub mod error;
pub mod identity;
pub mod metrics;
pub mod peer;
pub mod records;
pub mod relay;
pub mod rendezvous;
pub mod routing;
pub mod runtime;
pub mod service;
pub mod session;
pub mod transport;
pub mod wire;

pub const REPOSITORY_STAGE: &str = "milestone-28-production-gates-packaging-safety-hardening";

#[cfg(test)]
mod tests {
    use super::REPOSITORY_STAGE;

    const REPOSITORY_STAGE_FILE: &str = include_str!("../../../REPOSITORY_STAGE");

    #[test]
    fn reports_repository_stage() {
        assert_eq!(REPOSITORY_STAGE, REPOSITORY_STAGE_FILE.trim());
    }

    #[test]
    fn repository_stage_marker_file_matches_constant() {
        assert_eq!(REPOSITORY_STAGE_FILE.trim(), REPOSITORY_STAGE);
    }
}
