use async_trait::async_trait;
use crate::types::{BridgeError, Command, Response, Result};

/// Async transport trait for communicating with ESP32.
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send_and_recv(&self, cmd: &Command) -> Result<Response>;
}

/// Mock transport for testing.
pub struct MockTransport {
    responses: tokio::sync::Mutex<Vec<Response>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            responses: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub async fn push_response(&self, resp: Response) {
        self.responses.lock().await.push(resp);
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn send_and_recv(&self, _cmd: &Command) -> Result<Response> {
        let mut responses = self.responses.lock().await;
        responses.pop().ok_or(BridgeError::Disconnected)
    }
}
