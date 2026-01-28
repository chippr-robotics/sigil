//! Daemon IPC client for sigil-bridge

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::debug;

/// Client for communicating with sigil-daemon via IPC
pub struct DaemonClient {
    socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: PathBuf::from(socket_path),
        }
    }

    async fn send_request(&self, request: Value) -> Result<Value> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Send request
        let request_str = serde_json::to_string(&request)?;
        debug!("Sending request: {}", request_str);
        writer.write_all(request_str.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // Read response
        let mut response_str = String::new();
        reader.read_line(&mut response_str).await?;
        debug!("Received response: {}", response_str);

        let response: Value = serde_json::from_str(&response_str)?;

        // Check for error response
        if let Some(msg) = response.get("message") {
            if response.get("type") == Some(&Value::String("Error".to_string())) {
                return Err(anyhow!("{}", msg.as_str().unwrap_or("Unknown error")));
            }
        }

        Ok(response)
    }

    /// Ping the daemon
    pub async fn ping(&self) -> Result<String> {
        let response = self.send_request(serde_json::json!({
            "type": "Ping"
        })).await?;

        Ok(response["version"]
            .as_str()
            .unwrap_or("unknown")
            .to_string())
    }

    /// Get disk status
    pub async fn get_disk_status(&self) -> Result<Value> {
        let response = self.send_request(serde_json::json!({
            "type": "GetDiskStatus"
        })).await?;
        Ok(response)
    }

    /// Get presignature count
    pub async fn get_presig_count(&self) -> Result<Value> {
        let response = self.send_request(serde_json::json!({
            "type": "GetPresigCount"
        })).await?;
        Ok(response)
    }

    /// Sign a message (ECDSA)
    pub async fn sign(&self, message_hash: &str, chain_id: u32, description: &str) -> Result<Value> {
        let response = self.send_request(serde_json::json!({
            "type": "Sign",
            "message_hash": message_hash,
            "chain_id": chain_id,
            "description": description
        })).await?;
        Ok(response)
    }

    /// Sign with FROST
    pub async fn sign_frost(&self, scheme: &str, message_hash: &str, description: &str) -> Result<Value> {
        // FROST signing would go through a separate endpoint if supported by daemon
        // For now, simulate the response format
        let response = self.send_request(serde_json::json!({
            "type": "SignFrost",
            "scheme": scheme,
            "message_hash": message_hash,
            "description": description
        })).await?;
        Ok(response)
    }

    /// Get address
    pub async fn get_address(&self, scheme: Option<&str>, format: &str, cosmos_prefix: Option<&str>) -> Result<Value> {
        let mut request = serde_json::json!({
            "type": "GetAddress",
            "format": format
        });

        if let Some(s) = scheme {
            request["scheme"] = Value::String(s.to_string());
        }
        if let Some(p) = cosmos_prefix {
            request["cosmos_prefix"] = Value::String(p.to_string());
        }

        let response = self.send_request(request).await?;
        Ok(response)
    }

    /// Update transaction hash
    pub async fn update_tx_hash(&self, presig_index: u32, tx_hash: &str) -> Result<()> {
        self.send_request(serde_json::json!({
            "type": "UpdateTxHash",
            "presig_index": presig_index,
            "tx_hash": tx_hash
        })).await?;
        Ok(())
    }

    /// List children
    pub async fn list_children(&self) -> Result<Vec<String>> {
        let response = self.send_request(serde_json::json!({
            "type": "ListChildren"
        })).await?;

        let children: Vec<String> = response["child_ids"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(children)
    }

    /// Import agent shard
    pub async fn import_agent_shard(&self, shard_hex: &str) -> Result<()> {
        self.send_request(serde_json::json!({
            "type": "ImportAgentShard",
            "agent_shard_hex": shard_hex
        })).await?;
        Ok(())
    }

    /// Import child shares
    pub async fn import_child_shares(&self, shares_json: &str, replace: bool) -> Result<()> {
        self.send_request(serde_json::json!({
            "type": "ImportChildShares",
            "shares_json": shares_json,
            "replace": replace
        })).await?;
        Ok(())
    }
}
