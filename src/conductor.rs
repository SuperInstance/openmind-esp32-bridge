use std::sync::Arc;

use crate::registry;
use crate::transport::{MockTransport, Transport};
use crate::types::{BridgeError, Command, ParamType, Result, Trit};

/// The high-level ESP32 bridge — the agent's motor cortex.
pub struct Esp32Bridge {
    transport: Arc<dyn Transport>,
}

impl Esp32Bridge {
    /// Create a bridge connected via serial.
    #[cfg(feature = "serial")]
    pub async fn serial(port: &str, baud: u32) -> Result<Self> {
        let transport = crate::serial::SerialTransport::new(port, baud).await?;
        Ok(Self {
            transport: Arc::new(transport),
        })
    }

    /// Create a bridge connected via WebSocket.
    #[cfg(feature = "ws")]
    pub async fn websocket(url: &str) -> Result<Self> {
        let transport = crate::websocket::WsTransport::new(url).await?;
        Ok(Self {
            transport: Arc::new(transport),
        })
    }

    /// Create a bridge with a mock transport (for testing).
    pub fn mock() -> (Self, Arc<MockTransport>) {
        let mock = Arc::new(MockTransport::new());
        let bridge = Self {
            transport: mock.clone(),
        };
        (bridge, mock)
    }

    /// Flex a named chord — look up, serialize, send, return.
    pub async fn flex(&self, name: &str, args: &[FlexArg]) -> Result<FlexResult> {
        let chord = registry::lookup(name)
            .ok_or_else(|| BridgeError::UnknownChord(name.to_string()))?;

        validate_args(chord.name, &chord.signature.params, args)?;

        let payload = serialize_args(args);
        let cmd = Command {
            cmd_id: chord.cmd_id,
            payload,
        };

        let resp = self.transport.send_and_recv(&cmd).await?;

        if !resp.is_ok() {
            return Err(BridgeError::DeviceError {
                status: resp.status,
                detail: String::from_utf8_lossy(&resp.payload).to_string(),
            });
        }

        Ok(deserialize_result(&chord.signature.ret, &resp.payload))
    }
}

/// Arguments that can be passed to flex().
#[derive(Debug, Clone, PartialEq)]
pub enum FlexArg {
    U8(u8),
    U16(u16),
    Usize(usize),
    Bytes(Vec<u8>),
    String(String),
    Trit(Trit),
}

impl From<u8> for FlexArg {
    fn from(v: u8) -> Self { FlexArg::U8(v) }
}

impl From<u16> for FlexArg {
    fn from(v: u16) -> Self { FlexArg::U16(v) }
}

impl From<Trit> for FlexArg {
    fn from(v: Trit) -> Self { FlexArg::Trit(v) }
}

impl From<Vec<u8>> for FlexArg {
    fn from(v: Vec<u8>) -> Self { FlexArg::Bytes(v) }
}

impl From<&str> for FlexArg {
    fn from(v: &str) -> Self { FlexArg::String(v.to_string()) }
}

/// Result from flex().
#[derive(Debug, Clone, PartialEq)]
pub enum FlexResult {
    None,
    Trit(Trit),
    U8(u8),
    U16(u16),
    Bytes(Vec<u8>),
}

fn validate_args(name: &str, expected: &[ParamType], args: &[FlexArg]) -> Result<()> {
    if args.len() != expected.len() {
        return Err(BridgeError::WrongArgs {
            name: name.to_string(),
            detail: format!("expected {} args, got {}", expected.len(), args.len()),
        });
    }

    for (i, (exp, arg)) in expected.iter().zip(args.iter()).enumerate() {
        let ok = match (exp, arg) {
            (ParamType::U8, FlexArg::U8(_)) => true,
            (ParamType::U16, FlexArg::U16(_)) => true,
            (ParamType::Usize, FlexArg::Usize(_)) => true,
            (ParamType::Bytes, FlexArg::Bytes(_)) => true,
            (ParamType::String, FlexArg::String(_)) => true,
            (ParamType::Trit, FlexArg::Trit(_)) => true,
            _ => false,
        };
        if !ok {
            return Err(BridgeError::WrongArgs {
                name: name.to_string(),
                detail: format!("arg {i}: expected {exp}, got {}", arg_type_name(arg)),
            });
        }
    }

    Ok(())
}

fn arg_type_name(arg: &FlexArg) -> &'static str {
    match arg {
        FlexArg::U8(_) => "u8",
        FlexArg::U16(_) => "u16",
        FlexArg::Usize(_) => "usize",
        FlexArg::Bytes(_) => "bytes",
        FlexArg::String(_) => "string",
        FlexArg::Trit(_) => "Trit",
    }
}

