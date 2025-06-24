//! 网络服务抽象层
//!
//! 提供统一的网络服务接口，使业务模块可以方便地使用网络功能，
//! 同时保持与具体网络实现的解耦。

pub mod anemo_impl;
pub mod error;
pub mod event_bus;
pub mod message;
pub mod service;

// 重新导出主要接口
pub use anemo_impl::AnemoNetworkService;
pub use error::{NetworkError, Result};
pub use event_bus::{EventBus, EventHandler, NetworkEvent};
pub use message::{BroadcastOptions, MessageType, NetworkMessage, UnicastOptions};
pub use service::{NetworkService, NetworkServiceConfig};

use async_trait::async_trait;
use uuid::Uuid;

/// 网络节点ID类型
pub type NodeId = String;

/// 消息ID类型  
pub type MessageId = Uuid;

/// 网络服务的核心trait，定义所有网络操作接口
#[async_trait]
pub trait NetworkServiceTrait: Send + Sync + Clone {
    /// 启动网络服务
    async fn start(&self, config: NetworkServiceConfig) -> Result<()>;

    /// 停止网络服务
    async fn stop(&self) -> Result<()>;

    /// 广播消息给所有连接的节点
    async fn broadcast(
        &self,
        message: NetworkMessage,
        options: Option<BroadcastOptions>,
    ) -> Result<MessageId>;

    /// 单播消息给指定节点
    async fn unicast(
        &self,
        target: NodeId,
        message: NetworkMessage,
        options: Option<UnicastOptions>,
    ) -> Result<MessageId>;

    /// 获取当前连接的节点列表
    async fn get_connected_nodes(&self) -> Result<Vec<NodeId>>;

    /// 获取本地节点ID
    async fn get_local_node_id(&self) -> Result<NodeId>;

    /// 注册消息处理器
    async fn register_message_handler(
        &self,
        message_type: MessageType,
        handler: Box<dyn MessageHandler>,
    ) -> Result<()>;

    /// 注册事件处理器
    async fn register_event_handler(&self, handler: Box<dyn EventHandler>) -> Result<()>;
}

/// 消息处理器trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// 处理接收到的消息
    async fn handle_message(
        &self,
        from: NodeId,
        message: NetworkMessage,
    ) -> Result<Option<NetworkMessage>>;
}

/// 服务健康状态
#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub is_running: bool,
    pub connected_nodes: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub last_activity: Option<std::time::SystemTime>,
}

/// 网络统计信息
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub connection_count: usize,
    pub error_count: u64,
}
