//! Anemo网络服务的具体实现

use crate::{
    BroadcastOptions, EventBus, EventHandler, MessageHandler, MessageId, MessageType,
    NetworkMessage, NetworkServiceConfig, NetworkServiceTrait, NodeId, Result, UnicastOptions,
};
use anemo::codegen::Bytes;
use anemo::{Network, PeerId, Request, Router};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use serde_json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// 全局节点注册表 - 在实际应用中应该使用分布式注册中心
static GLOBAL_NODES: Lazy<Arc<RwLock<HashMap<NodeId, PeerId>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// 基于Anemo的网络服务实现
#[derive(Clone)]
pub struct AnemoNetworkService {
    /// 网络实例
    network: Arc<RwLock<Option<Network>>>,
    /// 事件总线
    event_bus: Arc<EventBus>,
    /// 消息处理器
    message_handlers: Arc<RwLock<HashMap<MessageType, Box<dyn MessageHandler>>>>,
    /// 服务状态
    is_running: Arc<RwLock<bool>>,
    /// 本地节点ID
    local_node_id: Arc<RwLock<Option<NodeId>>>,
    /// 已知的服务器地址列表
    known_servers: Arc<RwLock<Vec<String>>>,
}

impl AnemoNetworkService {
    /// 创建新的网络服务实例
    pub fn new() -> Self {
        Self {
            network: Arc::new(RwLock::new(None)),
            event_bus: Arc::new(EventBus::new(1000)),
            message_handlers: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            local_node_id: Arc::new(RwLock::new(None)),
            known_servers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 添加已知的服务器地址
    pub async fn add_known_server(&self, server_addr: String) {
        let mut servers = self.known_servers.write().await;
        if !servers.contains(&server_addr) {
            servers.push(server_addr.clone());
            info!("添加已知服务器: {}", server_addr);
        }
    }

    /// 将PeerId转换为NodeId
    fn peer_id_to_node_id(peer_id: PeerId) -> NodeId {
        peer_id.to_string()
    }

    /// 将NodeId转换为PeerId
    fn node_id_to_peer_id(node_id: &NodeId) -> Result<PeerId> {
        // 从全局节点表中查找
        let global_nodes = GLOBAL_NODES
            .try_read()
            .map_err(|_| crate::NetworkError::config_error("无法获取节点表"))?;

        global_nodes
            .get(node_id)
            .cloned()
            .ok_or_else(|| crate::NetworkError::node_not_found(node_id.clone()))
    }

    /// 连接到已知的服务器（延迟执行）
    pub async fn connect_to_known_servers_delayed(&self) {
        // 等待一段时间让网络服务完全启动
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let network = self.network.read().await;
        if let Some(network) = network.as_ref() {
            let servers = self.known_servers.read().await.clone();

            if servers.is_empty() {
                info!("没有已知的服务器地址，跳过连接");
                return;
            }

            for server_addr in servers {
                match server_addr.parse::<SocketAddr>() {
                    Ok(addr) => {
                        info!("尝试连接到服务器: {}", server_addr);
                        match network.connect(addr).await {
                            Ok(peer_id) => {
                                info!("成功连接到服务器: {} -> {}", server_addr, peer_id);

                                // 注册到全局节点表
                                if let Some(local_id) = self.local_node_id.read().await.as_ref() {
                                    let mut global_nodes = GLOBAL_NODES.write().await;
                                    global_nodes.insert(local_id.clone(), peer_id);
                                    info!("节点 {} 已注册到全局节点表", local_id);
                                }
                            }
                            Err(e) => {
                                warn!("连接到服务器 {} 失败: {}", server_addr, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("解析服务器地址 {} 失败: {}", server_addr, e);
                    }
                }
            }
        }
    }
}

#[async_trait]
impl NetworkServiceTrait for AnemoNetworkService {
    async fn start(&self, config: NetworkServiceConfig) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Err(crate::NetworkError::config_error("服务已启动"));
        }

        // 创建路由器
        let router = Router::new();

        // 启动网络服务
        let network = Network::bind(config.bind_address)
            .server_name(config.server_name.clone())
            .private_key(config.private_key)
            .start(router)
            .map_err(|e| crate::NetworkError::connection_error(format!("启动网络失败: {}", e)))?;

        info!("网络服务启动在地址: {}", network.local_addr());

        // 生成本地节点ID（基于地址和服务名）
        let local_id = format!("{}:{}", config.server_name, network.local_addr());

        // 注册到全局节点表
        {
            let mut global_nodes = GLOBAL_NODES.write().await;
            global_nodes.insert(local_id.clone(), network.peer_id());
        }

        // 存储本地信息
        *self.local_node_id.write().await = Some(local_id.clone());
        *self.network.write().await = Some(network);
        *is_running = true;

        info!("网络服务启动完成，节点ID: {}", local_id);
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return Ok(());
        }

        // 从全局节点表中移除自己
        if let Some(local_id) = self.local_node_id.read().await.as_ref() {
            let mut global_nodes = GLOBAL_NODES.write().await;
            global_nodes.remove(local_id);
            info!("节点 {} 已从网络中移除", local_id);
        }

        // 清理本地状态
        *self.local_node_id.write().await = None;
        *self.network.write().await = None;
        *is_running = false;

        info!("网络服务已停止");
        Ok(())
    }

    async fn broadcast(
        &self,
        message: NetworkMessage,
        options: Option<BroadcastOptions>,
    ) -> Result<MessageId> {
        let is_running = *self.is_running.read().await;
        if !is_running {
            return Err(crate::NetworkError::config_error("服务未启动"));
        }

        let exclude_nodes = options
            .as_ref()
            .map(|opt| opt.exclude_nodes.clone())
            .unwrap_or_default();

        info!("广播消息: {:?}", message.message_type);

        let mut sent_count = 0;
        let network = self.network.read().await;

        if let Some(network) = network.as_ref() {
            let global_nodes = GLOBAL_NODES.read().await;
            let local_id = self.local_node_id.read().await;

            for (node_id, peer_id) in global_nodes.iter() {
                // 跳过排除的节点
                if exclude_nodes.contains(node_id) {
                    continue;
                }

                // 跳过自己
                if let Some(ref local) = *local_id {
                    if node_id == local {
                        continue;
                    }
                }

                // 使用Anemo RPC发送消息
                let message_bytes = serde_json::to_vec(&message).map_err(|e| {
                    crate::NetworkError::send_error(format!("序列化消息失败: {}", e))
                })?;
                let request = Request::new(Bytes::from(message_bytes));
                match network.rpc(*peer_id, request).await {
                    Ok(_) => {
                        sent_count += 1;
                    }
                    Err(e) => {
                        warn!("发送消息到节点 {} 失败: {}", node_id, e);
                    }
                }
            }
        }

        info!("广播完成，成功发送到 {} 个节点", sent_count);
        Ok(message.id)
    }

    async fn unicast(
        &self,
        target: NodeId,
        message: NetworkMessage,
        _options: Option<UnicastOptions>,
    ) -> Result<MessageId> {
        let is_running = *self.is_running.read().await;
        if !is_running {
            return Err(crate::NetworkError::config_error("服务未启动"));
        }

        info!("单播消息到 {}: {:?}", target, message.message_type);

        let peer_id = Self::node_id_to_peer_id(&target)?;
        let network = self.network.read().await;

        if let Some(network) = network.as_ref() {
            let message_bytes = serde_json::to_vec(&message)
                .map_err(|e| crate::NetworkError::send_error(format!("序列化消息失败: {}", e)))?;
            let request = Request::new(Bytes::from(message_bytes));
            network
                .rpc(peer_id, request)
                .await
                .map_err(|e| crate::NetworkError::send_error(format!("RPC调用失败: {}", e)))?;
            info!("消息已发送到节点: {}", target);
            Ok(message.id)
        } else {
            Err(crate::NetworkError::config_error("网络服务未启动"))
        }
    }

    async fn get_local_node_id(&self) -> Result<NodeId> {
        let local_id = self.local_node_id.read().await;
        local_id
            .clone()
            .ok_or_else(|| crate::NetworkError::config_error("服务未启动"))
    }

    async fn get_connected_nodes(&self) -> Result<Vec<NodeId>> {
        let is_running = *self.is_running.read().await;
        if !is_running {
            return Err(crate::NetworkError::config_error("服务未启动"));
        }

        let global_nodes = GLOBAL_NODES.read().await;
        let local_id = self.local_node_id.read().await;

        let connected_nodes: Vec<NodeId> = global_nodes
            .keys()
            .filter(|&node_id| {
                // 排除自己
                if let Some(ref local) = *local_id {
                    node_id != local
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        info!("当前连接的节点数: {}", connected_nodes.len());
        Ok(connected_nodes)
    }

    async fn register_message_handler(
        &self,
        message_type: MessageType,
        handler: Box<dyn MessageHandler>,
    ) -> Result<()> {
        let mut handlers = self.message_handlers.write().await;
        handlers.insert(message_type.clone(), handler);
        info!("注册消息处理器: {:?}", message_type);
        Ok(())
    }

    async fn register_event_handler(&self, _handler: Box<dyn EventHandler>) -> Result<()> {
        // 暂时不实现事件处理器
        Ok(())
    }
}
