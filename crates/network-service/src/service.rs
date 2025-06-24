//! 网络服务核心实现

use crate::MessageHandler;
use crate::{EventBus, MessageType, NetworkMessage, NodeId, Result};
use rand::RngCore;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 网络服务配置
#[derive(Debug, Clone)]
pub struct NetworkServiceConfig {
    /// 监听地址
    pub bind_address: SocketAddr,
    /// 服务器名称
    pub server_name: String,
    /// 私钥（用于TLS）
    pub private_key: [u8; 32],
    /// 最大连接数
    pub max_connections: usize,
    /// 心跳间隔（毫秒）
    pub heartbeat_interval_ms: u64,
    /// 消息缓冲区大小
    pub message_buffer_size: usize,
    /// 事件总线容量
    pub event_bus_capacity: usize,
}

impl Default for NetworkServiceConfig {
    fn default() -> Self {
        // 生成随机私钥
        let mut private_key = [0u8; 32];
        rand::rng().fill_bytes(&mut private_key);

        Self {
            bind_address: "127.0.0.1:8080".parse().unwrap(),
            server_name: "anemo-network-service".to_string(),
            private_key,
            max_connections: 1000,
            heartbeat_interval_ms: 30000,
            message_buffer_size: 1000,
            event_bus_capacity: 1000,
        }
    }
}

/// 网络服务主结构
#[derive(Clone)]
pub struct NetworkService {
    /// 事件总线
    event_bus: EventBus,
    /// 消息处理器注册表
    message_handlers: Arc<RwLock<HashMap<MessageType, Arc<dyn MessageHandler>>>>,
    /// 服务状态
    is_running: Arc<RwLock<bool>>,
    /// 配置
    config: Arc<RwLock<Option<NetworkServiceConfig>>>,
}

impl NetworkService {
    /// 创建新的网络服务
    pub fn new() -> Self {
        let event_bus = EventBus::new(1000);

        Self {
            event_bus,
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            config: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取事件总线
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// 检查服务是否正在运行
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// 获取配置
    pub async fn get_config(&self) -> Option<NetworkServiceConfig> {
        self.config.read().await.clone()
    }

    /// 设置配置
    pub async fn set_config(&self, config: NetworkServiceConfig) {
        *self.config.write().await = Some(config);
    }

    /// 注册消息处理器
    pub async fn register_message_handler_internal(
        &self,
        message_type: MessageType,
        handler: Arc<dyn MessageHandler>,
    ) -> Result<()> {
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(message_type, handler);
        Ok(())
    }

    /// 获取消息处理器
    pub async fn get_message_handler(
        &self,
        message_type: &MessageType,
    ) -> Option<Arc<dyn MessageHandler>> {
        let handlers = self.message_handlers.read().await;
        handlers.get(message_type).cloned()
    }

    /// 处理接收到的消息
    pub async fn handle_incoming_message(
        &self,
        from: NodeId,
        message: NetworkMessage,
    ) -> Result<()> {
        // 发布消息接收事件
        self.event_bus
            .publish(crate::event_bus::NetworkEvent::MessageReceived {
                from: from.clone(),
                message: message.clone(),
            })
            .await;

        // 查找消息处理器
        if let Some(handler) = self.get_message_handler(&message.message_type).await {
            // 异步处理消息
            let handler_clone = handler.clone();
            let from_clone = from.clone();
            let message_clone = message.clone();
            let event_bus = self.event_bus.clone();

            tokio::spawn(async move {
                match handler_clone
                    .handle_message(from_clone.clone(), message_clone)
                    .await
                {
                    Ok(response) => {
                        if let Some(response_msg) = response {
                            // 如果有响应消息，可以在这里处理发送逻辑
                            tracing::info!("消息处理器返回响应: {:?}", response_msg);
                        }
                    }
                    Err(e) => {
                        tracing::error!("消息处理器处理消息失败: {}", e);
                        event_bus
                            .publish(crate::event_bus::NetworkEvent::Error {
                                error: format!("处理来自 {} 的消息失败: {}", from_clone, e),
                            })
                            .await;
                    }
                }
            });
        } else {
            tracing::warn!("未找到消息类型 {:?} 的处理器", message.message_type);
        }

        Ok(())
    }

    /// 设置运行状态
    async fn set_running(&self, running: bool) {
        *self.is_running.write().await = running;
    }
}

impl Default for NetworkService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageType;
    use async_trait::async_trait;

    struct TestMessageHandler;

    #[async_trait]
    impl MessageHandler for TestMessageHandler {
        async fn handle_message(
            &self,
            _from: NodeId,
            _message: NetworkMessage,
        ) -> Result<Option<NetworkMessage>> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_network_service_creation() {
        let service = NetworkService::new();
        assert!(!service.is_running().await);
        assert!(service.get_config().await.is_none());
    }

    #[tokio::test]
    async fn test_message_handler_registration() {
        let service = NetworkService::new();
        let handler = Arc::new(TestMessageHandler);
        let message_type = MessageType::chat();

        service
            .register_message_handler_internal(message_type.clone(), handler)
            .await
            .unwrap();
        assert!(service.get_message_handler(&message_type).await.is_some());
    }
}
