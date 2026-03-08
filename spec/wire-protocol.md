# Wire Protocol

## Frame header
- `version: u8`
- `msg_type: u16`
- `flags: u16`
- `body_len: u32`
- `correlation_id: u64`

## Encoding
- big-endian integers
- max frame size for MVP: 64 KiB

## Message catalog
### Session
- `ClientHello`
- `ServerHello`
- `ClientFinish`
- `Ping`
- `Pong`
- `Close`

### Bootstrap
- `BootstrapRequest`
- `BootstrapResponse`

### Presence / lookup
- `PublishPresence`
- `PublishAck`
- `LookupNode`
- `LookupResult`
- `LookupNotFound`
- `ResolveIntro`
- `IntroResponse`

### Routing
- `PathProbe`
- `PathProbeResult`

### Service
- `GetServiceRecord`
- `ServiceRecordResponse`
- `OpenAppSession`
- `OpenAppSessionResult`
