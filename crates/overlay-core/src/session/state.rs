#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Opening,
    Established,
    Degraded,
    Closing,
    Closed,
}
