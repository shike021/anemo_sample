use crate::message::{ChatMessage, ChatRequest, ChatResponse};
use anemo::PeerId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

/// 聊天用户信息
#[derive(Debug, Clone)]
pub struct ChatUser {
    pub peer_id: PeerId,
    pub username: String,
    pub joined_at: u64,
}

/// 聊天服务 - 管理聊天室的业务逻辑
#[derive(Clone)]
pub struct ChatService {
    /// 已连接的用户 (PeerId -> ChatUser)
    users: Arc<RwLock<HashMap<PeerId, ChatUser>>>,
    /// 用户名到PeerId的映射，防重名
    username_to_peer: Arc<RwLock<HashMap<String, PeerId>>>,
    /// 消息历史（最近100条）
    message_history: Arc<RwLock<Vec<ChatMessage>>>,
    /// 消息广播通道
    broadcast_tx: Arc<RwLock<Option<mpsc::UnboundedSender<(ChatMessage, Option<PeerId>)>>>>,
}

impl ChatService {
    const MAX_HISTORY: usize = 100;

    /// 创建新的聊天服务
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            username_to_peer: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(Vec::new())),
            broadcast_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// 设置消息广播通道
    pub async fn set_broadcast_channel(
        &self,
        tx: mpsc::UnboundedSender<(ChatMessage, Option<PeerId>)>,
    ) {
        let mut broadcast_tx = self.broadcast_tx.write().await;
        *broadcast_tx = Some(tx);
    }

    /// 用户加入聊天室
    pub async fn user_join(
        &self,
        peer_id: PeerId,
        username: String,
    ) -> Result<ChatResponse, String> {
        // 检查用户名是否已被使用
        let username_to_peer = self.username_to_peer.read().await;
        if username_to_peer.contains_key(&username) {
            return Ok(ChatResponse::error(format!(
                "用户名 '{}' 已被使用",
                username
            )));
        }
        drop(username_to_peer);

        // 添加用户
        let user = ChatUser {
            peer_id,
            username: username.clone(),
            joined_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let mut users = self.users.write().await;
        let mut username_to_peer = self.username_to_peer.write().await;

        users.insert(peer_id, user);
        username_to_peer.insert(username.clone(), peer_id);

        drop(users);
        drop(username_to_peer);

        info!("用户 {} 加入聊天室，PeerId: {}", username, peer_id);

        // 发送加入消息
        let join_message = ChatMessage::new_user_joined(username);
        self.add_to_history(join_message.clone()).await;
        self.broadcast_message(join_message, Some(peer_id)).await;

        Ok(ChatResponse::success())
    }

    /// 用户离开聊天室
    pub async fn user_leave(&self, peer_id: PeerId) {
        let mut users = self.users.write().await;
        let mut username_to_peer = self.username_to_peer.write().await;

        if let Some(user) = users.remove(&peer_id) {
            username_to_peer.remove(&user.username);
            info!("用户 {} 离开聊天室，PeerId: {}", user.username, peer_id);

            // 发送离开消息
            let leave_message = ChatMessage::new_user_left(user.username);
            drop(users);
            drop(username_to_peer);

            self.add_to_history(leave_message.clone()).await;
            self.broadcast_message(leave_message, Some(peer_id)).await;
        }
    }

    /// 处理聊天消息
    pub async fn handle_message(&self, peer_id: PeerId, request: ChatRequest) -> ChatResponse {
        let users = self.users.read().await;
        let user = match users.get(&peer_id) {
            Some(user) => user.clone(),
            None => {
                warn!("收到来自未注册用户的消息: {}", peer_id);
                return ChatResponse::error("用户未注册".to_string());
            }
        };
        drop(users);

        match request.message {
            ChatMessage::Text {
                sender, content, ..
            } => {
                // 验证发送者
                if sender != user.username {
                    warn!("用户 {} 尝试伪造发送者: {}", user.username, sender);
                    return ChatResponse::error("发送者验证失败".to_string());
                }

                // 创建新消息（重新生成时间戳）
                let message = ChatMessage::new_text(sender, content);
                info!("收到消息: {}", message.format_for_display());

                // 添加到历史记录并广播
                self.add_to_history(message.clone()).await;
                self.broadcast_message(message, Some(peer_id)).await;

                ChatResponse::success()
            }
            ChatMessage::UserJoined { username, .. } => {
                // 处理用户加入
                match self.user_join(peer_id, username).await {
                    Ok(response) => response,
                    Err(err) => {
                        error!("用户加入失败: {}", err);
                        ChatResponse::error(err)
                    }
                }
            }
            ChatMessage::Heartbeat { .. } => {
                // 心跳消息，简单返回成功
                ChatResponse::success()
            }
            _ => {
                warn!("收到不支持的消息类型");
                ChatResponse::error("不支持的消息类型".to_string())
            }
        }
    }

    /// 获取在线用户列表
    pub async fn get_online_users(&self) -> Vec<String> {
        let users = self.users.read().await;
        users.values().map(|user| user.username.clone()).collect()
    }

    /// 获取消息历史
    pub async fn get_message_history(&self) -> Vec<ChatMessage> {
        let history = self.message_history.read().await;
        history.clone()
    }

    /// 获取用户数量
    pub async fn get_user_count(&self) -> usize {
        let users = self.users.read().await;
        users.len()
    }

    /// 添加消息到历史记录
    async fn add_to_history(&self, message: ChatMessage) {
        let mut history = self.message_history.write().await;
        history.push(message);

        // 保持历史记录不超过最大数量
        if history.len() > Self::MAX_HISTORY {
            history.remove(0);
        }
    }

    /// 广播消息给所有用户（除了排除的用户）
    async fn broadcast_message(&self, message: ChatMessage, exclude_peer: Option<PeerId>) {
        let broadcast_tx = self.broadcast_tx.read().await;
        if let Some(tx) = broadcast_tx.as_ref() {
            if let Err(_) = tx.send((message, exclude_peer)) {
                error!("消息广播通道已关闭");
            }
        }
    }
}

impl Default for ChatService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anemo::PeerId;

    fn mock_peer_id() -> PeerId {
        // 创建一个模拟的PeerId用于测试
        // 在实际测试中，你可能需要使用anemo提供的测试工具
        unsafe { std::mem::zeroed() }
    }

    #[tokio::test]
    async fn test_chat_service() {
        let service = ChatService::new();
        let peer_id = mock_peer_id();

        // 测试用户加入
        let result = service.user_join(peer_id, "Alice".to_string()).await;
        assert!(result.is_ok());

        // 测试获取用户数量
        let count = service.get_user_count().await;
        assert_eq!(count, 1);

        // 测试获取在线用户
        let users = service.get_online_users().await;
        assert_eq!(users, vec!["Alice"]);
    }
}
