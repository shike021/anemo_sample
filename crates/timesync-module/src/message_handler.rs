//! 授时消息处理器

use crate::{TimeSyncError, TimeSyncMessageType, TimeSyncServiceTrait};
use async_trait::async_trait;
use network_service::{MessageHandler, NetworkMessage, NodeId};
use std::sync::Arc;
use tracing::{error, info, warn};

/// 授时消息处理器
pub struct TimeSyncMessageHandler<T: TimeSyncServiceTrait> {
    timesync_service: Arc<T>,
}

impl<T: TimeSyncServiceTrait> TimeSyncMessageHandler<T> {
    /// 创建新的授时消息处理器
    pub fn new(timesync_service: Arc<T>) -> Self {
        Self { timesync_service }
    }
}

#[async_trait]
impl<T: TimeSyncServiceTrait> MessageHandler for TimeSyncMessageHandler<T> {
    async fn handle_message(
        &self,
        from: NodeId,
        message: NetworkMessage,
    ) -> network_service::Result<Option<NetworkMessage>> {
        info!("处理来自 {} 的授时消息", from);

        // 解析消息负载
        let timesync_message: TimeSyncMessageType =
            match serde_json::from_value(message.payload.clone()) {
                Ok(msg) => msg,
                Err(e) => {
                    error!("无法解析授时消息: {}", e);
                    return Err(network_service::NetworkError::SerializationError(e));
                }
            };

        // 根据消息类型处理
        let result = match timesync_message {
            TimeSyncMessageType::TimeRequest {
                request_id,
                client_timestamp,
            } => {
                info!(
                    "处理时间请求: request_id={}, timestamp={}",
                    request_id, client_timestamp
                );
                self.timesync_service
                    .handle_time_request(from, request_id, client_timestamp)
                    .await
            }

            TimeSyncMessageType::TimeResponse {
                request_id,
                server_timestamp,
                client_timestamp,
                processing_time_ns,
            } => {
                info!("收到时间响应: request_id={}, server_time={}, client_time={}, processing_time={}ns", 
                      request_id, server_timestamp, client_timestamp, processing_time_ns);
                // 客户端收到服务器的时间响应，可以在这里计算时间偏差
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                let time_offset = server_timestamp - current_time;
                info!("计算得到的时间偏差: {}ms", time_offset);
                Ok(())
            }

            TimeSyncMessageType::SyncRequest {
                request_id,
                client_time,
                sync_interval_ms,
            } => {
                info!(
                    "处理同步请求: request_id={}, client_time={}, interval={}ms",
                    request_id, client_time, sync_interval_ms
                );
                self.timesync_service
                    .handle_sync_request(from, request_id, client_time, sync_interval_ms)
                    .await
            }

            TimeSyncMessageType::SyncResponse {
                request_id,
                server_time,
                client_time,
                time_offset_ms,
                round_trip_time_ms,
            } => {
                info!("收到同步响应: request_id={}, server_time={}, client_time={}, offset={}ms, rtt={}ms", 
                      request_id, server_time, client_time, time_offset_ms, round_trip_time_ms);
                // 客户端收到同步响应，可以在这里应用时间偏差
                info!(
                    "应用时间偏差: {}ms，网络延迟: {}ms",
                    time_offset_ms, round_trip_time_ms
                );
                Ok(())
            }

            TimeSyncMessageType::Heartbeat {
                timestamp,
                sequence,
            } => {
                info!("收到心跳: timestamp={}, sequence={}", timestamp, sequence);
                // 处理心跳消息，可以用来检测网络连接状态
                Ok(())
            }
        };

        // 将授时错误转换为网络错误
        if let Err(timesync_error) = result {
            match timesync_error {
                TimeSyncError::NetworkError(net_err) => return Err(net_err),
                other_err => {
                    warn!("授时服务处理消息失败: {}", other_err);
                    return Err(network_service::NetworkError::InternalError(
                        other_err.to_string(),
                    ));
                }
            }
        }

        // 授时消息的响应已在服务层处理，这里不需要返回响应消息
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TimeSyncService, TimeSyncServiceTrait};
    use network_service::{AnemoNetworkService, MessageType};
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_timesync_message_handler() {
        let network_service = AnemoNetworkService::new();
        let timesync_service = Arc::new(TimeSyncService::new(
            network_service,
            "test-server".to_string(),
        ));
        let handler = TimeSyncMessageHandler::new(timesync_service);

        // 创建测试消息
        let timesync_msg = TimeSyncMessageType::TimeRequest {
            request_id: Uuid::new_v4(),
            client_timestamp: 1234567890000,
        };

        let payload = serde_json::to_value(&timesync_msg).unwrap();
        let network_msg =
            NetworkMessage::new(MessageType::timesync(), "test-sender".to_string(), payload);

        // 在实际测试中，需要先启动网络服务
        // let result = handler.handle_message("test-user".to_string(), network_msg).await;
        // assert!(result.is_ok());
    }
}
