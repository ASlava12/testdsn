use serde::{Deserialize, Serialize};

use crate::error::FrameError;

pub const FRAME_HEADER_LEN: usize = 17;
pub const MAX_FRAME_SIZE: usize = 64 * 1024;
pub const MAX_FRAME_BODY_LEN: u32 = (MAX_FRAME_SIZE - FRAME_HEADER_LEN) as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeader {
    pub version: u8,
    pub msg_type: u16,
    pub flags: u16,
    pub body_len: u32,
    pub correlation_id: u64,
}

impl FrameHeader {
    pub fn new(
        version: u8,
        msg_type: MessageType,
        flags: u16,
        body_len: u32,
        correlation_id: u64,
    ) -> Result<Self, FrameError> {
        let header = Self {
            version,
            msg_type: msg_type as u16,
            flags,
            body_len,
            correlation_id,
        };
        header.validate()?;
        Ok(header)
    }

    pub fn validate(&self) -> Result<(), FrameError> {
        if self.body_len > MAX_FRAME_BODY_LEN {
            return Err(FrameError::BodyTooLarge {
                body_len: self.body_len,
                max_body_len: MAX_FRAME_BODY_LEN,
            });
        }

        Ok(())
    }

    pub fn encode(self) -> Result<[u8; FRAME_HEADER_LEN], FrameError> {
        self.validate()?;

        let mut bytes = [0_u8; FRAME_HEADER_LEN];
        bytes[0] = self.version;
        bytes[1..3].copy_from_slice(&self.msg_type.to_be_bytes());
        bytes[3..5].copy_from_slice(&self.flags.to_be_bytes());
        bytes[5..9].copy_from_slice(&self.body_len.to_be_bytes());
        bytes[9..17].copy_from_slice(&self.correlation_id.to_be_bytes());
        Ok(bytes)
    }

    pub fn decode(bytes: [u8; FRAME_HEADER_LEN]) -> Result<Self, FrameError> {
        let header = Self {
            version: bytes[0],
            msg_type: u16::from_be_bytes([bytes[1], bytes[2]]),
            flags: u16::from_be_bytes([bytes[3], bytes[4]]),
            body_len: u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]),
            correlation_id: u64::from_be_bytes([
                bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
                bytes[16],
            ]),
        };
        header.validate()?;
        Ok(header)
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, FrameError> {
        let actual = bytes.len();
        let bytes = bytes
            .try_into()
            .map_err(|_| FrameError::InvalidHeaderLength {
                expected: FRAME_HEADER_LEN,
                actual,
            })?;
        Self::decode(bytes)
    }

    pub fn message_type(&self) -> Result<MessageType, FrameError> {
        self.msg_type.try_into()
    }
}

/// Provisional wire IDs in catalog order until the spec assigns stable numeric values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum MessageType {
    ClientHello = 1,
    ServerHello = 2,
    ClientFinish = 3,
    Ping = 4,
    Pong = 5,
    Close = 6,
    BootstrapRequest = 7,
    BootstrapResponse = 8,
    PublishPresence = 9,
    PublishAck = 10,
    LookupNode = 11,
    LookupResult = 12,
    LookupNotFound = 13,
    ResolveIntro = 14,
    IntroResponse = 15,
    PathProbe = 16,
    PathProbeResult = 17,
    GetServiceRecord = 18,
    ServiceRecordResponse = 19,
    OpenAppSession = 20,
    OpenAppSessionResult = 21,
}

impl TryFrom<u16> for MessageType {
    type Error = FrameError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let msg_type = match value {
            1 => Self::ClientHello,
            2 => Self::ServerHello,
            3 => Self::ClientFinish,
            4 => Self::Ping,
            5 => Self::Pong,
            6 => Self::Close,
            7 => Self::BootstrapRequest,
            8 => Self::BootstrapResponse,
            9 => Self::PublishPresence,
            10 => Self::PublishAck,
            11 => Self::LookupNode,
            12 => Self::LookupResult,
            13 => Self::LookupNotFound,
            14 => Self::ResolveIntro,
            15 => Self::IntroResponse,
            16 => Self::PathProbe,
            17 => Self::PathProbeResult,
            18 => Self::GetServiceRecord,
            19 => Self::ServiceRecordResponse,
            20 => Self::OpenAppSession,
            21 => Self::OpenAppSessionResult,
            _ => return Err(FrameError::UnknownMessageType(value)),
        };

        Ok(msg_type)
    }
}

pub trait Message {
    const TYPE: MessageType;
}

macro_rules! define_message_markers {
    ($($name:ident => $message_type:ident),* $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
            pub struct $name;

            impl Message for $name {
                const TYPE: MessageType = MessageType::$message_type;
            }
        )*
    };
}

define_message_markers!(
    Ping => Ping,
    Pong => Pong,
    Close => Close,
    BootstrapRequest => BootstrapRequest,
    BootstrapResponse => BootstrapResponse,
    PublishPresence => PublishPresence,
    PublishAck => PublishAck,
    LookupNode => LookupNode,
    LookupResult => LookupResult,
    LookupNotFound => LookupNotFound,
    ResolveIntro => ResolveIntro,
    IntroResponse => IntroResponse,
    PathProbe => PathProbe,
    PathProbeResult => PathProbeResult,
    GetServiceRecord => GetServiceRecord,
    ServiceRecordResponse => ServiceRecordResponse,
    OpenAppSession => OpenAppSession,
    OpenAppSessionResult => OpenAppSessionResult,
);

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use serde::Deserialize;

    use super::{FrameHeader, MessageType};

    #[test]
    fn frame_header_round_trips_with_big_endian_encoding() {
        let header = FrameHeader::new(
            1,
            MessageType::LookupNode,
            0x1203,
            1024,
            0x0102_0304_0506_0708,
        )
        .expect("header should be valid");

        let encoded = header.encode().expect("encode should succeed");
        assert_eq!(
            encoded,
            [1, 0, 11, 0x12, 0x03, 0, 0, 4, 0, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,]
        );

        let decoded = FrameHeader::from_slice(&encoded).expect("decode should succeed");
        assert_eq!(decoded, header);
        assert_eq!(
            decoded.message_type().expect("message type should decode"),
            MessageType::LookupNode
        );
    }

    #[test]
    fn frame_header_vector_matches_fixture() {
        let vector = read_frame_header_vector();
        let message_type = MessageType::try_from(vector.message_type_id)
            .expect("frame header vector must use a known message type");
        let header = FrameHeader::new(
            vector.version,
            message_type,
            vector.flags,
            vector.body_len,
            vector.correlation_id,
        )
        .expect("frame header vector must be valid");

        let encoded = header.encode().expect("frame header should encode");
        assert_eq!(encode_hex(&encoded), vector.encoded_hex);
        assert_eq!(header.msg_type, vector.message_type_id);
        assert_eq!(
            header.message_type().expect("message type should decode"),
            message_type
        );
    }

    #[derive(Debug, Deserialize)]
    struct FrameHeaderVector {
        version: u8,
        message_type_id: u16,
        flags: u16,
        body_len: u32,
        correlation_id: u64,
        encoded_hex: String,
    }

    fn frame_header_vector_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("tests")
            .join("vectors")
            .join("frame_header.json")
    }

    fn read_frame_header_vector() -> FrameHeaderVector {
        let bytes =
            fs::read(frame_header_vector_path()).expect("frame header vector file should exist");
        serde_json::from_slice(&bytes).expect("frame header vector file should parse")
    }

    fn encode_hex(bytes: &[u8]) -> String {
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}").expect("hex encoding should succeed");
        }

        encoded
    }
}
