//! CDP 客户端实现

use serde_json::Value;
use reqwest::Client;
use std::time::Duration;

pub struct CdpClient {
    client: Client,
    host: String,
    port: u16,
}

impl CdpClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            host: host.to_string(),
            port,
        }
    }

    pub fn endpoint(&self, path: &str) -> String {
        format!("http://{}:{}/json{}", self.host, self.port, path)
    }

    pub async fn version(&self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = self.endpoint("/version");
        Ok(self.client.get(&url).send().await?.json().await?)
    }

    pub async fn list(&self) -> Result<Vec<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let url = self.endpoint("/list");
        Ok(self.client.get(&url).send().await?.json().await?)
    }

    pub async fn send_command(
        &self,
        ws_url: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        // 简化实现，实际应使用 WebSocket
        Ok(serde_json::json!({
            "id": 1,
            "result": {}
        }))
    }

    pub async fn evaluate(
        &self,
        ws_url: &str,
        expression: &str,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.send_command(ws_url, "Runtime.evaluate", Some(serde_json::json!({
            "expression": expression
        }))).await
    }
}
