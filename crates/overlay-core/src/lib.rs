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

pub const REPOSITORY_STAGE: &str = "milestone-19-pilot-closure";

#[cfg(test)]
mod tests {
    use super::REPOSITORY_STAGE;

    #[test]
    fn reports_repository_stage() {
        assert_eq!(REPOSITORY_STAGE, "milestone-19-pilot-closure");
    }
}
