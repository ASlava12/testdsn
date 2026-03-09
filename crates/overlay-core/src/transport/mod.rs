use serde::{Deserialize, Serialize};
use std::fmt;

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

pub trait Transport {
    fn transport_class(&self) -> TransportClass;

    fn adapter_name(&self) -> &'static str;

    fn is_placeholder(&self) -> bool {
        true
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

#[cfg(test)]
mod tests {
    use super::{
        QuicTransport, RelayTransport, TcpTransport, Transport, TransportClass, WsTunnelTransport,
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
}
