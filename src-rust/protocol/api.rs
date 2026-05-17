//! API 类型定义
//! 
//! 与 src-tauri/src/infrastructure/runtime/api.rs 保持一致

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FingerprintConfig {
    pub language: Option<String>,
    pub interface_language: Option<String>,
    pub timezone: Option<String>,
    pub platform: Option<String>,
    pub user_agent: Option<String>,
    pub sound: Option<bool>,
    pub images: Option<bool>,
    pub video: Option<bool>,
    pub window_width: Option<i32>,
    pub window_height: Option<i32>,
    pub canvas: Option<String>,
    pub webgl_vendor: Option<String>,
    pub webgl_renderer: Option<String>,
    pub font_list: Option<Vec<String>>,
    pub audio_context: Option<String>,
    pub webrtc: Option<String>,
    pub do_not_track: Option<bool>,
    pub hardware_concurrency: Option<i32>,
    pub device_memory: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthInfo {
    pub is_authenticated: bool,
    pub access_token: Option<String>,
    pub user_info: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieGroup {
    pub site: String,
    pub cookie_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmptyPayload {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResponse {
    pub context_id: String,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub protocol_version: u8,
    pub client_name: String,
    pub client_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub protocol_version: u8,
    pub runtime_version: String,
    pub runtime_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeContextInput {
    pub user_id: Option<String>,
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub auth_info: Option<AuthInfo>,
    #[serde(default)]
    pub attributes: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeContextRequest {
    pub context: RuntimeContextInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DestroyContextRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum RuntimePhase {
    #[default]
    Booting = 0,
    Uninitialized = 1,
    Initializing = 2,
    Ready = 3,
    Destroying = 4,
    ShuttingDown = 5,
    Stopped = 6,
    Failed = 7,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStateSnapshot {
    pub runtime_id: String,
    pub runtime_version: String,
    pub phase: RuntimePhase,
    pub booted_at_unix_ms: u64,
    pub uptime_ms: u64,
    pub context_id: Option<String>,
    pub last_error: Option<String>,
    pub module_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResponse {
    pub state: RuntimeStateSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEventEnvelope {
    pub name: String,
    pub emitted_at_unix_ms: u64,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningEnvironment {
    pub uuid: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserProxyConfigPayload {
    pub proxy_type: Option<String>,
    pub server: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentStartRequest {
    pub exe_path: Option<String>,  // 可选，不指定则自动检测
    pub env_uuid: String,
    pub user_data_dir: String,
    pub cookies: Option<Vec<CookieGroup>>,
    pub urls: Option<Vec<String>>,
    pub proxy: Option<BrowserProxyConfigPayload>,
    pub fingerprint_config: Option<FingerprintConfig>,
    pub accounts: Option<Vec<AccountConfig>>,
    pub display_id: Option<String>,
    pub window_position: Option<String>,
    pub window_size: Option<String>,
    pub extension_dirs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchLaunchResult {
    pub env_uuid: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdpEndpointResponse {
    pub env_uuid: String,
    pub host: String,
    pub port: u16,
    pub version_url: String,
    pub list_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_ws_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowBoundsRequest {
    pub env_uuid: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentStatus {
    pub uuid: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum EnvironmentCommandRequest {
    StartEnvironment { request: EnvironmentStartRequest },
    BatchStartEnvironments { requests: Vec<EnvironmentStartRequest> },
    StopEnvironment { env_uuid: String },
    BatchStopEnvironments { env_uuids: Vec<String> },
    RefreshProxy { env_uuid: String, proxy: Option<BrowserProxyConfigPayload> },
    SetWindowBounds { request: WindowBoundsRequest },
    GetConnectedEnvironments,
    GetCdpEndpoint { env_uuid: String },
    GetEnvironmentStatus { env_uuid: String },
    GetAllEnvironmentStatuses,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnvironmentCommandResponse {
    Ack,
    Started { endpoint: CdpEndpointResponse },
    ConnectedEnvironments { env_ids: Vec<String> },
    CdpEndpoint { endpoint: Option<CdpEndpointResponse> },
    BatchLaunchResults { results: Vec<BatchLaunchResult> },
    Status { status: Option<EnvironmentStatus> },
    AllStatuses { statuses: HashMap<String, EnvironmentStatus> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentResponse {
    pub result: EnvironmentCommandResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum SyncCommandRequest {
    GetRunningEnvironments,
    StartSync { master_env_id: String, slave_env_ids: Vec<String> },
    StopSync,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncCommandResponse {
    Ack,
    RunningEnvironments { environments: Vec<RunningEnvironment> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub result: SyncCommandResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum AuthCommandRequest {
    SetAuthState { auth_info: AuthInfo },
    ClearAuthState,
    GetAuthState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthCommandResponse {
    Ack,
    State { auth_info: AuthInfo },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub result: AuthCommandResponse,
}
