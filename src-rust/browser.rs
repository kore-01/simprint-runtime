//! 浏览器管理器 - Linux 实现
//! 
//! 使用 Chromium + CDP 进行浏览器自动化，
//! 支持指纹注入、代理切换、扩展加载等功能。

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;
use tokio::sync::Mutex;
use rand::Rng;

use crate::protocol::BrowserProxyConfigPayload;
use crate::fingerprint::Fingerprint;

pub struct BrowserManager {
    chrome_path: Option<PathBuf>,
    environments: HashMap<String, ChromeInstance>,
}

struct ChromeInstance {
    _pid: u32,
    cdp_port: u16,
    _user_data_dir: PathBuf,
}

impl BrowserManager {
    pub fn new() -> Self {
        let chrome_path = Self::find_chrome();
        
        Self {
            chrome_path,
            environments: HashMap::new(),
        }
    }

    fn find_chrome() -> Option<PathBuf> {
        // 按优先级查找 Chrome
        let paths = [
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
            "/snap/bin/chromium",
        ];
        
        for path in &paths {
            if std::path::Path::new(path).exists() {
                return Some(PathBuf::from(path));
            }
        }
        
        // 尝试 which
        if let Ok(path) = which::which("google-chrome") {
            return Some(path);
        }
        if let Ok(path) = which::which("chromium-browser") {
            return Some(path);
        }
        
        None
    }

    pub async fn launch_chromium(
        &self,
        env_uuid: &str,
        user_data_dir: &str,
        proxy: Option<&BrowserProxyConfigPayload>,
        fingerprint: &Fingerprint,
        _extension_dirs: Option<&Vec<String>>,
        exe_path: Option<&str>,  // 新增：允许指定 Chrome 路径
    ) -> Result<(tokio::process::Child, u16), Box<dyn std::error::Error + Send + Sync>> {
        // 添加扩展目录参数
        let extension_args: Vec<String> = if let Some(dirs) = _extension_dirs {
            dirs.iter()
                .filter(|d| std::path::Path::new(d).exists())
                .map(|d| format!("--load-extension={}", d))
                .collect()
        } else {
            vec![]
        };
        
        // 优先使用请求中的 exe_path，否则使用自动检测的路径
        let chrome_path = if let Some(path) = exe_path {
            let p = PathBuf::from(path);
            if p.exists() {
                p
            } else {
                return Err(format!("Specified Chrome path not found: {}", path).into());
            }
        } else {
            self.chrome_path
                .clone()
                .ok_or("Chrome not found. Please install chromium-browser or google-chrome")?
        };
        
        // 分配 CDP 端口 - 使用更广范围避免冲突
        let cdp_port: u16 = rand::random::<u16>() % 1000 + 9200;
        
        // 构建用户数据目录
        let user_data_path = PathBuf::from(user_data_dir);
        std::fs::create_dir_all(&user_data_path)?;
        
        // 构建 Chrome 启动参数
        let mut args = vec![
            // 基础参数
            format!("--user-data-dir={}", user_data_path.display()),
            format!("--remote-debugging-port={}", cdp_port),
            
            // 指纹伪装参数
            format!("--window-size={},{}", fingerprint.window_width, fingerprint.window_height),
            format!("--lang={}", fingerprint.language),
            
            // 服务器模式（无头）
            "--headless=new".to_string(),
            "--no-sandbox".to_string(),
            "--disable-setuid-sandbox".to_string(),
            "--disable-dev-shm-usage".to_string(),
            
            // 防检测
            "--disable-blink-features=AutomationControlled".to_string(),
            "--disable-infobars".to_string(),
            "--mute-audio".to_string(),
            
// CDP WebSocket
            "--remote-debugging-address=127.0.0.1".to_string(),
            "--remote-allow-origins=*".to_string(),
        ];

        // 添加代理
        if let Some(proxy_config) = proxy {
            if let (Some(server), Some(port)) = (&proxy_config.server, proxy_config.port) {
                let proxy_url = format!("{}://{}:{}", 
                    proxy_config.proxy_type.as_deref().unwrap_or("http"),
                    server,
                    port
                );
                args.push(format!("--proxy-server={}", proxy_url));
            }
        }
        
        // User-Agent
        args.push(format!("--user-agent={}", fingerprint.user_agent));
        
        // Timezone
        args.push(format!("--timezone={}", fingerprint.timezone));
        
        // Platform
        args.push(format!("--platform={}", fingerprint.platform));
        
        log::info!("Launching Chrome: {:?} with args: {:?}", chrome_path, args);
        
        let mut command = Command::new(&chrome_path);
        command
            .args(&args)
            .args(&extension_args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null());
        
        // 设置环境变量
        command.env("DISPLAY", std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()));
        
        let child = command.spawn()
            .map_err(|e| format!("Failed to spawn Chrome: {}. Is Chrome installed?", e))?;
        
        let pid = child.id().unwrap_or(0);
        
        log::info!("Chrome started with PID {} on CDP port {}", pid, cdp_port);
        
        // 等待 CDP 就绪
        Self::wait_for_cdp(cdp_port).await?;
        
        Ok((child, cdp_port))
    }

    async fn wait_for_cdp(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("http://127.0.0.1:{}/json/version", port);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        
        for _ in 0..30 {
            if client.get(&url).send().await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        
        Err("CDP did not become ready in time".into())
    }
}

impl Default for BrowserManager {
    fn default() -> Self {
        Self::new()
    }
}