fn serialize_args(args: &[FlexArg]) -> Vec<u8> {
    let mut buf = Vec::new();
    for arg in args {
        match arg {
            FlexArg::U8(v) => buf.push(*v),
            FlexArg::U16(v) => buf.extend_from_slice(&v.to_le_bytes()),
            FlexArg::Usize(v) => buf.extend_from_slice(&(*v as u16).to_le_bytes()),
            FlexArg::Bytes(v) => {
                buf.extend_from_slice(&(v.len() as u16).to_le_bytes());
                buf.extend_from_slice(v);
            }
            FlexArg::String(v) => {
                let bytes = v.as_bytes();
                buf.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
            FlexArg::Trit(v) => buf.push(v.to_byte()),
        }
    }
    buf
}

fn deserialize_result(ret: &Option<ParamType>, payload: &[u8]) -> FlexResult {
    match ret {
        None => FlexResult::None,
        Some(ParamType::Trit) => {
            let t = payload.first().and_then(|&b| Trit::from_byte(b)).unwrap_or(Trit::Zero);
            FlexResult::Trit(t)
        }
        Some(ParamType::U8) => FlexResult::U8(payload.first().copied().unwrap_or(0)),
        Some(ParamType::U16) => {
            let v = if payload.len() >= 2 {
                u16::from_le_bytes([payload[0], payload[1]])
            } else {
                0
            };
            FlexResult::U16(v)
        }
        Some(ParamType::Bytes) => FlexResult::Bytes(payload.to_vec()),
        Some(ParamType::Usize) => FlexResult::U16(
            if payload.len() >= 2 {
                u16::from_le_bytes([payload[0], payload[1]])
            } else {
                0
            },
        ),
        Some(ParamType::String) => FlexResult::Bytes(payload.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Response;

    #[tokio::test]
    async fn flex_gpio_read() {
        let (bridge, mock) = Esp32Bridge::mock();
        mock.push_response(Response { status: Response::OK, payload: vec![2] }).await;

        let result = bridge.flex("gpio_read", &[FlexArg::U8(4)]).await.unwrap();
        assert_eq!(result, FlexResult::Trit(Trit::PlusOne));
    }

    #[tokio::test]
    async fn flex_gpio_write() {
        let (bridge, mock) = Esp32Bridge::mock();
        mock.push_response(Response { status: Response::OK, payload: vec![] }).await;

        let result = bridge.flex("gpio_write", &[FlexArg::U8(2), FlexArg::Trit(Trit::PlusOne)]).await.unwrap();
        assert_eq!(result, FlexResult::None);
    }

    #[tokio::test]
    async fn flex_unknown_chord() {
        let (bridge, _mock) = Esp32Bridge::mock();
        let err = bridge.flex("nonexistent", &[]).await.unwrap_err();
        assert!(matches!(err, BridgeError::UnknownChord(_)));
    }

    #[tokio::test]
    async fn flex_wrong_arg_count() {
        let (bridge, _mock) = Esp32Bridge::mock();
        let err = bridge.flex("gpio_read", &[]).await.unwrap_err();
        assert!(matches!(err, BridgeError::WrongArgs { .. }));
    }

    #[tokio::test]
    async fn flex_wrong_arg_type() {
        let (bridge, _mock) = Esp32Bridge::mock();
        let err = bridge.flex("gpio_read", &[FlexArg::U16(4)]).await.unwrap_err();
        assert!(matches!(err, BridgeError::WrongArgs { .. }));
    }

    #[tokio::test]
    async fn flex_device_error() {
        let (bridge, mock) = Esp32Bridge::mock();
        mock.push_response(Response { status: Response::ERROR, payload: b"pin not available".to_vec() }).await;

        let err = bridge.flex("gpio_read", &[FlexArg::U8(99)]).await.unwrap_err();
        assert!(matches!(err, BridgeError::DeviceError { .. }));
    }

    #[tokio::test]
    async fn flex_adc_read() {
        let (bridge, mock) = Esp32Bridge::mock();
        mock.push_response(Response { status: Response::OK, payload: 1024u16.to_le_bytes().to_vec() }).await;

        let result = bridge.flex("adc_read", &[FlexArg::U8(4)]).await.unwrap();
        assert_eq!(result, FlexResult::U16(1024));
    }

    #[tokio::test]
    async fn flex_disconnected() {
        let (bridge, _mock) = Esp32Bridge::mock();
        let err = bridge.flex("gpio_read", &[FlexArg::U8(4)]).await.unwrap_err();
        assert!(matches!(err, BridgeError::Disconnected));
    }
}
