//! Topic 定义
//! 
//! 与 src-tauri/src/infrastructure/runtime/topics.rs 保持一致

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum Topic {
    Handshake = 0x0100,
    InitializeContext = 0x0101,
    DestroyContext = 0x0102,
    GetState = 0x0103,
    AuthCommand = 0x0200,
    SyncCommand = 0x0300,
    EnvironmentCommand = 0x0400,
    RuntimeEvent = 0x0500,
    Shutdown = 0xFFFF,
}

impl From<Topic> for u16 {
    fn from(t: Topic) -> Self {
        t as u16
    }
}

impl From<u16> for Topic {
    fn from(v: u16) -> Self {
        match v {
            0x0100 => Topic::Handshake,
            0x0101 => Topic::InitializeContext,
            0x0102 => Topic::DestroyContext,
            0x0103 => Topic::GetState,
            0x0200 => Topic::AuthCommand,
            0x0300 => Topic::SyncCommand,
            0x0400 => Topic::EnvironmentCommand,
            0x0500 => Topic::RuntimeEvent,
            0xFFFF => Topic::Shutdown,
            _ => Topic::Shutdown,
        }
    }
}
