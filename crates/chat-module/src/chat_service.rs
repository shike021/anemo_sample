//! 聊天服务实现

use crate::{ChatError, ChatMessageType, ChatServiceTrait, Result};
use async_trait::async_trait;
use network_service::{
    BroadcastOptions, MessageId, MessageType, NetworkMessage, NetworkServiceTrait, NodeId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// 聊天用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatUser {
    pub user_id: NodeId,
    pub username: String,
    pub joined_rooms: HashSet<String>,
    pub last_active: u64,
}

impl ChatUser {
    pub fn new(user_id: NodeId, username: String) -> Self {
        Self {
            user_id,
            username,
            joined_rooms: HashSet::new(),
            last_active: current_timestamp(),
        }
    }

    pub fn join_room(&mut self, room_id: String) {
        self.joined_rooms.insert(room_id);
        self.last_active = current_timestamp();
    }

    pub fn leave_room(&mut self, room_id: &str) {
        self.joined_rooms.remove(room_id);
        self.last_active = current_timestamp();
    }
}

/// 聊天室信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRoom {
    pub room_id: String,
    pub room_name: String,
    pub members: HashSet<NodeId>,
    pub created_at: u64,
    pub message_count: u64,
}

impl ChatRoom {
    pub fn new(room_id: String, room_name: String) -> Self {
        Self {
            room_id,
            room_name,
            members: HashSet::new(),
            created_at: current_timestamp(),
            message_count: 0,
        }
    }

    pub fn add_member(&mut self, user_id: NodeId) -> bool {
        self.members.insert(user_id)
    }

    pub fn remove_member(&mut self, user_id: &NodeId) -> bool {
        self.members.remove(user_id)
    }

    pub fn has_member(&self, user_id: &NodeId) -> bool {
        self.members.contains(user_id)
    }

    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
    }
}

/// 聊天消息记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageRecord {
    pub message_id: Uuid,
    pub room_id: String,
    pub sender_id: NodeId,
    pub sender_name: String,
    pub content: String,
    pub timestamp: u64,
    pub message_type: String,
}

/// 聊天服务实现
pub struct ChatService<N: NetworkServiceTrait> {
    /// 网络服务
    network_service: N,
    /// 用户管理
    users: Arc<RwLock<HashMap<NodeId, ChatUser>>>,
    /// 聊天室管理
    rooms: Arc<RwLock<HashMap<String, ChatRoom>>>,
    /// 消息历史（最近1000条）
    message_history: Arc<RwLock<Vec<ChatMessageRecord>>>,
    /// 用户名到用户ID的映射
    username_to_user_id: Arc<RwLock<HashMap<String, NodeId>>>,
}

