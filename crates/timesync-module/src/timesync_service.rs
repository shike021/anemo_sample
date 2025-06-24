//! 授时服务实现

use crate::{Result, TimeSyncError, TimeSyncMessageType, TimeSyncServiceTrait};
use async_trait::async_trait;
use chrono::Utc;
use network_service::{
    MessageId, MessageType, NetworkMessage, NetworkServiceTrait, NodeId, UnicastOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tokio::time::interval;
use tracing::{info, warn};
use uuid::Uuid;

/// 时间信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeInfo {
    pub current_time: i64,
    pub timezone: String,
    pub precision_ns: u64,
    pub server_id: String,
}

/// 同步统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub total_requests: u64,
    pub total_responses: u64,
    pub avg_response_time_ms: f64,
    pub last_sync_time: Option<i64>,
    pub active_sessions: usize,
    pub heartbeat_count: u64,
}

/// 时间请求记录
#[derive(Debug, Clone)]
struct TimeRequest {
    request_id: Uuid,
    from_node: NodeId,
    client_timestamp: i64,
    server_receive_time: Instant,
}

/// 同步会话信息
#[derive(Debug, Clone)]
struct SyncSession {
    node_id: NodeId,
    last_sync_time: i64,
    time_offset_ms: i64,
    sync_interval_ms: u64,
    request_count: u64,
}

/// 授时服务实现
pub struct TimeSyncService<N: NetworkServiceTrait> {
    /// 网络服务
    network_service: N,
    /// 待处理的时间请求
    pending_requests: Arc<RwLock<HashMap<Uuid, TimeRequest>>>,
    /// 同步会话
    sync_sessions: Arc<RwLock<HashMap<NodeId, SyncSession>>>,
    /// 统计信息
    stats: Arc<RwLock<SyncStats>>,
    /// 心跳状态
    heartbeat_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 心跳序列号
    heartbeat_sequence: Arc<RwLock<u64>>,
    /// 服务器ID
    server_id: String,
}

impl<N: NetworkServiceTrait> TimeSyncService<N> {
    /// 创建新的授时服务
    pub fn new(network_service: N, server_id: String) -> Self {
        Self {
            network_service,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            sync_sessions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(SyncStats {
                total_requests: 0,
                total_responses: 0,
                avg_response_time_ms: 0.0,
                last_sync_time: None,
                active_sessions: 0,
                heartbeat_count: 0,
            })),
            heartbeat_handle: Arc::new(Mutex::new(None)),
            heartbeat_sequence: Arc::new(RwLock::new(0)),
            server_id,
        }
    }

    /// 获取当前高精度时间戳（纳秒）
    fn get_current_timestamp_ns() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// 获取当前时间戳（毫秒）
    fn get_current_timestamp_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    /// 计算两个时间戳之间的差值（毫秒）
    fn calculate_time_diff_ms(time1: i64, time2: i64) -> i64 {
        time1 - time2
    }

    /// 验证时间戳是否合理
    fn validate_timestamp(timestamp: i64) -> Result<()> {
        let current = Self::get_current_timestamp_ms();
        let diff = (current - timestamp).abs();

        // 允许最大1小时的时间差
        if diff > 3600000 {
            return Err(TimeSyncError::InvalidTimestamp(timestamp));
        }

        Ok(())
    }

    /// 更新统计信息
    async fn update_stats(&self, request_processed: bool, response_time_ms: Option<f64>) {
        let mut stats = self.stats.write().await;

        if request_processed {
            stats.total_requests += 1;
        } else {
            stats.total_responses += 1;
        }

        if let Some(response_time) = response_time_ms {
            // 更新平均响应时间
            let total_count = stats.total_responses as f64;
            stats.avg_response_time_ms =
                (stats.avg_response_time_ms * (total_count - 1.0) + response_time) / total_count;
        }

        stats.last_sync_time = Some(Self::get_current_timestamp_ms());
        stats.active_sessions = self.sync_sessions.read().await.len();
    }

    /// 发送时间响应
    async fn send_time_response(
        &self,
        target: NodeId,
        request_id: Uuid,
        client_timestamp: i64,
        processing_time_ns: u64,
    ) -> Result<()> {
        let server_timestamp = Self::get_current_timestamp_ms();

        let response_message = TimeSyncMessageType::TimeResponse {
            request_id,
            server_timestamp,
            client_timestamp,
            processing_time_ns,
        };

        let payload = serde_json::to_value(&response_message)?;
        let network_msg =
            NetworkMessage::new(MessageType::timesync(), self.server_id.clone(), payload);

        let options = UnicastOptions {
            wait_for_response: false,
            timeout_ms: Some(3000),
            retry_count: 1,
        };

        let _message_id = self
            .network_service
            .unicast(target.clone(), network_msg, Some(options))
            .await?;

        info!("发送时间响应给 {}: {}", target, server_timestamp);
        Ok(())
    }

    /// 发送同步响应
    async fn send_sync_response(
        &self,
        target: NodeId,
        request_id: Uuid,
        client_time: i64,
        time_offset_ms: i64,
        round_trip_time_ms: u64,
    ) -> Result<()> {
        let server_time = Self::get_current_timestamp_ms();

        let response_message = TimeSyncMessageType::SyncResponse {
            request_id,
            server_time,
            client_time,
            time_offset_ms,
            round_trip_time_ms,
        };

        let payload = serde_json::to_value(&response_message)?;
        let network_msg =
            NetworkMessage::new(MessageType::timesync(), self.server_id.clone(), payload);

        let options = UnicastOptions {
            wait_for_response: false,
            timeout_ms: Some(3000),
            retry_count: 1,
        };

        let _message_id = self
            .network_service
            .unicast(target.clone(), network_msg, Some(options))
            .await?;

        info!("发送同步响应给 {}: offset={}ms", target, time_offset_ms);
        Ok(())
    }
}

