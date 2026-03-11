use std::{
    fmt, io,
    io::{Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::wire::{FrameHeader, FRAME_HEADER_LEN};

pub const DEFAULT_MAX_TRANSPORT_BUFFER_BYTES: usize = 65_536;
const DEFAULT_TCP_CONNECT_TIMEOUT_MS: u64 = 250;

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

#[derive(Debug)]
pub struct TcpSocketTransport {
    stream: TcpStream,
    buffer_config: TransportBufferConfig,
    pending_events: std::collections::VecDeque<TransportPollEvent>,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    write_offset: usize,
    write_closed: bool,
    local_addr: Option<SocketAddr>,
    peer_addr: Option<SocketAddr>,
}

#[derive(Debug)]
pub struct TcpListenerHandle {
    listener: TcpListener,
    local_addr: SocketAddr,
    buffer_config: TransportBufferConfig,
}

#[derive(Debug, Error)]
pub enum TcpTransportIoError {
    #[error("tcp listener address must not be blank")]
    BlankListenerAddress,
    #[error("tcp dial hint must use the form tcp://host:port, got '{hint}'")]
    InvalidDialHint { hint: String },
    #[error("tcp endpoint '{endpoint}' did not resolve to a socket address")]
    UnresolvedEndpoint { endpoint: String },
    #[error("failed to bind tcp listener {addr}: {source}")]
    Bind { addr: String, source: io::Error },
    #[error("failed to accept tcp connection on {addr}: {source}")]
    Accept { addr: String, source: io::Error },
    #[error("failed to connect tcp socket to {endpoint}: {source}")]
    Connect { endpoint: String, source: io::Error },
    #[error("failed to configure tcp socket for {operation}: {source}")]
    Configure {
        operation: &'static str,
        source: io::Error,
    },
    #[error(transparent)]
    Buffer(#[from] TransportBufferError),
}

impl TcpListenerHandle {
    pub fn bind(
        bind_addr: &str,
        buffer_config: TransportBufferConfig,
    ) -> Result<Self, TcpTransportIoError> {
        let bind_addr = bind_addr.trim();
        if bind_addr.is_empty() {
            return Err(TcpTransportIoError::BlankListenerAddress);
        }
        let buffer_config = buffer_config.validate()?;
        let listener =
            TcpListener::bind(bind_addr).map_err(|source| TcpTransportIoError::Bind {
                addr: bind_addr.to_string(),
                source,
            })?;
        listener
            .set_nonblocking(true)
            .map_err(|source| TcpTransportIoError::Configure {
                operation: "nonblocking listener mode",
                source,
            })?;
        let local_addr =
            listener
                .local_addr()
                .map_err(|source| TcpTransportIoError::Configure {
                    operation: "listener local_addr",
                    source,
                })?;

        Ok(Self {
            listener,
            local_addr,
            buffer_config,
        })
    }

    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn accept(&self) -> Result<Option<TcpSocketTransport>, TcpTransportIoError> {
        match self.listener.accept() {
            Ok((stream, _peer_addr)) => Ok(Some(TcpSocketTransport::from_stream(
                stream,
                self.buffer_config,
            )?)),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(source) => Err(TcpTransportIoError::Accept {
                addr: self.local_addr.to_string(),
                source,
            }),
        }
    }
}

impl TcpSocketTransport {
    pub fn connect(
        dial_hint: &str,
        buffer_config: TransportBufferConfig,
    ) -> Result<Self, TcpTransportIoError> {
        let endpoint = parse_tcp_dial_hint(dial_hint)?;
        let socket_addr = endpoint
            .to_socket_addrs()
            .map_err(|source| TcpTransportIoError::Connect {
                endpoint: endpoint.clone(),
                source,
            })?
            .next()
            .ok_or_else(|| TcpTransportIoError::UnresolvedEndpoint {
                endpoint: endpoint.clone(),
            })?;
        let stream = TcpStream::connect_timeout(
            &socket_addr,
            Duration::from_millis(DEFAULT_TCP_CONNECT_TIMEOUT_MS),
        )
        .map_err(|source| TcpTransportIoError::Connect {
            endpoint: endpoint.clone(),
            source,
        })?;

        Self::from_stream(stream, buffer_config)
    }

    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr
    }

    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer_addr
    }

    fn from_stream(
        stream: TcpStream,
        buffer_config: TransportBufferConfig,
    ) -> Result<Self, TcpTransportIoError> {
        let buffer_config = buffer_config.validate()?;
        stream
            .set_nonblocking(true)
            .map_err(|source| TcpTransportIoError::Configure {
                operation: "nonblocking stream mode",
                source,
            })?;
        stream
            .set_nodelay(true)
            .map_err(|source| TcpTransportIoError::Configure {
                operation: "tcp nodelay",
                source,
            })?;

        let local_addr = stream.local_addr().ok();
        let peer_addr = stream.peer_addr().ok();
        let mut pending_events = std::collections::VecDeque::new();
        pending_events.push_back(TransportPollEvent::Opened);

        Ok(Self {
            stream,
            buffer_config,
            pending_events,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
            write_offset: 0,
            write_closed: false,
            local_addr,
            peer_addr,
        })
    }

    fn max_buffer_bytes(&self) -> usize {
        self.buffer_config.max_buffer_bytes
    }

    fn queue_failure(&mut self, detail: impl Into<String>) {
        self.pending_events.push_back(TransportPollEvent::Failed {
            detail: detail.into(),
        });
    }

    fn flush_writes(&mut self) {
        while self.write_offset < self.write_buffer.len() {
            match self.stream.write(&self.write_buffer[self.write_offset..]) {
                Ok(0) => {
                    self.queue_failure("tcp socket closed while writing frame");
                    return;
                }
                Ok(written) => {
                    self.write_offset = self.write_offset.saturating_add(written);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
                Err(error) => {
                    self.queue_failure(format!("tcp write failed: {error}"));
                    return;
                }
            }
        }

        if self.write_offset == self.write_buffer.len() {
            self.write_buffer.clear();
            self.write_offset = 0;
        }
    }

    fn next_frame(&mut self) -> Option<Vec<u8>> {
        if self.read_buffer.len() < FRAME_HEADER_LEN {
            return None;
        }

        let header = match FrameHeader::from_slice(&self.read_buffer[..FRAME_HEADER_LEN]) {
            Ok(header) => header,
            Err(error) => {
                self.read_buffer.clear();
                self.queue_failure(format!("invalid received frame header: {error}"));
                return None;
            }
        };
        let frame_len = FRAME_HEADER_LEN.saturating_add(header.body_len as usize);
        if frame_len > self.max_buffer_bytes() {
            self.read_buffer.clear();
            self.queue_failure(format!(
                "received frame length {frame_len} exceeds max_transport_buffer_bytes {}",
                self.max_buffer_bytes()
            ));
            return None;
        }
        if self.read_buffer.len() < frame_len {
            return None;
        }

        Some(self.read_buffer.drain(..frame_len).collect())
    }

    fn fill_read_buffer(&mut self) {
        let mut chunk = [0_u8; 8 * 1024];

        loop {
            match self.stream.read(&mut chunk) {
                Ok(0) => {
                    self.pending_events.push_back(TransportPollEvent::Closed);
                    return;
                }
                Ok(read_len) => {
                    if self.read_buffer.len().saturating_add(read_len) > self.max_buffer_bytes() {
                        self.read_buffer.clear();
                        self.queue_failure(format!(
                            "received frame length exceeds max_transport_buffer_bytes {}",
                            self.max_buffer_bytes()
                        ));
                        return;
                    }
                    self.read_buffer.extend_from_slice(&chunk[..read_len]);
                    if self.read_buffer.len() >= FRAME_HEADER_LEN {
                        match FrameHeader::from_slice(&self.read_buffer[..FRAME_HEADER_LEN]) {
                            Ok(header)
                                if self.read_buffer.len()
                                    >= FRAME_HEADER_LEN + header.body_len as usize =>
                            {
                                return;
                            }
                            Ok(_) => {}
                            Err(error) => {
                                self.read_buffer.clear();
                                self.queue_failure(format!(
                                    "invalid received frame header: {error}"
                                ));
                                return;
                            }
                        }
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return,
                Err(error) => {
                    self.queue_failure(format!("tcp read failed: {error}"));
                    return;
                }
            }
        }
    }
}

impl Transport for TcpSocketTransport {
    fn transport_class(&self) -> TransportClass {
        TransportClass::Tcp
    }

    fn adapter_name(&self) -> &'static str {
        "tcp"
    }

    fn is_placeholder(&self) -> bool {
        false
    }
}

impl TransportRunner for TcpSocketTransport {
    fn begin_open(&mut self, _correlation_id: u64) -> Result<(), TransportRunnerError> {
        Ok(())
    }

    fn send_frame(
        &mut self,
        _correlation_id: u64,
        frame: &[u8],
    ) -> Result<(), TransportRunnerError> {
        if self.write_closed {
            self.queue_failure("tcp transport cannot send after begin_close");
            return Ok(());
        }

        let frame_len = frame.len();
        if frame_len > self.max_buffer_bytes() {
            self.queue_failure(format!(
                "outbound frame length {frame_len} exceeds max_transport_buffer_bytes {}",
                self.max_buffer_bytes()
            ));
            return Ok(());
        }
        let pending_len = self.write_buffer.len().saturating_sub(self.write_offset);
        if pending_len.saturating_add(frame_len) > self.max_buffer_bytes() {
            self.queue_failure(format!(
                "outbound buffered bytes exceed max_transport_buffer_bytes {}",
                self.max_buffer_bytes()
            ));
            return Ok(());
        }

        if self.write_offset == self.write_buffer.len() {
            self.write_buffer.clear();
            self.write_offset = 0;
        }
        self.write_buffer.extend_from_slice(frame);
        self.flush_writes();
        Ok(())
    }

    fn begin_close(&mut self, _correlation_id: u64) -> Result<(), TransportRunnerError> {
        self.write_closed = true;
        if let Err(error) = self.stream.shutdown(Shutdown::Write) {
            self.queue_failure(format!("tcp shutdown(write) failed: {error}"));
        }
        Ok(())
    }

    fn abort(&mut self, _correlation_id: u64) -> Result<(), TransportRunnerError> {
        if let Err(error) = self.stream.shutdown(Shutdown::Both) {
            self.queue_failure(format!("tcp shutdown(both) failed: {error}"));
        } else {
            self.pending_events.push_back(TransportPollEvent::Closed);
        }
        Ok(())
    }

    fn poll_event(
        &mut self,
        _now_unix_ms: u64,
    ) -> Result<Option<TransportPollEvent>, TransportRunnerError> {
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(Some(event));
        }

        self.flush_writes();
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(Some(event));
        }

        if let Some(frame) = self.next_frame() {
            return Ok(Some(TransportPollEvent::FrameReceived { bytes: frame }));
        }

        self.fill_read_buffer();
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(Some(event));
        }

        Ok(self
            .next_frame()
            .map(|bytes| TransportPollEvent::FrameReceived { bytes }))
    }
}

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

