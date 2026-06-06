use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::framing;
use crate::transport::Transport;
use crate::types::{BridgeError, Command, Response, Result};

type WsStream = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// WebSocket transport for remote ESP32s.
pub struct WsTransport {
    inner: Mutex<Option<WsStream>>,
    url: String,
}

impl WsTransport {
    pub async fn new(url: &str) -> Result<Self> {
        let ws = Self::connect(url).await?;
        Ok(Self {
            inner: Mutex::new(Some(ws)),
            url: url.to_string(),
        })
    }

    async fn connect(url: &str) -> Result<WsStream> {
        let (ws, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| BridgeError::WebSocket(e.to_string()))?;
        Ok(ws)
    }

    pub async fn reconnect(&self) -> Result<()> {
        let ws = Self::connect(&self.url).await?;
        let mut guard = self.inner.lock().await;
        *guard = Some(ws);
        Ok(())
    }
}

#[async_trait]
impl Transport for WsTransport {
    async fn send_and_recv(&self, cmd: &Command) -> Result<Response> {
        let frame = framing::encode_command(cmd)?;
        let mut guard = self.inner.lock().await;

        let ws = guard.as_mut().ok_or(BridgeError::Disconnected)?;

        ws.send(Message::Binary(frame.into()))
            .await
            .map_err(|e| BridgeError::WebSocket(e.to_string()))?;

        loop {
            match ws.next().await {
                Some(Ok(Message::Binary(data))) => {
                    return framing::decode_response(&data);
                }
                Some(Ok(Message::Ping(_))) => continue,
                Some(Ok(Message::Close(_))) | None => {
                    *guard = None;
                    return Err(BridgeError::Disconnected);
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => {
                    *guard = None;
                    return Err(BridgeError::WebSocket(e.to_string()));
                }
            }
        }
    }
}
