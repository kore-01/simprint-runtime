//! Simprint Runtime IPC 协议
//! 
//! 复制 src-tauri/src/infrastructure/runtime/ 中的协议定义，
//! 确保 Linux runtime 与 Tauri app 完全兼容。

pub mod topics;
pub mod api;

pub use topics::Topic;
pub use api::*;

use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::sync::atomic::{AtomicU32, Ordering};

const HEADER_SIZE: usize = 9;
const MAGIC: [u8; 4] = [0x73, 0x70, 0x72, 0x74]; // "sprt"
pub const PROTOCOL_VERSION: u8 = 3;

static MESSAGE_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

fn next_message_id() -> u32 {
    MESSAGE_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Request = 1,
    Response = 2,
    Event = 3,
}

impl TryFrom<u8> for MessageType {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Request),
            2 => Ok(Self::Response),
            3 => Ok(Self::Event),
            other => Err(format!("unknown message type: {}", other)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub msg_id: u32,
    pub msg_type: MessageType,
    pub topic: Topic,
    pub error_code: i32,
    pub data: Vec<u8>,
}

impl Message {
    pub fn request(topic: Topic, data: Vec<u8>) -> Self {
        Self {
            msg_id: next_message_id(),
            msg_type: MessageType::Request,
            topic,
            error_code: 0,
            data,
        }
    }

    pub fn response(request_id: u32, topic: Topic, error_code: ErrorCode, data: Vec<u8>) -> Self {
        Self {
            msg_id: request_id,
            msg_type: MessageType::Response,
            topic,
            error_code: error_code.as_i32(),
            data,
        }
    }

    pub fn encode(&self) -> Result<Bytes, String> {
        let payload_len = HEADER_SIZE + self.data.len();
        let mut payload = BytesMut::with_capacity(payload_len);

        payload.put_u32_le(self.msg_id);
        payload.put_u8(self.msg_type as u8);
        payload.put_u16_le(u16::from(self.topic));
        payload.put_i32_le(self.error_code);
        payload.put_u32_le(self.data.len() as u32);
        payload.put_slice(&self.data);

        let mut frame = BytesMut::with_capacity(HEADER_SIZE + payload.len());
        frame.put_slice(&MAGIC);
        frame.put_u8(PROTOCOL_VERSION);
        frame.put_u32_le(payload.len() as u32);
        frame.put_slice(&payload);

        Ok(frame.freeze())
    }

    pub fn decode(data: &[u8]) -> Result<Self, String> {
        if data.len() < HEADER_SIZE {
            return Err("frame shorter than header".into());
        }

        let mut buffer = data;

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&buffer[..4]);
        buffer.advance(4);
        if magic != MAGIC {
            return Err("invalid magic number".into());
        }

        let version = buffer.get_u8();
        if version != PROTOCOL_VERSION {
            return Err(format!("unsupported protocol version: {}", version));
        }

        let payload_len = buffer.get_u32_le() as usize;
        if buffer.len() < payload_len {
            return Err("incomplete payload".into());
        }
        if payload_len < HEADER_SIZE {
            return Err("payload too short".into());
        }

        let msg_id = buffer.get_u32_le();
        let msg_type = MessageType::try_from(buffer.get_u8())?;
        let topic = Topic::from(buffer.get_u16_le());
        let error_code = buffer.get_i32_le();
        let data_len = buffer.get_u32_le() as usize;
        if buffer.len() < data_len {
            return Err("data length mismatch".into());
        }

        let mut msg_data = vec![0u8; data_len];
        msg_data.copy_from_slice(&buffer[..data_len]);

        Ok(Self {
            msg_id,
            msg_type,
            topic,
            error_code,
            data: msg_data,
        })
    }
}

pub fn encode_payload<T: Serialize>(payload: &T) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(payload)
        .map_err(|e| e.to_string())
}

pub fn decode_payload<T: DeserializeOwned>(data: &[u8]) -> Result<T, String> {
    rmp_serde::from_slice(data).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Success = 0,
    InvalidPayload = 1,
    NotImplemented = 2,
    InternalError = 3,
}

impl ErrorCode {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}
