//! 网络事件总线

use crate::{NetworkMessage, NodeId};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};

/// 网络事件类型
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// 节点连接事件
    NodeConnected {
        node_id: NodeId,
        metadata: HashMap<String, String>,
    },
    /// 节点断开事件
    NodeDisconnected { node_id: NodeId, reason: String },
    /// 消息接收事件
    MessageReceived {
        from: NodeId,
        message: NetworkMessage,
    },
    /// 消息发送成功事件
    MessageSent { to: NodeId, message_id: uuid::Uuid },
    /// 消息发送失败事件
    MessageSendFailed {
        to: NodeId,
        message_id: uuid::Uuid,
        error: String,
    },
    /// 服务启动事件
    ServiceStarted,
    /// 服务停止事件
    ServiceStopped,
    /// 错误事件
    Error { error: String },
}

/// 事件处理器trait
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// 处理网络事件
    async fn handle_event(&self, event: NetworkEvent);

    /// 获取处理器名称
    fn name(&self) -> &str;
}

/// 事件总线
#[derive(Clone)]
pub struct EventBus {
    /// 事件广播通道
    sender: broadcast::Sender<NetworkEvent>,
    /// 事件处理器注册表
    handlers: Arc<RwLock<HashMap<String, Arc<dyn EventHandler>>>>,
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);

        Self {
            sender,
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 发布事件
    pub async fn publish(&self, event: NetworkEvent) {
        info!("发布网络事件: {:?}", event);

        // 广播事件
        if let Err(e) = self.sender.send(event.clone()) {
            warn!("事件广播失败: {}", e);
        }

        // 调用注册的处理器
        let handlers = self.handlers.read().await;
        for (name, handler) in handlers.iter() {
            let handler = handler.clone();
            let event_clone = event.clone();
            let name_clone = name.clone();

            tokio::spawn(async move {
                if let Err(e) = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    handler.handle_event(event_clone),
                )
                .await
                {
                    error!("事件处理器 {} 处理超时: {}", name_clone, e);
                }
            });
        }
    }

    /// 注册事件处理器
    pub async fn register_handler(&self, handler: Arc<dyn EventHandler>) {
        let name = handler.name().to_string();
        info!("注册事件处理器: {}", name);

        let mut handlers = self.handlers.write().await;
        handlers.insert(name, handler);
    }

    /// 注销事件处理器
    pub async fn unregister_handler(&self, name: &str) {
        info!("注销事件处理器: {}", name);

        let mut handlers = self.handlers.write().await;
        handlers.remove(name);
    }

    /// 创建事件订阅者
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkEvent> {
        self.sender.subscribe()
    }

    /// 获取当前注册的处理器数量
    pub async fn handler_count(&self) -> usize {
        self.handlers.read().await.len()
    }
}

/// 默认日志事件处理器
pub struct LogEventHandler {
    name: String,
}

impl LogEventHandler {
    pub fn new() -> Self {
        Self {
            name: "log_handler".to_string(),
        }
    }
}

#[async_trait]
impl EventHandler for LogEventHandler {
    async fn handle_event(&self, event: NetworkEvent) {
        match event {
            NetworkEvent::NodeConnected { node_id, .. } => {
                info!("节点已连接: {}", node_id);
            }
            NetworkEvent::NodeDisconnected { node_id, reason } => {
                info!("节点已断开: {} (原因: {})", node_id, reason);
            }
            NetworkEvent::MessageReceived { from, message } => {
                info!("收到来自 {} 的消息: {:?}", from, message.message_type);
            }
            NetworkEvent::MessageSent { to, message_id } => {
                info!("成功发送消息 {} 到 {}", message_id, to);
            }
            NetworkEvent::MessageSendFailed {
                to,
                message_id,
                error,
            } => {
                warn!("发送消息 {} 到 {} 失败: {}", message_id, to, error);
            }
            NetworkEvent::ServiceStarted => {
                info!("网络服务已启动");
            }
            NetworkEvent::ServiceStopped => {
                info!("网络服务已停止");
            }
            NetworkEvent::Error { error } => {
                error!("网络服务错误: {}", error);
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus() {
        let event_bus = EventBus::new(100);
        let handler = Arc::new(LogEventHandler::new());

        event_bus.register_handler(handler).await;
        assert_eq!(event_bus.handler_count().await, 1);

        let event = NetworkEvent::ServiceStarted;
        event_bus.publish(event).await;

        // 等待处理完成
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
