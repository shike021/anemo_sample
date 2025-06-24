//! 授时业务模块
//!
//! 基于网络服务层实现的时间同步功能，包括：
//! - 时间查询服务
//! - 时间同步服务
//! - 时间差计算
//! - 网络时延测量

pub mod error;
pub mod message_handler;
pub mod timesync_service;

pub use error::{Result, TimeSyncError};
pub use message_handler::TimeSyncMessageHandler;
pub use timesync_service::{SyncStats, TimeInfo, TimeSyncService};

use async_trait::async_trait;
use network_service::NodeId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 授时消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSyncMessageType {
    /// 时间查询请求
    TimeRequest {
        request_id: Uuid,
        client_timestamp: i64,
    },
    /// 时间查询响应
    TimeResponse {
        request_id: Uuid,
        server_timestamp: i64,
        client_timestamp: i64,
        processing_time_ns: u64,
    },
    /// 时间同步请求
    SyncRequest {
        request_id: Uuid,
        client_time: i64,
        sync_interval_ms: u64,
    },
    /// 时间同步响应
    SyncResponse {
        request_id: Uuid,
        server_time: i64,
        client_time: i64,
        time_offset_ms: i64,
        round_trip_time_ms: u64,
    },
    /// 心跳时间戳
    Heartbeat { timestamp: i64, sequence: u64 },
}

/// 授时响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSyncResponseType {
    /// 时间信息
    TimeInfo {
        current_time: i64,
        timezone: String,
        precision_ns: u64,
    },
    /// 同步结果
    SyncResult {
        success: bool,
        time_offset_ms: i64,
        error_message: Option<String>,
    },
    /// 统计信息
    Stats {
        total_requests: u64,
        avg_response_time_ms: f64,
        last_sync_time: Option<i64>,
    },
    /// 错误信息
    Error { error: String },
}

/// 授时服务trait
#[async_trait]
pub trait TimeSyncServiceTrait: Send + Sync {
    /// 处理时间查询请求
    async fn handle_time_request(
        &self,
        from: NodeId,
        request_id: Uuid,
        client_timestamp: i64,
    ) -> Result<()>;

    /// 发送时间查询请求
    async fn request_time(&self, target: NodeId) -> Result<Uuid>;

    /// 处理时间同步请求
    async fn handle_sync_request(
        &self,
        from: NodeId,
        request_id: Uuid,
        client_time: i64,
        sync_interval_ms: u64,
    ) -> Result<()>;

    /// 发送时间同步请求
    async fn request_sync(&self, target: NodeId, sync_interval_ms: u64) -> Result<Uuid>;

    /// 获取当前时间信息
    async fn get_time_info(&self) -> Result<TimeInfo>;

    /// 获取同步统计信息
    async fn get_sync_stats(&self) -> Result<SyncStats>;

    /// 启动定时心跳
    async fn start_heartbeat(&self, interval_ms: u64) -> Result<()>;

    /// 停止定时心跳
    async fn stop_heartbeat(&self) -> Result<()>;
}