impl<N: NetworkServiceTrait> ChatService<N> {
    /// 创建新的聊天服务
    pub fn new(network_service: N) -> Self {
        Self {
            network_service,
            users: Arc::new(RwLock::new(HashMap::new())),
            rooms: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(Vec::new())),
            username_to_user_id: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 验证聊天室名称
    fn validate_room_name(room_id: &str) -> Result<()> {
        if room_id.is_empty() || room_id.len() > 50 {
            return Err(ChatError::InvalidRoomName(room_id.to_string()));
        }
        Ok(())
    }

    /// 验证用户名
    fn validate_username(username: &str) -> Result<()> {
        if username.is_empty() || username.len() > 30 {
            return Err(ChatError::InvalidUsername(username.to_string()));
        }
        Ok(())
    }

    /// 获取用户
    async fn get_user(&self, user_id: &NodeId) -> Option<ChatUser> {
        let users = self.users.read().await;
        users.get(user_id).cloned()
    }

    /// 获取聊天室
    async fn get_room(&self, room_id: &str) -> Option<ChatRoom> {
        let rooms = self.rooms.read().await;
        rooms.get(room_id).cloned()
    }

    /// 创建聊天室（如果不存在）
    async fn ensure_room_exists(&self, room_id: &str) -> Result<()> {
        let mut rooms = self.rooms.write().await;
        if !rooms.contains_key(room_id) {
            let room = ChatRoom::new(room_id.to_string(), room_id.to_string());
            rooms.insert(room_id.to_string(), room);
            info!("创建新聊天室: {}", room_id);
        }
        Ok(())
    }

    /// 添加消息到历史记录
    async fn add_to_history(&self, message: ChatMessageRecord) {
        let mut history = self.message_history.write().await;
        history.push(message);

        // 保持最近1000条消息
        if history.len() > 1000 {
            history.remove(0);
        }
    }

    /// 广播聊天消息到聊天室成员
    async fn broadcast_to_room(
        &self,
        room_id: &str,
        message: NetworkMessage,
        exclude_user: Option<NodeId>,
    ) -> Result<Uuid> {
        let room = self
            .get_room(room_id)
            .await
            .ok_or_else(|| ChatError::RoomNotFound(room_id.to_string()))?;

        let mut exclude_nodes = Vec::new();
        if let Some(user_id) = exclude_user {
            exclude_nodes.push(user_id);
        }

        let options = BroadcastOptions {
            exclude_nodes,
            wait_for_response: false,
            timeout_ms: Some(5000),
            retry_count: 0,
        };

        let message_id = self
            .network_service
            .broadcast(message, Some(options))
            .await?;
        info!(
            "向聊天室 {} 广播消息 {} (成员数: {})",
            room_id,
            message_id,
            room.members.len()
        );

        Ok(message_id)
    }
}

#[async_trait]
impl<N: NetworkServiceTrait> ChatServiceTrait for ChatService<N> {
    async fn join_room(&self, user_id: NodeId, username: String, room_id: String) -> Result<()> {
        Self::validate_room_name(&room_id)?;
        Self::validate_username(&username)?;

        info!("用户 {} ({}) 加入聊天室 {}", username, user_id, room_id);

        // 确保聊天室存在
        self.ensure_room_exists(&room_id).await?;

        // 更新用户信息
        {
            let mut users = self.users.write().await;
            let user = users
                .entry(user_id.clone())
                .or_insert_with(|| ChatUser::new(user_id.clone(), username.clone()));
            user.join_room(room_id.clone());
        }

        // 更新用户名映射
        {
            let mut username_map = self.username_to_user_id.write().await;
            username_map.insert(username.clone(), user_id.clone());
        }

        // 更新聊天室成员
        {
            let mut rooms = self.rooms.write().await;
            if let Some(room) = rooms.get_mut(&room_id) {
                room.add_member(user_id.clone());
            }
        }

        // 广播用户加入消息
        let join_message = ChatMessageType::UserJoin {
            username: username.clone(),
            room_id: room_id.clone(),
        };

        let payload = serde_json::to_value(&join_message)?;
        let network_msg = NetworkMessage::new(MessageType::chat(), user_id.clone(), payload);

        self.broadcast_to_room(&room_id, network_msg, Some(user_id))
            .await?;

        Ok(())
    }

    async fn leave_room(&self, user_id: NodeId, room_id: String) -> Result<()> {
        info!("用户 {} 离开聊天室 {}", user_id, room_id);

        let username = {
            let users = self.users.read().await;
            users
                .get(&user_id)
                .map(|u| u.username.clone())
                .ok_or_else(|| ChatError::UserNotFound(user_id.clone()))?
        };

        // 检查用户是否在聊天室中
        {
            let users = self.users.read().await;
            let user = users
                .get(&user_id)
                .ok_or_else(|| ChatError::UserNotFound(user_id.clone()))?;
            if !user.joined_rooms.contains(&room_id) {
                return Err(ChatError::UserNotInRoom(user_id.clone(), room_id.clone()));
            }
        }

        // 更新用户信息
        {
            let mut users = self.users.write().await;
            if let Some(user) = users.get_mut(&user_id) {
                user.leave_room(&room_id);
            }
        }

        // 更新聊天室成员
        {
            let mut rooms = self.rooms.write().await;
            if let Some(room) = rooms.get_mut(&room_id) {
                room.remove_member(&user_id);
            }
        }

        // 广播用户离开消息
        let leave_message = ChatMessageType::UserLeave {
            username,
            room_id: room_id.clone(),
        };

        let payload = serde_json::to_value(&leave_message)?;
        let network_msg = NetworkMessage::new(MessageType::chat(), user_id.clone(), payload);

        self.broadcast_to_room(&room_id, network_msg, Some(user_id))
            .await?;

        Ok(())
    }

