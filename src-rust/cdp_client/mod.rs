//! CDP 客户端模块
//! 
//! 用于与 Chromium DevTools Protocol 通信，实现浏览器自动化控制。

pub mod client;

pub use client::CdpClient;

pub mod types;
