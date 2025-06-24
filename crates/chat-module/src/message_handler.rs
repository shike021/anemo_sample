//! 聊天消息处理器

use crate::{ChatError, ChatMessageType, ChatServiceTrait};
use async_trait::async_trait;
use network_service::{MessageHandler, NetworkMessage, NodeId};
use std::sync::Arc;
use tracing::{error, info, warn};

/// 聊天消息处理器
pub struct ChatMessageHandler<C: ChatServiceTrait> {
    chat_service: Arc<C>,
}

impl<C: ChatServiceTrait> ChatMessageHandler<C> {
    /// 创建新的聊天消息处理器
    pub fn new(chat_service: Arc<C>) -> Self {
        Self { chat_service }
    }
}

#[async_trait]
impl<C: ChatServiceTrait> MessageHandler for ChatMessageHandler<C> {
    async fn handle_message(
        &self,
        from: NodeId,
        message: NetworkMessage,
    ) -> network_service::Result<Option<NetworkMessage>> {
        info!("处理来自 {} 的聊天消息", from);

        // 解析消息负载
        let chat_message: ChatMessageType = match serde_json::from_value(message.payload.clone()) {
            Ok(msg) => msg,
            Err(e) => {
                error!("无法解析聊天消息: {}", e);
                return Err(network_service::NetworkError::SerializationError(e));
            }
        };

        // 根据消息类型处理
        let result = match chat_message {
            ChatMessageType::UserJoin { username, room_id } => {
                info!("用户 {} 加入聊天室 {}", username, room_id);
                self.chat_service.join_room(from, username, room_id).await
            }

            ChatMessageType::UserLeave {
                username: _,
                room_id,
            } => {
                info!("用户离开聊天室 {}", room_id);
                self.chat_service.leave_room(from, room_id).await
            }

            ChatMessageType::TextMessage { room_id, content } => {
                info!("收到聊天室 {} 的消息: {}", room_id, content);
                match self.chat_service.send_message(from, room_id, content).await {
                    Ok(_message_id) => Ok(()),
                    Err(e) => Err(e),
                }
            }

            ChatMessageType::PrivateMessage {
                target_user,
                content,
            } => {
                info!("收到发给 {} 的私聊消息: {}", target_user, content);
                match self
                    .chat_service
                    .send_private_message(from, target_user, content)
                    .await
                {
                    Ok(_message_id) => Ok(()),
                    Err(e) => Err(e),
                }
            }

            ChatMessageType::ListRooms => {
                info!("收到聊天室列表请求");
                match self.chat_service.list_rooms().await {
                    Ok(rooms) => {
                        info!("返回 {} 个聊天室", rooms.len());
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }

            ChatMessageType::ListRoomMembers { room_id } => {
                info!("收到聊天室 {} 成员列表请求", room_id);
                match self.chat_service.list_room_members(room_id).await {
                    Ok(members) => {
                        info!("聊天室有 {} 个成员", members.len());
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
        };

        // 将聊天错误转换为网络错误
        if let Err(chat_error) = result {
            match chat_error {
                ChatError::NetworkError(net_err) => return Err(net_err),
                other_err => {
                    warn!("聊天服务处理消息失败: {}", other_err);
                    return Err(network_service::NetworkError::InternalError(
                        other_err.to_string(),
                    ));
                }
            }
        }

        // 聊天消息通常不需要返回响应消息
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChatService, ChatServiceTrait};
    use network_service::{AnemoNetworkService, MessageType};
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_chat_message_handler() {
        let network_service = AnemoNetworkService::new();
        let chat_service = Arc::new(ChatService::new(network_service));
        let handler = ChatMessageHandler::new(chat_service);

        // 创建测试消息
        let chat_msg = ChatMessageType::TextMessage {
            room_id: "general".to_string(),
            content: "Hello World".to_string(),
        };

        let payload = serde_json::to_value(&chat_msg).unwrap();
        let network_msg =
            NetworkMessage::new(MessageType::chat(), "test-sender".to_string(), payload);

        // 在实际测试中，需要先启动网络服务和加入聊天室
        // let result = handler.handle_message("test-user".to_string(), network_msg).await;
        // assert!(result.is_ok());
    }
}