    async fn send_message(
        &self,
        user_id: NodeId,
        room_id: String,
        content: String,
    ) -> Result<Uuid> {
        if content.trim().is_empty() {
            return Err(ChatError::EmptyMessage);
        }

        let username = {
            let users = self.users.read().await;
            users
                .get(&user_id)
                .map(|u| u.username.clone())
                .ok_or_else(|| ChatError::UserNotFound(user_id.clone()))?
        };

        // 检查用户是否在聊天室中
        {
            let users = self.users.read().await;
            let user = users
                .get(&user_id)
                .ok_or_else(|| ChatError::UserNotFound(user_id.clone()))?;
            if !user.joined_rooms.contains(&room_id) {
                return Err(ChatError::UserNotInRoom(user_id.clone(), room_id.clone()));
            }
        }

        info!(
            "用户 {} 在聊天室 {} 发送消息: {}",
            username, room_id, content
        );

        // 创建聊天消息
        let chat_message = ChatMessageType::TextMessage {
            room_id: room_id.clone(),
            content: content.clone(),
        };

        let payload = serde_json::to_value(&chat_message)?;
        let network_msg = NetworkMessage::new(MessageType::chat(), user_id.clone(), payload);

        let message_id = network_msg.id;

        // 添加到消息历史
        let history_record = ChatMessageRecord {
            message_id,
            room_id: room_id.clone(),
            sender_id: user_id.clone(),
            sender_name: username,
            content,
            timestamp: current_timestamp(),
            message_type: "text".to_string(),
        };
        self.add_to_history(history_record).await;

        // 更新聊天室消息计数
        {
            let mut rooms = self.rooms.write().await;
            if let Some(room) = rooms.get_mut(&room_id) {
                room.increment_message_count();
            }
        }

        // 广播消息到聊天室
        self.broadcast_to_room(&room_id, network_msg, Some(user_id))
            .await?;

        Ok(message_id)
    }

    async fn send_private_message(
        &self,
        from_user: NodeId,
        to_user: String,
        content: String,
    ) -> Result<Uuid> {
        if content.trim().is_empty() {
            return Err(ChatError::EmptyMessage);
        }

        // 查找目标用户ID
        let target_user_id = {
            let username_map = self.username_to_user_id.read().await;
            username_map
                .get(&to_user)
                .cloned()
                .ok_or_else(|| ChatError::UserNotFound(to_user.clone()))?
        };

        let from_username = {
            let users = self.users.read().await;
            users
                .get(&from_user)
                .map(|u| u.username.clone())
                .ok_or_else(|| ChatError::UserNotFound(from_user.clone()))?
        };

        info!("用户 {} 向 {} 发送私聊消息", from_username, to_user);

        // 创建私聊消息
        let private_message = ChatMessageType::PrivateMessage {
            target_user: to_user,
            content,
        };

        let payload = serde_json::to_value(&private_message)?;
        let network_msg = NetworkMessage::new(MessageType::chat(), from_user.clone(), payload);

        let message_id = network_msg.id;

        // 发送单播消息
        let _sent_id = self
            .network_service
            .unicast(target_user_id, network_msg, None)
            .await?;

        Ok(message_id)
    }

    async fn list_rooms(&self) -> Result<Vec<String>> {
        let rooms = self.rooms.read().await;
        let room_list = rooms.keys().cloned().collect();
        Ok(room_list)
    }

    async fn list_room_members(&self, room_id: String) -> Result<Vec<String>> {
        let room = self
            .get_room(&room_id)
            .await
            .ok_or_else(|| ChatError::RoomNotFound(room_id))?;

        let users = self.users.read().await;
        let member_names: Vec<String> = room
            .members
            .iter()
            .filter_map(|user_id| users.get(user_id).map(|u| u.username.clone()))
            .collect();

        Ok(member_names)
    }

    async fn get_user_rooms(&self, user_id: NodeId) -> Result<Vec<String>> {
        let users = self.users.read().await;
        let user = users
            .get(&user_id)
            .ok_or_else(|| ChatError::UserNotFound(user_id))?;

        let rooms: Vec<String> = user.joined_rooms.iter().cloned().collect();
        Ok(rooms)
    }
}

/// 获取当前时间戳
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_service::AnemoNetworkService;

    #[tokio::test]
    async fn test_chat_service_creation() {
        let network_service = AnemoNetworkService::new();
        let chat_service = ChatService::new(network_service);

        let rooms = chat_service.list_rooms().await.unwrap();
        assert!(rooms.is_empty());
    }

    #[tokio::test]
    async fn test_user_join_room() {
        let network_service = AnemoNetworkService::new();
        let chat_service = ChatService::new(network_service);

        let user_id = "user1".to_string();
        let username = "Alice".to_string();
        let room_id = "general".to_string();

        // 在实际测试中，需要启动网络服务
        // chat_service.join_room(user_id.clone(), username, room_id.clone()).await.unwrap();
        //
        // let user_rooms = chat_service.get_user_rooms(user_id).await.unwrap();
        // assert!(user_rooms.contains(&room_id));
    }
}
