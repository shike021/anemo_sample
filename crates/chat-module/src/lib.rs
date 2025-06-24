//! 聊天业务模块
//!
//! 基于网络服务层实现的聊天功能，包括：
//! - 用户加入/离开聊天室
//! - 发送/接收聊天消息
//! - 聊天室管理
//! - 消息历史记录

pub mod chat_service;
pub mod error;
pub mod message_handler;

pub use chat_service::{ChatRoom, ChatService, ChatUser};
pub use error::{ChatError, Result};
pub use message_handler::ChatMessageHandler;

use async_trait::async_trait;
use network_service::NodeId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// 聊天消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatMessageType {
    /// 用户加入聊天室
    UserJoin { username: String, room_id: String },
    /// 用户离开聊天室
    UserLeave { username: String, room_id: String },
    /// 文本消息
    TextMessage { room_id: String, content: String },
    /// 私聊消息
    PrivateMessage {
        target_user: String,
        content: String,
    },
    /// 聊天室列表请求
    ListRooms,
    /// 聊天室成员列表请求
    ListRoomMembers { room_id: String },
}

/// 聊天响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatResponseType {
    /// 操作成功
    Success { message: String },
    /// 操作失败
    Error { error: String },
    /// 聊天室列表
    RoomList { rooms: Vec<String> },
    /// 聊天室成员列表
    MemberList {
        room_id: String,
        members: Vec<String>,
    },
    /// 消息广播确认
    MessageBroadcast { message_id: Uuid },
}

/// 聊天服务trait
#[async_trait]
pub trait ChatServiceTrait: Send + Sync {
    /// 用户加入聊天室
    async fn join_room(&self, user_id: NodeId, username: String, room_id: String) -> Result<()>;

    /// 用户离开聊天室
    async fn leave_room(&self, user_id: NodeId, room_id: String) -> Result<()>;

    /// 发送聊天消息
    async fn send_message(&self, user_id: NodeId, room_id: String, content: String)
        -> Result<Uuid>;

    /// 发送私聊消息
    async fn send_private_message(
        &self,
        from_user: NodeId,
        to_user: String,
        content: String,
    ) -> Result<Uuid>;

    /// 获取聊天室列表
    async fn list_rooms(&self) -> Result<Vec<String>>;

    /// 获取聊天室成员列表
    async fn list_room_members(&self, room_id: String) -> Result<Vec<String>>;

    /// 获取用户所在的聊天室
    async fn get_user_rooms(&self, user_id: NodeId) -> Result<Vec<String>>;
}
