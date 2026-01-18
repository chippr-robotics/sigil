//! stdio transport for MCP
//!
//! Implements the standard input/output transport for MCP servers.
//! Messages are newline-delimited JSON.

use std::io::{self, BufRead, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

use crate::protocol::{JsonRpcNotification, JsonRpcResponse};

/// stdio transport for synchronous operation
pub struct StdioTransport {
    stdin: io::BufReader<io::Stdin>,
    stdout: io::Stdout,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            stdin: io::BufReader::new(io::stdin()),
            stdout: io::stdout(),
        }
    }

    /// Read a single message from stdin
    pub fn read_message(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let bytes_read = self.stdin.read_line(&mut line)?;

        if bytes_read == 0 {
            // EOF
            return Ok(None);
        }

        // Trim the newline
        let line = line.trim_end().to_string();

        if line.is_empty() {
            return Ok(None);
        }

        trace!("Received: {}", line);
        Ok(Some(line))
    }

    /// Write a message to stdout
    pub fn write_message(&mut self, message: &str) -> io::Result<()> {
        trace!("Sending: {}", message);
        writeln!(self.stdout, "{}", message)?;
        self.stdout.flush()?;
        Ok(())
    }

    /// Write a JSON-RPC response
    pub fn write_response(&mut self, response: &JsonRpcResponse) -> io::Result<()> {
        let json = serde_json::to_string(response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.write_message(&json)
    }

    /// Write a JSON-RPC notification
    pub fn write_notification(&mut self, notification: &JsonRpcNotification) -> io::Result<()> {
        let json = serde_json::to_string(notification)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.write_message(&json)
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Async stdio transport using tokio
pub struct AsyncStdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
}

impl AsyncStdioTransport {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
        }
    }

    /// Read a single message from stdin
    pub async fn read_message(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            return Ok(None);
        }

        let line = line.trim_end().to_string();

        if line.is_empty() {
            return Ok(None);
        }

        trace!("Received: {}", line);
        Ok(Some(line))
    }

    /// Write a message to stdout
    pub async fn write_message(&mut self, message: &str) -> io::Result<()> {
        trace!("Sending: {}", message);
        self.writer.write_all(message.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// Write a JSON-RPC response
    pub async fn write_response(&mut self, response: &JsonRpcResponse) -> io::Result<()> {
        let json = serde_json::to_string(response)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.write_message(&json).await
    }

    /// Write a JSON-RPC notification
    pub async fn write_notification(&mut self, notification: &JsonRpcNotification) -> io::Result<()>
    {
        let json = serde_json::to_string(notification)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.write_message(&json).await
    }
}

impl Default for AsyncStdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Message receiver that can be used with channels
pub struct StdioReceiver {
    rx: mpsc::Receiver<String>,
}

impl StdioReceiver {
    /// Receive the next message
    pub async fn recv(&mut self) -> Option<String> {
        self.rx.recv().await
    }
}

/// Message sender that can be used with channels
#[derive(Clone)]
pub struct StdioSender {
    tx: mpsc::Sender<String>,
}

impl StdioSender {
    /// Send a message
    pub async fn send(&self, message: String) -> Result<(), mpsc::error::SendError<String>> {
        self.tx.send(message).await
    }

    /// Send a JSON-RPC response
    pub async fn send_response(
        &self,
        response: &JsonRpcResponse,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string(response)?;
        self.tx.send(json).await?;
        Ok(())
    }

    /// Send a JSON-RPC notification
    pub async fn send_notification(
        &self,
        notification: &JsonRpcNotification,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string(notification)?;
        self.tx.send(json).await?;
        Ok(())
    }
}

/// Create a channel-based stdio transport
///
/// This spawns background tasks for reading from stdin and writing to stdout,
/// returning channel-based sender and receiver for use in async code.
pub fn create_stdio_channels(buffer_size: usize) -> (StdioSender, StdioReceiver) {
    let (in_tx, in_rx) = mpsc::channel(buffer_size);
    let (out_tx, mut out_rx) = mpsc::channel::<String>(buffer_size);

    // Spawn stdin reader task
    tokio::spawn(async move {
        let mut transport = AsyncStdioTransport::new();
        loop {
            match transport.read_message().await {
                Ok(Some(msg)) => {
                    if in_tx.send(msg).await.is_err() {
                        debug!("Receiver dropped, stopping stdin reader");
                        break;
                    }
                }
                Ok(None) => {
                    debug!("EOF on stdin");
                    break;
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }
    });

    // Spawn stdout writer task
    tokio::spawn(async move {
        let mut transport = AsyncStdioTransport::new();
        while let Some(msg) = out_rx.recv().await {
            if let Err(e) = transport.write_message(&msg).await {
                error!("Error writing to stdout: {}", e);
                break;
            }
        }
        debug!("Stdout writer task ended");
    });

    (StdioSender { tx: out_tx }, StdioReceiver { rx: in_rx })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_transport_creation() {
        // Just verify we can create the transport
        let _transport = StdioTransport::new();
    }
}