#[async_trait]
impl<N: NetworkServiceTrait + 'static> TimeSyncServiceTrait for TimeSyncService<N> {
    async fn handle_time_request(
        &self,
        from: NodeId,
        request_id: Uuid,
        client_timestamp: i64,
    ) -> Result<()> {
        info!("处理来自 {} 的时间请求: {}", from, client_timestamp);

        // 验证时间戳
        Self::validate_timestamp(client_timestamp)?;

        let start_time = Instant::now();

        // 记录请求信息
        {
            let mut requests = self.pending_requests.write().await;
            requests.insert(
                request_id,
                TimeRequest {
                    request_id,
                    from_node: from.clone(),
                    client_timestamp,
                    server_receive_time: start_time,
                },
            );
        }

        // 计算处理时间
        let processing_time_ns = start_time.elapsed().as_nanos() as u64;

        // 发送响应
        self.send_time_response(
            from.clone(),
            request_id,
            client_timestamp,
            processing_time_ns,
        )
        .await?;

        // 清理请求记录
        {
            let mut requests = self.pending_requests.write().await;
            requests.remove(&request_id);
        }

        // 更新统计
        self.update_stats(true, None).await;

        Ok(())
    }

    async fn request_time(&self, target: NodeId) -> Result<Uuid> {
        let request_id = Uuid::new_v4();
        let client_timestamp = Self::get_current_timestamp_ms();

        info!("向 {} 请求时间: {}", target, client_timestamp);

        let request_message = TimeSyncMessageType::TimeRequest {
            request_id,
            client_timestamp,
        };

        let payload = serde_json::to_value(&request_message)?;
        let network_msg =
            NetworkMessage::new(MessageType::timesync(), self.server_id.clone(), payload);

        let options = UnicastOptions {
            wait_for_response: true,
            timeout_ms: Some(5000),
            retry_count: 2,
        };

        self.network_service
            .unicast(target, network_msg, Some(options))
            .await?;

        Ok(request_id)
    }

    async fn handle_sync_request(
        &self,
        from: NodeId,
        request_id: Uuid,
        client_time: i64,
        sync_interval_ms: u64,
    ) -> Result<()> {
        info!(
            "处理来自 {} 的同步请求: client_time={}, interval={}ms",
            from, client_time, sync_interval_ms
        );

        // 验证时间戳和同步间隔
        Self::validate_timestamp(client_time)?;
        if sync_interval_ms < 1000 || sync_interval_ms > 3600000 {
            return Err(TimeSyncError::InvalidSyncInterval(sync_interval_ms));
        }

        let server_time = Self::get_current_timestamp_ms();
        let time_offset_ms = Self::calculate_time_diff_ms(server_time, client_time);

        // 更新或创建同步会话
        {
            let mut sessions = self.sync_sessions.write().await;
            let session = sessions.entry(from.clone()).or_insert_with(|| SyncSession {
                node_id: from.clone(),
                last_sync_time: server_time,
                time_offset_ms,
                sync_interval_ms,
                request_count: 0,
            });

            session.last_sync_time = server_time;
            session.time_offset_ms = time_offset_ms;
            session.sync_interval_ms = sync_interval_ms;
            session.request_count += 1;
        }

        // 模拟网络往返时间（实际应用中可以测量）
        let round_trip_time_ms = 10; // 假设10ms

        // 发送同步响应
        self.send_sync_response(
            from,
            request_id,
            client_time,
            time_offset_ms,
            round_trip_time_ms,
        )
        .await?;

        // 更新统计
        self.update_stats(true, Some(round_trip_time_ms as f64))
            .await;

        Ok(())
    }

    async fn request_sync(&self, target: NodeId, sync_interval_ms: u64) -> Result<Uuid> {
        let request_id = Uuid::new_v4();
        let client_time = Self::get_current_timestamp_ms();

        info!(
            "向 {} 请求时间同步: interval={}ms",
            target, sync_interval_ms
        );

        let request_message = TimeSyncMessageType::SyncRequest {
            request_id,
            client_time,
            sync_interval_ms,
        };

        let payload = serde_json::to_value(&request_message)?;
        let network_msg =
            NetworkMessage::new(MessageType::timesync(), self.server_id.clone(), payload);

        let options = UnicastOptions {
            wait_for_response: true,
            timeout_ms: Some(5000),
            retry_count: 2,
        };

        self.network_service
            .unicast(target, network_msg, Some(options))
            .await?;

        Ok(request_id)
    }

    async fn get_time_info(&self) -> Result<TimeInfo> {
        let current_time = Self::get_current_timestamp_ms();
        let timezone = Utc::now().timezone().to_string();
        let precision_ns = 1000000; // 毫秒精度

        Ok(TimeInfo {
            current_time,
            timezone,
            precision_ns,
            server_id: self.server_id.clone(),
        })
    }

    async fn get_sync_stats(&self) -> Result<SyncStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn start_heartbeat(&self, interval_ms: u64) -> Result<()> {
        let mut handle_guard = self.heartbeat_handle.lock().await;

        if handle_guard.is_some() {
            return Err(TimeSyncError::HeartbeatAlreadyStarted);
        }

        info!("启动心跳服务，间隔: {}ms", interval_ms);

        let network_service = self.network_service.clone();
        let server_id = self.server_id.clone();
        let heartbeat_sequence = self.heartbeat_sequence.clone();
        let stats = self.stats.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(interval_ms));

            loop {
                interval.tick().await;

                let sequence = {
                    let mut seq = heartbeat_sequence.write().await;
                    *seq += 1;
                    *seq
                };

                let timestamp = Self::get_current_timestamp_ms();

                let heartbeat_message = TimeSyncMessageType::Heartbeat {
                    timestamp,
                    sequence,
                };

                if let Ok(payload) = serde_json::to_value(&heartbeat_message) {
                    let network_msg =
                        NetworkMessage::new(MessageType::timesync(), server_id.clone(), payload);

                    // 广播心跳消息
                    if let Err(e) = network_service.broadcast(network_msg, None).await {
                        warn!("心跳广播失败: {}", e);
                    } else {
                        // 更新心跳计数
                        let mut stats_guard = stats.write().await;
                        stats_guard.heartbeat_count += 1;
                    }
                }
            }
        });

        *handle_guard = Some(handle);

        Ok(())
    }

    async fn stop_heartbeat(&self) -> Result<()> {
        let mut handle_guard = self.heartbeat_handle.lock().await;

        if let Some(handle) = handle_guard.take() {
            handle.abort();
            info!("心跳服务已停止");
            Ok(())
        } else {
            Err(TimeSyncError::HeartbeatNotStarted)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_service::AnemoNetworkService;

    #[tokio::test]
    async fn test_timesync_service_creation() {
        let network_service = AnemoNetworkService::new();
        let timesync_service = TimeSyncService::new(network_service, "test-server".to_string());

        let time_info = timesync_service.get_time_info().await.unwrap();
        assert_eq!(time_info.server_id, "test-server");
    }

    #[tokio::test]
    async fn test_timestamp_validation() {
        let current = TimeSyncService::<AnemoNetworkService>::get_current_timestamp_ms();

        // 正常时间戳应该通过验证
        assert!(TimeSyncService::<AnemoNetworkService>::validate_timestamp(current).is_ok());

        // 过时的时间戳应该失败
        let old_timestamp = current - 7200000; // 2小时前
        assert!(TimeSyncService::<AnemoNetworkService>::validate_timestamp(old_timestamp).is_err());
    }
}
