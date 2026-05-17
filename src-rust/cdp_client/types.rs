//! CDP 类型定义

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpTarget {
    pub id: String,
    pub r#type: String,
    pub title: String,
    pub url: String,
    pub web_socket_debugger_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpVersion {
    pub browser: String,
    pub protocol_version: String,
    pub webkit_version: String,
    pub js_version: String,
}
