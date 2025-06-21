use crate::chat_service::ChatService;
use crate::message::{ChatMessage, ChatRequest};
use anemo::{Network, PeerId, Request};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// 聊天网络服务
#[derive(Clone)]
pub struct ChatNetworkService {
    chat_service: ChatService,
    network: Arc<Network>,
}

impl ChatNetworkService {
    pub fn new(chat_service: ChatService, network: Arc<Network>) -> Self {
        Self {
            chat_service,
            network,
        }
    }

    /// 启动消息广播处理器
    pub async fn start_broadcast_handler(&self) {
        let (tx, mut rx) = mpsc::unbounded_channel::<(ChatMessage, Option<PeerId>)>();

        // 设置聊天服务的广播通道
        self.chat_service.set_broadcast_channel(tx).await;

        let network = self.network.clone();
        let chat_service = self.chat_service.clone();

        tokio::spawn(async move {
            while let Some((message, exclude_peer)) = rx.recv().await {
                Self::broadcast_to_peers(&network, &chat_service, message, exclude_peer).await;
            }
        });
    }

    /// 广播消息给所有连接的节点
    async fn broadcast_to_peers(
        network: &Network,
        chat_service: &ChatService,
        message: ChatMessage,
        exclude_peer: Option<PeerId>,
    ) {
        let peers = network.peers();
        let message_display = message.format_for_display();

        info!("广播消息给 {} 个节点: {}", peers.len(), message_display);

        for peer_id in peers {
            // 跳过被排除的节点
            if let Some(exclude) = exclude_peer {
                if peer_id == exclude {
                    continue;
                }
            }

            // 序列化消息为字节
            let chat_request = ChatRequest {
                message: message.clone(),
            };
            match serde_json::to_vec(&chat_request) {
                Ok(request_bytes) => {
                    let request = Request::new(request_bytes.into());

                    // 异步发送，不等待响应
                    let network_clone = network.clone();
                    tokio::spawn(async move {
                        match network_clone.rpc(peer_id, request).await {
                            Ok(_) => {
                                // 成功发送
                            }
                            Err(e) => {
                                warn!("向节点 {} 发送消息失败: {}", peer_id, e);
                            }
                        }
                    });
                }
                Err(e) => {
                    warn!("序列化消息失败: {}", e);
                }
            }
        }
    }

    /// 处理连接断开事件
    pub async fn handle_peer_disconnected(&self, peer_id: PeerId) {
        info!("节点断开连接: {}", peer_id);
        self.chat_service.user_leave(peer_id).await;
    }
}

/// 实现anemo的RPC服务trait
/// 注意：这里的实现需要根据anemo的具体版本和API调整
/// 目前由于anemo API兼容性问题，暂时注释掉复杂的RPC实现
///
/// #[async_trait]
/// pub trait ChatRpcService {
///     async fn send_message(
///         &self,
///         request: Request<ChatRequest>,
///     ) -> Result<Response<ChatResponse>, Status>;
/// }
///
/// #[async_trait]
/// impl ChatRpcService for ChatNetworkService {
///     async fn send_message(
///         &self,
///         request: Request<ChatRequest>,
///     ) -> Result<Response<ChatResponse>, Status> {
///         let peer_id = request.peer_id().unwrap_or_else(|| {
///             error!("请求中没有PeerId信息");
///             // 返回一个默认的PeerId，实际应用中应处理这种情况
///             // PeerId::random()  // 这个方法不存在
///         });
///         
///         let chat_request = request.into_inner();
///         
///         info!("收到来自 {} 的RPC请求: {:?}", peer_id, chat_request);
///         
///         let response = self.chat_service.handle_message(peer_id, chat_request).await;
///         
///         Ok(Response::new(response))
///     }
/// }

/// 网络事件处理器
pub struct NetworkEventHandler {
    chat_service: ChatService,
}

impl NetworkEventHandler {
    pub fn new(chat_service: ChatService) -> Self {
        Self { chat_service }
    }

    /// 处理新节点连接
    pub async fn handle_peer_connected(&self, peer_id: PeerId) {
        info!("新节点连接: {}", peer_id);
        // 新连接暂时不自动加入聊天室，等待用户发送UserJoined消息
    }

    /// 处理节点断开
    pub async fn handle_peer_disconnected(&self, peer_id: PeerId) {
        info!("节点断开: {}", peer_id);
        self.chat_service.user_leave(peer_id).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_service::ChatService;

    #[tokio::test]
    async fn test_network_service_creation() {
        let chat_service = ChatService::new();
        // 在实际测试中，需要创建一个模拟的Network
        // let network = Arc::new(mock_network());
        // let service = ChatNetworkService::new(chat_service, network);
        // assert!(service.chat_service.get_user_count().await == 0);
    }
}
