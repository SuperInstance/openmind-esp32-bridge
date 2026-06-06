use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

use crate::framing;
use crate::transport::Transport;
use crate::types::{BridgeError, Command, Response, Result};

const DEFAULT_TIMEOUT_MS: u64 = 5000;
const DEFAULT_RETRIES: usize = 2;

/// Serial (UART/USB) transport to ESP32.
pub struct SerialTransport {
    port: tokio::sync::Mutex<tokio_serial::SerialStream>,
    timeout_ms: u64,
    retries: usize,
}

impl SerialTransport {
    pub async fn new(port: &str, baud: u32) -> Result<Self> {
        let builder = tokio_serial::new(port, baud);
        let stream = tokio_serial::SerialStream::open(&builder)
            .map_err(|e| BridgeError::Serial(e.to_string()))?;
        Ok(Self {
            port: tokio::sync::Mutex::new(stream),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            retries: DEFAULT_RETRIES,
        })
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub fn with_retries(mut self, n: usize) -> Self {
        self.retries = n;
        self
    }

    async fn do_send_recv(&self, cmd: &Command) -> Result<Response> {
        let frame = framing::encode_command(cmd)?;
        let mut port = self.port.lock().await;

        port.write_all(&frame)
            .await
            .map_err(|e| BridgeError::Serial(e.to_string()))?;

        let mut header = [0u8; 3];
        port.read_exact(&mut header)
            .await
            .map_err(|e| BridgeError::Serial(e.to_string()))?;

        if header[0] != crate::types::frame::RSP_HEADER {
            return Err(BridgeError::Frame(format!(
                "bad response header: {:02X}",
                header[0]
            )));
        }

        let inner_len = u16::from_le_bytes([header[1], header[2]]) as usize;
        let remaining = inner_len + 2;

        let mut rest = vec![0u8; remaining];
        port.read_exact(&mut rest)
            .await
            .map_err(|e| BridgeError::Serial(e.to_string()))?;

        let mut full = header.to_vec();
        full.extend_from_slice(&rest);

        framing::decode_response(&full)
    }
}

#[async_trait]
impl Transport for SerialTransport {
    async fn send_and_recv(&self, cmd: &Command) -> Result<Response> {
        let dur = Duration::from_millis(self.timeout_ms);
        let mut last_err = None;

        for _ in 0..=self.retries {
            match timeout(dur, self.do_send_recv(cmd)).await {
                Ok(Ok(resp)) => return Ok(resp),
                Ok(Err(e)) => {
                    last_err = Some(e);
                }
                Err(_) => {
                    last_err = Some(BridgeError::Timeout(self.timeout_ms));
                }
            }
        }

        Err(last_err.unwrap())
    }
}
