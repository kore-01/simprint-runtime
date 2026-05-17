//! Simprint Runtime Linux - 主程序
//! 
//! 通过 stdin/stdout 与 Tauri 应用通信，管理浏览器实例。

mod protocol;
mod browser;
mod fingerprint;
mod cdp_client;

use std::sync::Arc;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::sync::Mutex;
use parking_lot::RwLock;
use bytes::{Buf, Bytes, BytesMut};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use uuid::Uuid;

use protocol::{Topic, Message, ErrorCode, encode_payload, decode_payload, PROTOCOL_VERSION};
use protocol::api::*;
use browser::BrowserManager;
use fingerprint::FingerprintGenerator;

const RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Default)]
struct RuntimeState {
    phase: protocol::RuntimePhase,
    context_id: Option<String>,
    auth_info: Option<AuthInfo>,
    environments: std::collections::HashMap<String, EnvInfo>,
}

struct EnvInfo {
    _name: String,
    _status: String,
    cdp_port: u16,
}

struct AppState {
    runtime_id: String,
    runtime_version: String,
    booted_at_unix_ms: u64,
    state: RwLock<RuntimeState>,
    browser_manager: Mutex<BrowserManager>,
    fingerprint_generator: FingerprintGenerator,
}

impl AppState {
    fn new() -> Self {
        Self {
            runtime_id: Uuid::new_v4().to_string(),
            runtime_version: RUNTIME_VERSION.to_string(),
            booted_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            state: RwLock::new(RuntimeState::default()),
            browser_manager: Mutex::new(BrowserManager::new()),
            fingerprint_generator: FingerprintGenerator::new(),
        }
    }

    fn get_state_snapshot(&self) -> RuntimeStateSnapshot {
        let state = self.state.read();
        let uptime_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64 - self.booted_at_unix_ms;
        
        RuntimeStateSnapshot {
            runtime_id: self.runtime_id.clone(),
            runtime_version: self.runtime_version.clone(),
            phase: state.phase,
            booted_at_unix_ms: self.booted_at_unix_ms,
            uptime_ms,
            context_id: state.context_id.clone(),
            last_error: None,
            module_count: state.environments.len(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env().add_directive("simprint_runtime=info".parse()?))
        .init();

    log::info!("Simprint Runtime Linux v{} starting...", RUNTIME_VERSION);

    let state = Arc::new(AppState::new());
    let _shutdown_tx = tokio::sync::broadcast::channel::<()>(1).0;

    // 处理 stdin 输入
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = run_stdin_loop(state_clone).await {
            log::error!("Stdin loop error: {}", e);
        }
    });

    // 等待中断信号
    tokio::signal::ctrl_c().await?;
    log::info!("Received Ctrl+C, shutting down");

    Ok(())
}

