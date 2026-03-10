use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub const DEFAULT_MAX_TRANSPORT_BUFFER_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportClass {
    Tcp,
    Quic,
    Ws,
    Relay,
}

impl TransportClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Quic => "quic",
            Self::Ws => "ws",
            Self::Relay => "relay",
        }
    }
}

impl fmt::Display for TransportClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportPollEvent {
    Opened,
    FrameReceived { bytes: Vec<u8> },
    Closed,
    Failed { detail: String },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TransportRunnerError {
    #[error(
        "transport adapter '{adapter_name}' does not implement runner operation '{operation}'"
    )]
    UnsupportedOperation {
        adapter_name: &'static str,
        operation: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportBufferConfig {
    pub max_buffer_bytes: usize,
}

impl Default for TransportBufferConfig {
    fn default() -> Self {
        Self {
            max_buffer_bytes: DEFAULT_MAX_TRANSPORT_BUFFER_BYTES,
        }
    }
}

impl TransportBufferConfig {
    pub fn validate(self) -> Result<Self, TransportBufferError> {
        if self.max_buffer_bytes == 0 {
            return Err(TransportBufferError::ZeroLimit {
                field: "max_buffer_bytes",
            });
        }

        Ok(self)
    }

    pub fn validate_poll_event(
        self,
        event: &TransportPollEvent,
    ) -> Result<(), TransportBufferError> {
        self.validate()?;

        if let TransportPollEvent::FrameReceived { bytes } = event {
            let byte_len = bytes.len();
            if byte_len > self.max_buffer_bytes {
                return Err(TransportBufferError::FrameExceedsBuffer {
                    byte_len,
                    max_buffer_bytes: self.max_buffer_bytes,
                });
            }
        }

        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TransportBufferError {
    #[error("transport buffer limit {field} must be non-zero")]
    ZeroLimit { field: &'static str },
    #[error(
        "transport frame length {byte_len} exceeds max_transport_buffer_bytes {max_buffer_bytes}"
    )]
    FrameExceedsBuffer {
        byte_len: usize,
        max_buffer_bytes: usize,
    },
}

pub trait Transport {
    fn transport_class(&self) -> TransportClass;

    fn adapter_name(&self) -> &'static str;

    fn is_placeholder(&self) -> bool {
        true
    }
}

/// Narrow Milestone 3 boundary for a future session runner:
/// open a transport, send a frame, start close, abort, and poll transport events.
pub trait TransportRunner: Transport {
    fn begin_open(&mut self, _correlation_id: u64) -> Result<(), TransportRunnerError> {
        Err(TransportRunnerError::UnsupportedOperation {
            adapter_name: self.adapter_name(),
            operation: "begin_open",
        })
    }

    fn send_frame(
        &mut self,
        _correlation_id: u64,
        _frame: &[u8],
    ) -> Result<(), TransportRunnerError> {
        Err(TransportRunnerError::UnsupportedOperation {
            adapter_name: self.adapter_name(),
            operation: "send_frame",
        })
    }

    fn begin_close(&mut self, _correlation_id: u64) -> Result<(), TransportRunnerError> {
        Err(TransportRunnerError::UnsupportedOperation {
            adapter_name: self.adapter_name(),
            operation: "begin_close",
        })
    }

    fn abort(&mut self, _correlation_id: u64) -> Result<(), TransportRunnerError> {
        Err(TransportRunnerError::UnsupportedOperation {
            adapter_name: self.adapter_name(),
            operation: "abort",
        })
    }

    fn poll_event(
        &mut self,
        _now_unix_ms: u64,
    ) -> Result<Option<TransportPollEvent>, TransportRunnerError> {
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TcpTransport;

impl Transport for TcpTransport {
    fn transport_class(&self) -> TransportClass {
        TransportClass::Tcp
    }

    fn adapter_name(&self) -> &'static str {
        "tcp"
    }
}

impl TransportRunner for TcpTransport {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QuicTransport;

impl Transport for QuicTransport {
    fn transport_class(&self) -> TransportClass {
        TransportClass::Quic
    }

    fn adapter_name(&self) -> &'static str {
        "quic"
    }
}

impl TransportRunner for QuicTransport {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WsTunnelTransport;

impl Transport for WsTunnelTransport {
    fn transport_class(&self) -> TransportClass {
        TransportClass::Ws
    }

    fn adapter_name(&self) -> &'static str {
        "ws-tunnel"
    }
}

impl TransportRunner for WsTunnelTransport {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RelayTransport;

impl Transport for RelayTransport {
    fn transport_class(&self) -> TransportClass {
        TransportClass::Relay
    }

    fn adapter_name(&self) -> &'static str {
        "relay"
    }
}

impl TransportRunner for RelayTransport {}

#[cfg(test)]
mod tests {
    use super::{
        QuicTransport, RelayTransport, TcpTransport, Transport, TransportBufferConfig,
        TransportBufferError, TransportClass, TransportPollEvent, TransportRunner,
        TransportRunnerError, WsTunnelTransport,
    };

    #[test]
    fn placeholder_adapters_report_expected_transport_classes() {
        let transports: [(&dyn Transport, TransportClass, &str); 4] = [
            (&TcpTransport, TransportClass::Tcp, "tcp"),
            (&QuicTransport, TransportClass::Quic, "quic"),
            (&WsTunnelTransport, TransportClass::Ws, "ws-tunnel"),
            (&RelayTransport, TransportClass::Relay, "relay"),
        ];

        for (transport, expected_class, expected_name) in transports {
            assert_eq!(transport.transport_class(), expected_class);
            assert_eq!(transport.adapter_name(), expected_name);
            assert!(transport.is_placeholder());
        }
    }

    #[test]
    fn placeholder_runners_expose_minimal_runner_boundary() {
        let mut tcp = TcpTransport;
        let mut quic = QuicTransport;
        let mut ws = WsTunnelTransport;
        let mut relay = RelayTransport;

        for transport in [
            &mut tcp as &mut dyn TransportRunner,
            &mut quic as &mut dyn TransportRunner,
            &mut ws as &mut dyn TransportRunner,
            &mut relay as &mut dyn TransportRunner,
        ] {
            assert!(matches!(
                transport.begin_open(7),
                Err(TransportRunnerError::UnsupportedOperation { .. })
            ));
            assert!(matches!(
                transport.send_frame(7, b"ping"),
                Err(TransportRunnerError::UnsupportedOperation { .. })
            ));
            assert!(matches!(
                transport.begin_close(7),
                Err(TransportRunnerError::UnsupportedOperation { .. })
            ));
            assert!(matches!(
                transport.abort(7),
                Err(TransportRunnerError::UnsupportedOperation { .. })
            ));
            assert_eq!(
                transport
                    .poll_event(100)
                    .expect("placeholder poll should succeed"),
                None
            );
        }
    }

    #[test]
    fn transport_buffer_config_rejects_zero_limit() {
        let error = TransportBufferConfig {
            max_buffer_bytes: 0,
        }
        .validate()
        .expect_err("zero transport buffer must be rejected");

        assert_eq!(
            error,
            TransportBufferError::ZeroLimit {
                field: "max_buffer_bytes",
            }
        );
    }

    #[test]
    fn transport_buffer_config_accepts_non_frame_events_and_bounded_frames() {
        let config = TransportBufferConfig {
            max_buffer_bytes: 8,
        }
        .validate()
        .expect("config should be valid");

        assert_eq!(
            config.validate_poll_event(&TransportPollEvent::Opened),
            Ok(())
        );
        assert_eq!(
            config.validate_poll_event(&TransportPollEvent::FrameReceived {
                bytes: vec![1_u8; 8],
            }),
            Ok(())
        );
    }

    #[test]
    fn transport_buffer_config_rejects_oversized_received_frames() {
        let config = TransportBufferConfig {
            max_buffer_bytes: 8,
        }
        .validate()
        .expect("config should be valid");

        let error = config
            .validate_poll_event(&TransportPollEvent::FrameReceived {
                bytes: vec![1_u8; 9],
            })
            .expect_err("oversized received frames must be rejected");

        assert_eq!(
            error,
            TransportBufferError::FrameExceedsBuffer {
                byte_len: 9,
                max_buffer_bytes: 8,
            }
        );
    }
}
