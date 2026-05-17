//! 指纹生成器
//! 
//! 为每个环境生成唯一的浏览器指纹，模拟真实用户特征。

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::protocol::FingerprintConfig;

#[derive(Debug, Clone)]
pub struct Fingerprint {
    pub language: String,
    pub timezone: String,
    pub platform: String,
    pub user_agent: String,
    pub window_width: u32,
    pub window_height: u32,
    pub canvas: String,
    pub webgl_vendor: String,
    pub webgl_renderer: String,
    pub audio_context: String,
    pub webrtc: String,
}

impl Default for Fingerprint {
    fn default() -> Self {
        let mut rng = rand::rng();
        
        Self {
            language: "zh-CN".to_string(),
            timezone: "Asia/Shanghai".to_string(),
            platform: "Linux x86_64".to_string(),
            user_agent: FingerprintGenerator::random_user_agent_static(&mut rng),
            window_width: rng.random_range(1920..=2560),
            window_height: rng.random_range(1080..=1440),
            canvas: "2d".to_string(),
            webgl_vendor: "Intel Inc.".to_string(),
            webgl_renderer: "Intel Iris OpenGL Engine".to_string(),
            audio_context: "default".to_string(),
            webrtc: "default".to_string(),
        }
    }
}

pub struct FingerprintGenerator;

impl FingerprintGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, config: &FingerprintConfig) -> Fingerprint {
        let mut rng = rand::rng();
        
        Fingerprint {
            language: config.language.clone().unwrap_or_else(|| "zh-CN".to_string()),
            timezone: config.timezone.clone().unwrap_or_else(|| "Asia/Shanghai".to_string()),
            platform: config.platform.clone().unwrap_or_else(|| "Linux x86_64".to_string()),
            user_agent: config.user_agent.clone().unwrap_or_else(|| Self::random_user_agent_static(&mut rng)),
            window_width: config.window_width.unwrap_or(rng.random_range(1920..=2560)) as u32,
            window_height: config.window_height.unwrap_or(rng.random_range(1080..=1440)) as u32,
            canvas: config.canvas.clone().unwrap_or_else(|| "2d".to_string()),
            webgl_vendor: config.webgl_vendor.clone().unwrap_or_else(|| "Intel Inc.".to_string()),
            webgl_renderer: config.webgl_renderer.clone().unwrap_or_else(|| "Intel Iris OpenGL Engine".to_string()),
            audio_context: config.audio_context.clone().unwrap_or_else(|| "default".to_string()),
            webrtc: config.webrtc.clone().unwrap_or_else(|| "default".to_string()),
        }
    }

    pub fn random_user_agent_static(rng: &mut impl Rng) -> String {
        let chrome_versions = [
            "120.0.6099.109", "121.0.6167.159", "122.0.6261.111",
            "123.0.6312.88", "124.0.6362.78", "125.0.6392.80",
        ];
        
        let chrome = chrome_versions[rng.random_range(0..chrome_versions.len())];
        
        format!(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{chrome} Safari/537.36",
        )
    }
}

impl Default for FingerprintGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub extension_id: String,
    pub name: String,
    pub version: String,
}