async fn run_stdin_loop(state: Arc<AppState>) -> Result<(), String> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    
    // 在独立线程中读取 stdin（tokio stdin 在 Linux 上不是真正异步）
    std::thread::spawn(move || {
        use std::io::Read;
        let mut stdin = std::io::stdin();
        let mut tmp = [0u8; 8192];
        loop {
            match stdin.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.blocking_send(tmp[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    let mut buffer = BytesMut::with_capacity(1024 * 1024);
    
    while let Some(chunk) = rx.recv().await {
        buffer.extend_from_slice(&chunk);
        
        // 尝试解析完整帧
        loop {
            if buffer.len() < 9 {
                break;
            }
            if &buffer[..4] != b"sprt" {
                buffer.advance(1);
                continue;
            }
            let payload_len = u32::from_le_bytes(buffer[5..9].try_into().unwrap()) as usize;
            let total_len = 9 + payload_len;
            if buffer.len() < total_len {
                break;
            }
            let frame_data = buffer.split_to(total_len).freeze();
            log::info!("Received frame: {} bytes", frame_data.len());
            if let Err(e) = handle_frame(&state, &frame_data).await {
                log::error!("Error handling frame: {}", e);
            }
        }
    }
    
    log::info!("Stdin channel closed, exiting");
    Ok(())
}

async fn handle_frame(state: &Arc<AppState>, data: &Bytes) -> Result<(), String> {
    log::info!("Decoding frame, {} bytes", data.len());
    let msg = Message::decode(data)?;
    log::info!("Decoded: topic={:?}, msg_id={}, msg_type={:?}", msg.topic, msg.msg_id, msg.msg_type);

    let response = match msg.topic {
        Topic::Handshake => handle_handshake(state, &msg).await?,
        Topic::InitializeContext => handle_initialize_context(state, &msg).await?,
        Topic::DestroyContext => handle_destroy_context(state, &msg).await?,
        Topic::GetState => handle_get_state(state, &msg).await?,
        Topic::EnvironmentCommand => handle_environment_command(state, &msg).await?,
        Topic::AuthCommand => handle_auth_command(state, &msg).await?,
        Topic::SyncCommand => handle_sync_command(state, &msg).await?,
        Topic::Shutdown => handle_shutdown(state, &msg).await?,
        _ => {
            log::warn!("Unhandled topic: {:?}", msg.topic);
            protocol::Message::response(
                msg.msg_id,
                msg.topic,
                ErrorCode::NotImplemented,
                encode_payload(&ErrorResponse { message: "Not implemented".into() })?
            )
        }
    };

    let response_data = response.encode()?;
    log::info!("Writing response: {} bytes", response_data.len());
    {
        use std::io::Write;
        let mut stdout = std::io::stdout();
        stdout.write_all(&response_data).map_err(|e| e.to_string())?;
        stdout.flush().map_err(|e| e.to_string())?;
    }
    log::info!("Response written successfully");

    Ok(())
}

async fn handle_handshake(state: &Arc<AppState>, msg: &Message) -> Result<Message, String> {
    let request: HandshakeRequest = decode_payload(&msg.data)?;
    
    if request.protocol_version != PROTOCOL_VERSION {
        return Err(format!("Unsupported protocol version: {}", request.protocol_version));
    }

    let response = HandshakeResponse {
        protocol_version: PROTOCOL_VERSION,
        runtime_version: state.runtime_version.clone(),
        runtime_id: state.runtime_id.clone(),
    };

    {
        let mut s = state.state.write();
        s.phase = protocol::RuntimePhase::Ready;
    }

    log::info!("Handshake successful: {}", state.runtime_id);

    Ok(protocol::Message::response(
        msg.msg_id,
        Topic::Handshake,
        ErrorCode::Success,
        encode_payload(&response)?
    ))
}

async fn handle_initialize_context(state: &Arc<AppState>, msg: &Message) -> Result<Message, String> {
    let _request: InitializeContextRequest = decode_payload(&msg.data)?;
    let context_id = Uuid::new_v4().to_string();

    {
        let mut s = state.state.write();
        s.phase = protocol::RuntimePhase::Ready;
        s.context_id = Some(context_id.clone());
    }

    log::info!("Context initialized: {}", context_id);

    Ok(protocol::Message::response(
        msg.msg_id,
        Topic::InitializeContext,
        ErrorCode::Success,
        encode_payload(&ContextResponse {
            context_id,
            phase: "ready".to_string(),
        })?
    ))
}

async fn handle_destroy_context(state: &Arc<AppState>, msg: &Message) -> Result<Message, String> {
    {
        let mut s = state.state.write();
        s.phase = protocol::RuntimePhase::Destroying;
        s.context_id = None;
        s.auth_info = None;
        s.environments.clear();
    }

    Ok(protocol::Message::response(
        msg.msg_id,
        Topic::DestroyContext,
        ErrorCode::Success,
        encode_payload(&EmptyPayload {})?
    ))
}

async fn handle_get_state(state: &Arc<AppState>, msg: &Message) -> Result<Message, String> {
    let snapshot = state.get_state_snapshot();
    let response = StateResponse { state: snapshot };

    Ok(protocol::Message::response(
        msg.msg_id,
        Topic::GetState,
        ErrorCode::Success,
        encode_payload(&response)?
    ))
}

async fn handle_environment_command(state: &Arc<AppState>, msg: &Message) -> Result<Message, String> {
    let request: EnvironmentCommandRequest = decode_payload(&msg.data)?;
    let result: EnvironmentCommandResponse;

    match request {
        EnvironmentCommandRequest::StartEnvironment { request: env_req } => {
            result = match start_environment(state, &env_req).await {
                Ok(endpoint) => EnvironmentCommandResponse::Started { endpoint },
                Err(e) => {
                    log::error!("Failed to start environment: {}", e);
                    EnvironmentCommandResponse::Ack
                }
            };
        }
        EnvironmentCommandRequest::StopEnvironment { env_uuid } => {
            {
                let mut s = state.state.write();
                s.environments.remove(&env_uuid);
            }
            result = EnvironmentCommandResponse::Ack;
        }
        EnvironmentCommandRequest::GetConnectedEnvironments => {
            let env_ids: Vec<String> = state.state.read().environments.keys().cloned().collect();
            result = EnvironmentCommandResponse::ConnectedEnvironments { env_ids };
        }
        EnvironmentCommandRequest::GetCdpEndpoint { env_uuid } => {
            let endpoint = state.state.read().environments.get(&env_uuid).map(|e| {
                CdpEndpointResponse {
                    env_uuid: env_uuid.clone(),
                    host: "127.0.0.1".to_string(),
                    port: e.cdp_port,
                    version_url: format!("http://127.0.0.1:{}/json/version", e.cdp_port),
                    list_url: format!("http://127.0.0.1:{}/json/list", e.cdp_port),
                    browser_ws_url: None,
                }
            });
            result = EnvironmentCommandResponse::CdpEndpoint { endpoint };
        }
        _ => {
            result = EnvironmentCommandResponse::Ack;
        }
    }

    let response = EnvironmentResponse { result };

    Ok(protocol::Message::response(
        msg.msg_id,
        Topic::EnvironmentCommand,
        ErrorCode::Success,
        encode_payload(&response)?
    ))
}

async fn start_environment(state: &Arc<AppState>, req: &EnvironmentStartRequest) -> Result<CdpEndpointResponse, String> {
    log::info!("start_environment called: env_uuid={}, exe_path={:?}", req.env_uuid, req.exe_path);
    let fingerprint = state.fingerprint_generator.generate(
        req.fingerprint_config.as_ref().unwrap_or(&FingerprintConfig::default())
    );
    log::info!("fingerprint generated: ua={}", fingerprint.user_agent);

    let browser = state.browser_manager.lock().await;
    log::info!("browser_manager locked, launching chromium...");
    let launch_result = browser.launch_chromium(
        &req.env_uuid,
        &req.user_data_dir,
        req.proxy.as_ref(),
        &fingerprint,
        req.extension_dirs.as_ref(),
        req.exe_path.as_deref(),
    ).await;
    
    match launch_result {
        Ok((_child, cdp_port)) => {
            log::info!("chromium launched successfully, cdp_port={}", cdp_port);
            let endpoint = CdpEndpointResponse {
                env_uuid: req.env_uuid.clone(),
                host: "127.0.0.1".to_string(),
                port: cdp_port,
                version_url: format!("http://127.0.0.1:{}/json/version", cdp_port),
                list_url: format!("http://127.0.0.1:{}/json/list", cdp_port),
                browser_ws_url: None,
            };
            {
                let mut s = state.state.write();
                s.environments.insert(req.env_uuid.clone(), EnvInfo {
                    _name: req.env_uuid.clone(),
                    _status: "running".into(),
                    cdp_port,
                });
            }
            Ok(endpoint)
        }
        Err(e) => {
            log::error!("chromium launch FAILED: {}", e);
            Err(e.to_string())
        }
    }
}

async fn handle_auth_command(state: &Arc<AppState>, msg: &Message) -> Result<Message, String> {
    let request: AuthCommandRequest = decode_payload(&msg.data)?;
    let result: AuthCommandResponse;

    match request {
        AuthCommandRequest::SetAuthState { auth_info } => {
            {
                let mut s = state.state.write();
                s.auth_info = Some(auth_info.clone());
            }
            result = AuthCommandResponse::Ack;
        }
        AuthCommandRequest::GetAuthState => {
            let auth_info = state.state.read().auth_info.clone().unwrap_or(AuthInfo {
                is_authenticated: false,
                access_token: None,
                user_info: None,
            });
            result = AuthCommandResponse::State { auth_info };
        }
        AuthCommandRequest::ClearAuthState => {
            let mut s = state.state.write();
            s.auth_info = None;
            result = AuthCommandResponse::Ack;
        }
    }

    let response = AuthResponse { result };

    Ok(protocol::Message::response(
        msg.msg_id,
        Topic::AuthCommand,
        ErrorCode::Success,
        encode_payload(&response)?
    ))
}

async fn handle_sync_command(state: &Arc<AppState>, msg: &Message) -> Result<Message, String> {
    let _request: SyncCommandRequest = decode_payload(&msg.data)?;
    
    let response = SyncResponse {
        result: SyncCommandResponse::RunningEnvironments {
            environments: vec![]
        }
    };

    Ok(protocol::Message::response(
        msg.msg_id,
        Topic::SyncCommand,
        ErrorCode::Success,
        encode_payload(&response)?
    ))
}

async fn handle_shutdown(state: &Arc<AppState>, msg: &Message) -> Result<Message, String> {
    {
        let mut s = state.state.write();
        s.phase = protocol::RuntimePhase::ShuttingDown;
    }

    std::process::exit(0);

    Ok(protocol::Message::response(
        msg.msg_id,
        Topic::Shutdown,
        ErrorCode::Success,
        encode_payload(&EmptyPayload {})?
    ))
}
