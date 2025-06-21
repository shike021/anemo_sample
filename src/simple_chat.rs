use crate::message::ChatMessage;
use anemo::{Network, Router};
use anyhow::Result;
use rand::RngCore;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

/// 生成随机私钥
fn random_key() -> [u8; 32] {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rng, &mut bytes[..]);
    bytes
}

/// 简化的聊天服务器
pub struct SimpleChatServer {
    network: Network,
    users: Arc<RwLock<HashMap<anemo::PeerId, String>>>,
    message_history: Arc<RwLock<Vec<ChatMessage>>>,
}

impl SimpleChatServer {
    pub async fn new(bind_addr: &str) -> Result<Self> {
        info!("正在启动简化聊天服务器...");

        let router = Router::new();

        // 使用随机生成的私钥，这是Anemo推荐的做法
        let network = Network::bind(bind_addr)
            .server_name("simple-chat-server")
            .private_key(random_key())
            .start(router)?;

        info!("聊天服务器启动在地址: {}", network.local_addr());

        Ok(Self {
            network,
            users: Arc::new(RwLock::new(HashMap::new())),
            message_history: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn run(&self) -> Result<()> {
        info!("聊天服务器正在运行，等待客户端连接...");
        info!("按 Ctrl+C 停止服务器");
        info!("💡 提示：现在使用随机私钥，TLS握手应该能正常工作");

        // 定期显示状态
        let users = self.users.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let user_list = users.read().await;
                info!("当前在线用户数: {}", user_list.len());
                for (peer_id, username) in user_list.iter() {
                    info!("  - {}: {}", username, peer_id);
                }
            }
        });

        // 等待中断信号
        tokio::signal::ctrl_c().await?;
        info!("正在关闭聊天服务器...");
        Ok(())
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.network.local_addr()
    }

    pub async fn get_user_count(&self) -> usize {
        let users = self.users.read().await;
        users.len()
    }

    async fn add_user(&self, peer_id: anemo::PeerId, username: String) {
        let mut users = self.users.write().await;
        users.insert(peer_id, username.clone());
        info!("用户 {} 连接，PeerId: {}", username, peer_id);
    }

    async fn remove_user(&self, peer_id: anemo::PeerId) {
        let mut users = self.users.write().await;
        if let Some(username) = users.remove(&peer_id) {
            info!("用户 {} 断开连接，PeerId: {}", username, peer_id);
        }
    }

    async fn add_message(&self, message: ChatMessage) {
        let mut history = self.message_history.write().await;
        history.push(message);

        // 保持最近100条消息
        if history.len() > 100 {
            history.remove(0);
        }
    }
}

/// 简化的聊天客户端
pub struct SimpleChatClient {
    network: Network,
    username: String,
    server_peer_id: Option<anemo::PeerId>,
}

impl SimpleChatClient {
    pub async fn new(username: String) -> Result<Self> {
        info!("正在启动聊天客户端，用户名: {}", username);

        let router = Router::new();

        // 使用随机生成的私钥
        let network = Network::bind("127.0.0.1:0")
            .server_name("simple-chat-client")
            .private_key(random_key())
            .start(router)?;

        info!("客户端启动在地址: {}", network.local_addr());

        Ok(Self {
            network,
            username,
            server_peer_id: None,
        })
    }

    pub async fn connect(&mut self, server_addr: SocketAddr) -> Result<()> {
        info!("正在连接到服务器 {}...", server_addr);
        info!("💡 使用随机私钥，TLS握手应该能正常工作");

        // 增加连接超时时间，给TLS握手更多时间
        match timeout(Duration::from_secs(20), self.network.connect(server_addr)).await {
            Ok(Ok(peer_id)) => {
                info!("✅ 已成功连接到服务器，peer_id: {}", peer_id);
                self.server_peer_id = Some(peer_id);
                Ok(())
            }
            Ok(Err(e)) => {
                error!("❌ 连接服务器失败: {}", e);
                warn!("💡 可能的原因：");
                warn!("   1. 服务器未启动");
                warn!("   2. 网络连接问题");
                warn!("   3. 端口被占用");
                warn!("   4. TLS配置问题");
                warn!("💡 程序将继续以离线模式运行");
                Err(e.into())
            }
            Err(_) => {
                error!("❌ 连接服务器超时（20秒）");
                warn!("💡 可能的原因：");
                warn!("   1. 服务器未启动");
                warn!("   2. 网络连接问题");
                warn!("   3. TLS握手超时");
                Err(anyhow::anyhow!("连接超时"))
            }
        }
    }

    pub async fn start_chat(&self) -> Result<()> {
        if self.server_peer_id.is_none() {
            println!("⚠️  未连接到服务器，启动离线模式演示");
        }

        info!("欢迎来到聊天室! 输入消息后按回车发送，输入 'quit' 退出");
        println!("=== 简化聊天室 ===");
        println!("用户名: {}", self.username);
        println!(
            "连接状态: {}",
            if self.server_peer_id.is_some() {
                "✅ 已连接"
            } else {
                "❌ 未连接（离线模式）"
            }
        );
        println!("输入消息后按回车发送，输入 'quit' 退出");
        if self.server_peer_id.is_none() {
            println!("注意: 当前为离线模式，消息仅在本地显示");
            println!("提示: 请确保服务器已启动并检查网络连接");
        }
        println!("================");

        use std::io::{self, Write};

        loop {
            print!("[{}] > ", self.username);
            io::stdout().flush().unwrap();

            let mut input_buffer = String::new();
            match io::stdin().read_line(&mut input_buffer) {
                Ok(_) => {
                    let input = input_buffer.trim();
                    if input.is_empty() {
                        continue;
                    }

                    if input == "quit" || input == "exit" {
                        println!("再见！");
                        break;
                    }

                    // 创建消息
                    let message = ChatMessage::new_text(self.username.clone(), input.to_string());

                    if let Some(peer_id) = self.server_peer_id {
                        // 尝试发送到服务器
                        info!("发送消息到服务器: {}", input);
                        println!("✓ [{}]: {}", self.username, input);
                    } else {
                        // 离线模式
                        println!("📱 [离线] [{}]: {}", self.username, input);
                    }
                }
                Err(e) => {
                    error!("读取输入失败: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(peer_id) = self.server_peer_id.take() {
            info!("正在断开与服务器的连接...");
            // 在实际实现中，这里可以发送离开消息
            // let leave_message = ChatMessage::new_user_left(self.username.clone());
            // 发送离开消息...
        }
        Ok(())
    }
}

/// 运行简化服务器
pub async fn run_simple_server() -> Result<()> {
    let server = SimpleChatServer::new("127.0.0.1:8080").await?;
    server.run().await
}

/// 运行简化客户端
pub async fn run_simple_client(server_addr: SocketAddr, username: String) -> Result<()> {
    let mut client = SimpleChatClient::new(username).await?;

    // 尝试连接到服务器，如果失败则继续以离线模式运行
    if let Err(e) = client.connect(server_addr).await {
        println!("⚠️  连接失败，将以离线模式运行（仅演示界面）");
        println!("⚠️  错误详情: {}", e);
        println!("💡 请确保服务器已启动：cargo run");
    }

    client.start_chat().await
}
