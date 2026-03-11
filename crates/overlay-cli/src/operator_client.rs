use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::Path,
    time::Duration,
};

use getrandom::getrandom;
use overlay_core::{
    crypto::kex::X25519StaticSecret,
    runtime::NodeRuntime,
    session::{ClientHandshake, HandshakeConfig, ServerHello},
    wire::{
        decode_framed_message, decode_message_body, encode_framed_message, Close, Message,
        MessageType, WireCodecError,
    },
};
use serde::{de::DeserializeOwned, Serialize};

const CONNECT_TIMEOUT_MS: u64 = 1_000;
const IO_TIMEOUT_MS: u64 = 2_000;
const SESSION_CORRELATION_ID: u64 = 1;

pub struct OperatorSessionClient {
    stream: TcpStream,
}

impl OperatorSessionClient {
    pub fn connect(config_path: &Path, dial_hint: &str) -> Result<Self, String> {
        let runtime =
            NodeRuntime::from_config_path(config_path).map_err(|error| error.to_string())?;
        let signing_key = runtime.context().signing_key().clone();
        let endpoint = parse_tcp_dial_hint(dial_hint)?;
        let socket_addr = endpoint
            .to_socket_addrs()
            .map_err(|error| format!("failed to resolve {endpoint}: {error}"))?
            .next()
            .ok_or_else(|| format!("dial target {endpoint} did not resolve"))?;
        let timeout = Duration::from_millis(CONNECT_TIMEOUT_MS);
        let mut stream = TcpStream::connect_timeout(&socket_addr, timeout)
            .map_err(|error| format!("failed to connect to {endpoint}: {error}"))?;
        let io_timeout = Some(Duration::from_millis(IO_TIMEOUT_MS));
        stream
            .set_read_timeout(io_timeout)
            .map_err(|error| format!("failed to configure read timeout: {error}"))?;
        stream
            .set_write_timeout(io_timeout)
            .map_err(|error| format!("failed to configure write timeout: {error}"))?;

        let ephemeral_secret = random_ephemeral_secret()?;
        let (client_handshake, client_hello) =
            ClientHandshake::start(HandshakeConfig::default(), signing_key, ephemeral_secret);
        write_message(&mut stream, &client_hello, SESSION_CORRELATION_ID)?;

        let (message_type, response_correlation_id, body) = read_frame(&mut stream)?;
        if message_type != MessageType::ServerHello {
            return Err(format!(
                "expected server_hello during operator session open, got {message_type:?}"
            ));
        }
        let server_hello: ServerHello =
            decode_message_body(&body).map_err(|error: WireCodecError| error.to_string())?;
        let (client_finish, _) = client_handshake
            .handle_server_hello(&server_hello)
            .map_err(|error| error.to_string())?;
        write_message(&mut stream, &client_finish, response_correlation_id)?;

        Ok(Self { stream })
    }

    pub fn request<T>(&mut self, request: &T) -> Result<(MessageType, u64, Vec<u8>), String>
    where
        T: Message + Serialize,
    {
        write_message(&mut self.stream, request, SESSION_CORRELATION_ID)?;
        read_frame(&mut self.stream)
    }

    pub fn request_typed<T, R>(
        &mut self,
        request: &T,
        expected_type: MessageType,
    ) -> Result<R, String>
    where
        T: Message + Serialize,
        R: DeserializeOwned,
    {
        let (message_type, _, body) = self.request(request)?;
        if message_type != expected_type {
            return Err(format!(
                "expected {expected_type:?} response, got {message_type:?}"
            ));
        }
        decode_message_body(&body).map_err(|error: WireCodecError| error.to_string())
    }

    pub fn close(mut self) -> Result<(), String> {
        write_message(&mut self.stream, &Close, SESSION_CORRELATION_ID)
    }
}

fn write_message<T>(stream: &mut TcpStream, message: &T, correlation_id: u64) -> Result<(), String>
where
    T: Message + Serialize,
{
    let frame =
        encode_framed_message(message, correlation_id).map_err(|error| error.to_string())?;
    stream
        .write_all(&frame)
        .map_err(|error| format!("failed to write framed message: {error}"))
}

fn read_frame(stream: &mut TcpStream) -> Result<(MessageType, u64, Vec<u8>), String> {
    let mut header_bytes = [0_u8; overlay_core::wire::FRAME_HEADER_LEN];
    stream
        .read_exact(&mut header_bytes)
        .map_err(|error| format!("failed to read frame header: {error}"))?;
    let header =
        overlay_core::wire::FrameHeader::decode(header_bytes).map_err(|error| error.to_string())?;
    let mut body = vec![0_u8; header.body_len as usize];
    stream
        .read_exact(&mut body)
        .map_err(|error| format!("failed to read frame body: {error}"))?;
    let mut framed = Vec::with_capacity(header_bytes.len() + body.len());
    framed.extend_from_slice(&header_bytes);
    framed.extend_from_slice(&body);
    let (decoded_header, decoded_body) =
        decode_framed_message(&framed).map_err(|error| error.to_string())?;
    let message_type = decoded_header
        .message_type()
        .map_err(|error| error.to_string())?;
    Ok((
        message_type,
        decoded_header.correlation_id,
        decoded_body.to_vec(),
    ))
}

fn parse_tcp_dial_hint(dial_hint: &str) -> Result<String, String> {
    let Some(endpoint) = dial_hint.trim().strip_prefix("tcp://") else {
        return Err(format!(
            "operator target must use tcp://host:port, got '{}'",
            dial_hint.trim()
        ));
    };
    if endpoint.is_empty() {
        return Err("operator target must not be empty".to_string());
    }
    Ok(endpoint.to_string())
}

fn random_ephemeral_secret() -> Result<X25519StaticSecret, String> {
    let mut bytes = [0_u8; 32];
    getrandom(&mut bytes).map_err(|error| error.to_string())?;
    Ok(X25519StaticSecret::from_bytes(bytes))
}
