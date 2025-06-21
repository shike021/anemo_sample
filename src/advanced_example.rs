use anemo::{Network, Router, Request, Response};
use anyhow::Result;
use std::net::SocketAddr;
use tracing::info;

/// 展示 Router 和 Network 的高级用法示例
pub struct AdvancedNetworkExample;

impl AdvancedNetworkExample {
    /// 创建一个配置完整的服务器网络
    pub async fn create_server(bind_addr: &str) -> Result<Network> {
        // 1. 创建路由器并添加多个服务
        let mut router = Router::new();
        
        // 添加不同的路由处理器
        // router.add_rpc_service(EchoService::new());
        // router.add_rpc_service(ChatService::new());
        
        // 2. 创建网络配置
        let network = Network::bind(bind_addr)
            .server_name("advanced-server")           // 设置服务器名称
            .private_key([0u8; 32])                  // 设置私钥（实际应用中应该随机生成）
            .start(router)?;                         // 启动网络

        info!("高级服务器启动在: {}", network.local_addr());
        Ok(network)
    }

    /// 创建一个客户端网络
    pub async fn create_client(bind_addr: &str) -> Result<Network> {
        let router = Router::new();
        
        let network = Network::bind(bind_addr)
            .server_name("advanced-client")
            .private_key([1u8; 32])                  // 不同的私钥
            .start(router)?;

        info!("高级客户端启动在: {}", network.local_addr());
        Ok(network)
    }

    /// 演示网络连接和基本操作
    pub async fn demonstrate_network_operations(
        client: &Network,
        server_addr: SocketAddr,
    ) -> Result<()> {
        // 1. 连接到服务器
        let peer_id = client.connect(server_addr).await?;
        info!("已连接到服务器，PeerId: {}", peer_id);

        // 2. 获取连接信息
        let local_addr = client.local_addr();
        info!("本地地址: {}", local_addr);

        // 3. 检查连接状态
        let peers = client.peers();
        info!("当前连接的节点数量: {}", peers.len());

        // 4. 断开连接
        client.disconnect(peer_id).await?;
        info!("已断开与服务器的连接");

        Ok(())
    }
}

/// Router 的高级用法示例
pub struct RouterExample;

impl RouterExample {
    /// 创建一个包含多个服务的路由器
    pub fn create_advanced_router() -> Router {
        let mut router = Router::new();

        // 在实际应用中，你会这样添加服务：
        // router.add_rpc_service(UserService::new());
        // router.add_rpc_service(MessageService::new());
        // router.add_rpc_service(FileService::new());

        info!("创建了包含多个服务的高级路由器");
        router
    }

    /// 演示路由器的配置选项
    pub fn demonstrate_router_features() {
        let router = Router::new();
        
        // Router 的主要功能：
        // 1. 路由 RPC 调用到正确的服务
        // 2. 处理服务发现
        // 3. 管理服务生命周期
        // 4. 提供中间件支持（如果支持的话）

        info!("路由器功能演示完成");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_network_creation() -> Result<()> {
        let server = AdvancedNetworkExample::create_server("127.0.0.1:0").await?;
        let client = AdvancedNetworkExample::create_client("127.0.0.1:0").await?;
        
        assert_ne!(server.local_addr(), client.local_addr());
        Ok(())
    }

    #[tokio::test]
    async fn test_router_creation() {
        let router = RouterExample::create_advanced_router();
        // 在实际测试中，你可以验证路由器的配置
    }
} 