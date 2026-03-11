use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    thread,
    time::Duration,
};

use crate::signal::{install_shutdown_handlers, pending_shutdown_signal};

const MAX_HTTP_REQUEST_BYTES: usize = 8 * 1024;
const READ_TIMEOUT_MS: u64 = 250;
const ACCEPT_POLL_MS: u64 = 50;

pub fn run(
    bind_addr: &str,
    bootstrap_file: &Path,
    max_requests: Option<usize>,
) -> Result<(), String> {
    let bind_addr = bind_addr.trim();
    if bind_addr.is_empty() {
        return Err("bootstrap-serve requires --bind <addr>".to_string());
    }
    if bootstrap_file.as_os_str().is_empty() {
        return Err("bootstrap-serve requires --bootstrap-file <path>".to_string());
    }

    let listener = TcpListener::bind(bind_addr)
        .map_err(|error| format!("failed to bind bootstrap server on {bind_addr}: {error}"))?;
    listener.set_nonblocking(true).map_err(|error| {
        format!("failed to configure bootstrap listener nonblocking mode: {error}")
    })?;
    install_shutdown_handlers()?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read bootstrap server local address: {error}"))?;
    println!(
        "{}",
        serde_json::json!({
            "step": "bootstrap_server_listen",
            "bind": local_addr.to_string(),
            "bootstrap_file": bootstrap_file.display().to_string(),
            "max_requests": max_requests,
        })
    );

    let mut served = 0usize;
    loop {
        if let Some(signal) = pending_shutdown_signal() {
            println!(
                "{}",
                serde_json::json!({
                    "step": "bootstrap_server_shutdown",
                    "bind": local_addr.to_string(),
                    "signal": signal.as_str(),
                    "served_requests": served,
                })
            );
            break;
        }
        let (mut stream, _) = match listener.accept() {
            Ok(connection) => connection,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                continue;
            }
            Err(error) => {
                return Err(format!("failed to accept bootstrap connection: {error}"));
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_millis(READ_TIMEOUT_MS)))
            .map_err(|error| format!("failed to configure bootstrap read timeout: {error}"))?;

        read_http_request(&mut stream)?;
        let response = build_http_response(bootstrap_file);
        stream
            .write_all(&response)
            .map_err(|error| format!("failed to write bootstrap response: {error}"))?;
        served = served.saturating_add(1);
        if max_requests.is_some_and(|limit| served >= limit) {
            break;
        }
    }

    Ok(())
}

fn read_http_request(stream: &mut impl Read) -> Result<(), String> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 512];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(read) => {
                request.extend_from_slice(&buffer[..read]);
                if request.len() > MAX_HTTP_REQUEST_BYTES {
                    return Err(format!(
                        "bootstrap HTTP request exceeded {MAX_HTTP_REQUEST_BYTES} bytes"
                    ));
                }
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    return Ok(());
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(());
            }
            Err(error) => return Err(format!("failed to read bootstrap request: {error}")),
        }
    }
}

fn build_http_response(bootstrap_file: &Path) -> Vec<u8> {
    match fs::read(bootstrap_file) {
        Ok(body) => {
            let mut response = ok_headers(body.len());
            response.extend_from_slice(&body);
            response
        }
        Err(error) => plain_response(
            "500 Internal Server Error",
            format!(
                "failed to read bootstrap file {}: {error}",
                bootstrap_file.display()
            )
            .into_bytes(),
        ),
    }
}

fn ok_headers(body_len: usize) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n"
    )
    .into_bytes()
}

fn plain_response(status: &str, body: Vec<u8>) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);
    response
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpStream,
        path::PathBuf,
        thread,
        time::Duration,
    };

    use super::run;

    #[test]
    fn bootstrap_server_serves_configured_file() {
        let dir = std::env::temp_dir().join(format!(
            "overlay-cli-bootstrap-server-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should exist");
        let bootstrap_path = dir.join("bootstrap.json");
        std::fs::write(&bootstrap_path, br#"{"version":1}"#).expect("bootstrap file should exist");

        let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return,
            Err(error) => panic!("probe listener should bind: {error}"),
        };
        let bind_addr = listener
            .local_addr()
            .expect("probe listener should expose local addr");
        drop(listener);

        let server_path = PathBuf::from(&bootstrap_path);
        let handle = thread::spawn(move || {
            run(&bind_addr.to_string(), &server_path, Some(1)).expect("server should run");
        });

        let mut stream = None;
        for _ in 0..20 {
            if let Ok(candidate) = TcpStream::connect(bind_addr) {
                stream = Some(candidate);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let mut stream = stream.expect("client should connect to bootstrap server");
        stream
            .write_all(b"GET /bootstrap.json HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("request should write");
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .expect("response should read");
        handle.join().expect("server thread should exit");

        let response = String::from_utf8(response).expect("response should be utf-8");
        assert!(response.contains("HTTP/1.1 200 OK"));
        assert!(response.contains(r#"{"version":1}"#));
    }
}
