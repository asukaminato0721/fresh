//! Agent communication channel
//!
//! Handles request/response multiplexing over SSH stdin/stdout.

use crate::services::remote::protocol::{AgentRequest, AgentResponse};
use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};

/// Error type for channel operations
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Request cancelled")]
    Cancelled,

    #[error("Request timed out")]
    Timeout,

    #[error("Remote error: {0}")]
    Remote(String),
}

/// Pending request state
struct PendingRequest {
    /// Channel for streaming data
    data_tx: mpsc::Sender<serde_json::Value>,
    /// Channel for final result
    result_tx: oneshot::Sender<Result<serde_json::Value, String>>,
}

/// Communication channel with the remote agent
pub struct AgentChannel {
    /// Sender to the write task
    write_tx: mpsc::Sender<String>,
    /// Pending requests awaiting responses
    pending: Arc<Mutex<HashMap<u64, PendingRequest>>>,
    /// Next request ID
    next_id: AtomicU64,
    /// Whether the channel is connected
    connected: Arc<std::sync::atomic::AtomicBool>,
    /// Runtime handle for blocking operations
    runtime_handle: tokio::runtime::Handle,
}

impl AgentChannel {
    /// Create a new channel from async read/write handles
    ///
    /// Must be called from within a Tokio runtime context.
    pub fn new(
        mut reader: tokio::io::BufReader<tokio::process::ChildStdout>,
        mut writer: tokio::process::ChildStdin,
    ) -> Self {
        let pending: Arc<Mutex<HashMap<u64, PendingRequest>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let connected = Arc::new(std::sync::atomic::AtomicBool::new(true));
        // Capture the runtime handle for later use in blocking operations
        let runtime_handle = tokio::runtime::Handle::current();

        // Channel for outgoing requests
        let (write_tx, mut write_rx) = mpsc::channel::<String>(64);

        // Spawn write task
        let connected_write = connected.clone();
        tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                if writer.write_all(msg.as_bytes()).await.is_err() {
                    connected_write.store(false, Ordering::SeqCst);
                    break;
                }
                if writer.flush().await.is_err() {
                    connected_write.store(false, Ordering::SeqCst);
                    break;
                }
            }
        });

        // Spawn read task
        let pending_read = pending.clone();
        let connected_read = connected.clone();
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        // EOF
                        connected_read.store(false, Ordering::SeqCst);
                        break;
                    }
                    Ok(_) => {
                        if let Ok(resp) = serde_json::from_str::<AgentResponse>(&line) {
                            Self::handle_response(&pending_read, resp);
                        }
                    }
                    Err(_) => {
                        connected_read.store(false, Ordering::SeqCst);
                        break;
                    }
                }
            }

            // Clean up pending requests on disconnect
            let mut pending = pending_read.lock().unwrap();
            for (_, req) in pending.drain() {
                let _ = req.result_tx.send(Err("connection closed".to_string()));
            }
        });

        Self {
            write_tx,
            pending,
            next_id: AtomicU64::new(1),
            connected,
            runtime_handle,
        }
    }

    /// Handle an incoming response
    fn handle_response(pending: &Arc<Mutex<HashMap<u64, PendingRequest>>>, resp: AgentResponse) {
        let mut pending = pending.lock().unwrap();

        if let Some(req) = pending.get(&resp.id) {
            if let Some(data) = resp.data {
                // Streaming data - send to channel (ignore if receiver dropped)
                let _ = req.data_tx.try_send(data);
            }

            if let Some(result) = resp.result {
                // Success - complete request
                if let Some(req) = pending.remove(&resp.id) {
                    let _ = req.result_tx.send(Ok(result));
                }
            } else if let Some(error) = resp.error {
                // Error - complete request
                if let Some(req) = pending.remove(&resp.id) {
                    let _ = req.result_tx.send(Err(error));
                }
            }
        }
    }

    /// Check if the channel is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Send a request and wait for the final result (ignoring streaming data)
    pub async fn request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ChannelError> {
        let (mut data_rx, result_rx) = self.request_streaming(method, params).await?;

        // Drain streaming data
        while data_rx.recv().await.is_some() {}

        // Wait for final result
        result_rx
            .await
            .map_err(|_| ChannelError::ChannelClosed)?
            .map_err(ChannelError::Remote)
    }

    /// Send a request that may stream data
    pub async fn request_streaming(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<
        (
            mpsc::Receiver<serde_json::Value>,
            oneshot::Receiver<Result<serde_json::Value, String>>,
        ),
        ChannelError,
    > {
        if !self.is_connected() {
            return Err(ChannelError::ChannelClosed);
        }

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Create channels for response
        let (data_tx, data_rx) = mpsc::channel(64);
        let (result_tx, result_rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.pending.lock().unwrap();
            pending.insert(id, PendingRequest { data_tx, result_tx });
        }

        // Build and send request
        let req = AgentRequest::new(id, method, params);
        self.write_tx
            .send(req.to_json_line())
            .await
            .map_err(|_| ChannelError::ChannelClosed)?;

        Ok((data_rx, result_rx))
    }

    /// Send a request synchronously (blocking)
    ///
    /// This can be called from outside the Tokio runtime context.
    pub fn request_blocking(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ChannelError> {
        self.runtime_handle.block_on(self.request(method, params))
    }

    /// Send a request and collect all streaming data along with the final result
    pub async fn request_with_data(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(Vec<serde_json::Value>, serde_json::Value), ChannelError> {
        let (mut data_rx, result_rx) = self.request_streaming(method, params).await?;

        // Collect all streaming data
        let mut data = Vec::new();
        while let Some(chunk) = data_rx.recv().await {
            data.push(chunk);
        }

        // Wait for final result
        let result = result_rx
            .await
            .map_err(|_| ChannelError::ChannelClosed)?
            .map_err(ChannelError::Remote)?;

        Ok((data, result))
    }

    /// Send a request with streaming data, synchronously (blocking)
    ///
    /// This can be called from outside the Tokio runtime context.
    pub fn request_with_data_blocking(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(Vec<serde_json::Value>, serde_json::Value), ChannelError> {
        self.runtime_handle
            .block_on(self.request_with_data(method, params))
    }

    /// Cancel a request
    pub async fn cancel(&self, request_id: u64) -> Result<(), ChannelError> {
        use crate::services::remote::protocol::cancel_params;
        self.request("cancel", cancel_params(request_id)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Tests are in the tests module to allow integration testing with mock agent
}