fn parse_tcp_dial_hint(dial_hint: &str) -> Result<String, TcpTransportIoError> {
    let Some(endpoint) = dial_hint.trim().strip_prefix("tcp://") else {
        return Err(TcpTransportIoError::InvalidDialHint {
            hint: dial_hint.trim().to_string(),
        });
    };
    if endpoint.trim().is_empty() {
        return Err(TcpTransportIoError::InvalidDialHint {
            hint: dial_hint.trim().to_string(),
        });
    }

    Ok(endpoint.trim().to_string())
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::{
        QuicTransport, RelayTransport, TcpListenerHandle, TcpSocketTransport, TcpTransport,
        TcpTransportIoError, Transport, TransportBufferConfig, TransportBufferError,
        TransportClass, TransportPollEvent, TransportRunner, TransportRunnerError,
        WsTunnelTransport,
    };
    use crate::wire::{encode_framed_message, Ping};

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

    #[test]
    fn tcp_listener_accepts_connections_and_exchanges_framed_bytes() {
        let buffer_config = TransportBufferConfig {
            max_buffer_bytes: 256,
        }
        .validate()
        .expect("buffer config should be valid");
        let listener =
            TcpListenerHandle::bind("127.0.0.1:0", buffer_config).expect("listener should bind");
        let dial_hint = format!("tcp://{}", listener.local_addr());
        let mut client =
            TcpSocketTransport::connect(&dial_hint, buffer_config).expect("client should connect");
        let mut server = wait_for_accept(&listener);

        client.begin_open(1).expect("client open should succeed");
        server.begin_open(2).expect("server open should succeed");
        assert_eq!(poll_event(&mut client), TransportPollEvent::Opened);
        assert_eq!(poll_event(&mut server), TransportPollEvent::Opened);

        let ping = encode_framed_message(&Ping, 11).expect("ping should encode");
        client
            .send_frame(11, &ping)
            .expect("client send should succeed");
        assert_eq!(
            poll_event(&mut server),
            TransportPollEvent::FrameReceived {
                bytes: ping.clone()
            }
        );

        server
            .send_frame(11, &ping)
            .expect("server send should succeed");
        assert_eq!(
            poll_event(&mut client),
            TransportPollEvent::FrameReceived { bytes: ping }
        );
    }

    #[test]
    fn tcp_transport_rejects_invalid_dial_hints() {
        let error =
            TcpSocketTransport::connect("http://127.0.0.1:4101", TransportBufferConfig::default())
                .expect_err("non-tcp dial hints must be rejected");

        assert!(matches!(error, TcpTransportIoError::InvalidDialHint { .. }));
    }

    fn wait_for_accept(listener: &TcpListenerHandle) -> TcpSocketTransport {
        for _ in 0..200 {
            match listener.accept().expect("accept should not error") {
                Some(transport) => return transport,
                None => thread::sleep(Duration::from_millis(5)),
            }
        }

        panic!("listener did not accept a connection");
    }

    fn poll_event(transport: &mut TcpSocketTransport) -> TransportPollEvent {
        for attempt in 0..200 {
            if let Some(event) = transport
                .poll_event(attempt)
                .expect("tcp transport poll should succeed")
            {
                return event;
            }
            thread::sleep(Duration::from_millis(5));
        }

        panic!("transport did not produce an event");
    }
}
