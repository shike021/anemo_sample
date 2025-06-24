//! Anemo网络服务示例程序
//!
//! 这是一个基于分层架构的网络服务示例，展示了如何使用Anemo网络组件
//! 构建网络服务模块，并支持多个业务模块使用网络功能。
//!
//! 架构层次：
//! - 业务层：聊天模块、授时模块
//! - 网络服务层：统一的网络服务接口
//! - 网络实现层：基于Anemo的具体实现
//! - 基础设施层：消息定义、配置、工具

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

// 导入各个模块
use chat_module::{ChatMessageHandler, ChatService, ChatServiceTrait};
use network_service::{
    AnemoNetworkService, MessageType, NetworkServiceConfig, NetworkServiceTrait,
};
use timesync_module::{TimeSyncMessageHandler, TimeSyncService, TimeSyncServiceTrait};

/// 命令行参数
#[derive(Parser)]
#[command(name = "anemo-example")]
#[command(about = "基于Anemo的分层网络服务示例")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 启动服务器
    Server {
        /// 监听地址
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        addr: SocketAddr,
        /// 服务器名称
        #[arg(short, long, default_value = "anemo-server")]
        name: String,
        /// 启用聊天服务
        #[arg(long, default_value = "true")]
        enable_chat: bool,
        /// 启用授时服务
        #[arg(long, default_value = "true")]
        enable_timesync: bool,
        /// 心跳间隔（毫秒）
        #[arg(long, default_value = "30000")]
        heartbeat_interval: u64,
    },
    /// 启动聊天客户端
    ChatClient {
        /// 服务器地址
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        server: SocketAddr,
        /// 用户名
        #[arg(short, long)]
        username: String,
        /// 聊天室
        #[arg(short, long, default_value = "general")]
        room: String,
    },
    /// 启动授时客户端
    TimeSyncClient {
        /// 服务器地址
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        server: SocketAddr,
        /// 同步间隔（毫秒）
        #[arg(long, default_value = "5000")]
        sync_interval: u64,
    },
    /// 运行演示
    Demo,
}

/// 应用程序状态
struct AppState {
    network_service: AnemoNetworkService,
    chat_service: Option<Arc<ChatService<AnemoNetworkService>>>,
    timesync_service: Option<Arc<TimeSyncService<AnemoNetworkService>>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            network_service: AnemoNetworkService::new(),
            chat_service: None,
            timesync_service: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志系统
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // 解析命令行参数
    let cli = Cli::parse();

    // 根据命令执行相应操作
    match cli.command {
        Commands::Server {
            addr,
            name,
            enable_chat,
            enable_timesync,
            heartbeat_interval,
        } => {
            run_server(addr, name, enable_chat, enable_timesync, heartbeat_interval).await?;
        }
        Commands::ChatClient {
            server,
            username,
            room,
        } => {
            run_chat_client(server, username, room).await?;
        }
        Commands::TimeSyncClient {
            server,
            sync_interval,
        } => {
            run_timesync_client(server, sync_interval).await?;
        }
        Commands::Demo => {
            run_demo().await?;
        }
    }

    Ok(())
}

/// 运行服务器
async fn run_server(
    addr: SocketAddr,
    name: String,
    enable_chat: bool,
    enable_timesync: bool,
    heartbeat_interval: u64,
) -> Result<()> {
    info!("🚀 启动网络服务器");
    info!("📍 监听地址: {}", addr);
    info!("📛 服务器名称: {}", name);
    info!("💬 聊天服务: {}", if enable_chat { "启用" } else { "禁用" });
    info!(
        "⏰ 授时服务: {}",
        if enable_timesync { "启用" } else { "禁用" }
    );

    // 创建应用状态
    let mut app_state = AppState::new();

    // 配置网络服务
    let mut config = NetworkServiceConfig::default();
    config.bind_address = addr;
    config.server_name = name.clone();
    config.heartbeat_interval_ms = heartbeat_interval;

    // 启动网络服务
    app_state.network_service.start(config).await?;

    // 启用聊天服务
    if enable_chat {
        info!("🏗️  初始化聊天服务");
        let chat_service = Arc::new(ChatService::new(app_state.network_service.clone()));
        let chat_handler = ChatMessageHandler::new(chat_service.clone());

        app_state
            .network_service
            .register_message_handler(MessageType::chat(), Box::new(chat_handler))
            .await?;

        app_state.chat_service = Some(chat_service);
        info!("✅ 聊天服务已启动");
    }

    // 启用授时服务
    if enable_timesync {
        info!("🏗️  初始化授时服务");
        let timesync_service = Arc::new(TimeSyncService::new(
            app_state.network_service.clone(),
            name.clone(),
        ));
        let timesync_handler = TimeSyncMessageHandler::new(timesync_service.clone());

        app_state
            .network_service
            .register_message_handler(MessageType::timesync(), Box::new(timesync_handler))
            .await?;

        // 启动心跳
        timesync_service.start_heartbeat(heartbeat_interval).await?;

        app_state.timesync_service = Some(timesync_service);
        info!("✅ 授时服务已启动");
    }

    info!("🎉 服务器启动完成！");
    info!("📊 服务状态:");

    // 显示服务状态
    if let Ok(local_id) = app_state.network_service.get_local_node_id().await {
        info!("   🆔 本地节点ID: {}", local_id);
    }

    if let Ok(connected_nodes) = app_state.network_service.get_connected_nodes().await {
        info!("   🔗 连接节点数: {}", connected_nodes.len());
    }

    info!("💡 使用说明:");
    info!("   聊天客户端: cargo run -- chat-client --username <用户名>");
    info!("   授时客户端: cargo run -- time-sync-client");
    info!("   按 Ctrl+C 停止服务器");

    // 等待中断信号
    signal::ctrl_c().await?;

    info!("🛑 收到停止信号，正在关闭服务器...");

    // 停止服务
    if let Some(timesync_service) = app_state.timesync_service {
        if let Err(e) = timesync_service.stop_heartbeat().await {
            error!("停止心跳服务失败: {}", e);
        }
    }

    if let Err(e) = app_state.network_service.stop().await {
        error!("停止网络服务失败: {}", e);
    }

    info!("✅ 服务器已关闭");
    Ok(())
}

/// 运行聊天客户端
async fn run_chat_client(server: SocketAddr, username: String, room: String) -> Result<()> {
    info!("💬 启动聊天客户端");
    info!("🏷️  用户名: {}", username);
    info!("🏠 聊天室: {}", room);
    info!("🌐 服务器: {}", server);

    // 创建网络服务
    let network_service = AnemoNetworkService::new();

    // 创建聊天服务
    let chat_service = Arc::new(ChatService::new(network_service.clone()));
    let chat_handler = ChatMessageHandler::new(chat_service.clone());

    // 注册消息处理器
    network_service
        .register_message_handler(MessageType::chat(), Box::new(chat_handler))
        .await?;

    // 启动网络服务（作为客户端）
    let mut config = NetworkServiceConfig::default();
    config.bind_address = "0.0.0.0:0".parse().unwrap(); // 客户端使用随机端口
    config.server_name = format!("chat-client-{}", username);
    config.max_connections = 10;
    config.message_buffer_size = 100;
    config.event_bus_capacity = 100;

    network_service.start(config).await?;

    // 连接到服务器
    let server_addr = format!("{}", server);
    network_service.add_known_server(server_addr).await;
    info!("🔗 正在连接到服务器: {}", server);

    // 启动延迟连接任务
    let network_service_clone = network_service.clone();
    tokio::spawn(async move {
        network_service_clone
            .connect_to_known_servers_delayed()
            .await;
    });

    // 等待连接建立
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 获取本地节点ID
    let local_id = network_service.get_local_node_id().await?;
    info!("🆔 本地节点ID: {}", local_id);

    // 加入聊天室
    chat_service
        .join_room(local_id.clone(), username.clone(), room.clone())
        .await?;
    info!("✅ 已加入聊天室: {}", room);

    // 启动交互式聊天
    info!("💡 开始聊天! 输入消息后按回车发送，输入 'quit' 退出");
    println!("================== 聊天室: {} ==================", room);

    use tokio::io::{self, AsyncBufReadExt, BufReader};
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);

    loop {
        print!("[{}] > ", username);
        use std::io::Write;
        std::io::stdout().flush().unwrap();

        let mut input = String::new();
        match reader.read_line(&mut input).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }

                if input == "quit" || input == "exit" {
                    break;
                }

                // 发送消息
                match chat_service
                    .send_message(local_id.clone(), room.clone(), input.to_string())
                    .await
                {
                    Ok(message_id) => {
                        println!("✓ 消息已发送 (ID: {})", message_id);
                    }
                    Err(e) => {
                        println!("✗ 发送失败: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("读取输入失败: {}", e);
                break;
            }
        }
    }

    // 离开聊天室
    info!("🚪 离开聊天室...");
    if let Err(e) = chat_service.leave_room(local_id, room).await {
        error!("离开聊天室失败: {}", e);
    }

    // 停止网络服务
    if let Err(e) = network_service.stop().await {
        error!("停止网络服务失败: {}", e);
    }

    info!("✅ 聊天客户端已关闭");
    Ok(())
}

/// 运行授时客户端
async fn run_timesync_client(server: SocketAddr, sync_interval: u64) -> Result<()> {
    info!("⏰ 启动授时客户端");
    info!("🌐 服务器: {}", server);
    info!("⏱️  同步间隔: {}ms", sync_interval);

    // 创建网络服务
    let network_service = AnemoNetworkService::new();

    // 创建授时服务
    let timesync_service = Arc::new(TimeSyncService::new(
        network_service.clone(),
        "timesync-client".to_string(),
    ));
    let timesync_handler = TimeSyncMessageHandler::new(timesync_service.clone());

    // 注册消息处理器
    network_service
        .register_message_handler(MessageType::timesync(), Box::new(timesync_handler))
        .await?;

    // 启动网络服务
    let config = NetworkServiceConfig {
        bind_address: "0.0.0.0:0".parse().unwrap(),
        server_name: "timesync-client".to_string(),
        private_key: [2u8; 32],
        max_connections: 10,
        heartbeat_interval_ms: 30000,
        message_buffer_size: 100,
        event_bus_capacity: 100,
    };

    network_service.start(config).await?;

    // 连接到服务器
    let server_addr = format!("{}", server);
    network_service.add_known_server(server_addr).await;
    info!("🔗 正在连接到服务器: {}", server);

    // 启动延迟连接任务
    let network_service_clone = network_service.clone();
    tokio::spawn(async move {
        network_service_clone
            .connect_to_known_servers_delayed()
            .await;
    });

    // 等待连接建立
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 获取本地节点ID
    let local_id = network_service.get_local_node_id().await?;
    info!("🆔 本地节点ID: {}", local_id);

    // 显示当前时间信息
    let time_info = timesync_service.get_time_info().await?;
    info!("📅 当前时间: {}", time_info.current_time);
    info!("🌍 时区: {}", time_info.timezone);
    info!("⚡ 精度: {}ns", time_info.precision_ns);

    info!("🔄 开始时间同步演示...");
    info!("💡 按 Ctrl+C 停止");

    // 定期同步时间
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(sync_interval));
    let mut sync_count = 0;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                sync_count += 1;
                info!("🔄 执行第 {} 次时间同步", sync_count);

                // 获取连接的节点
                match network_service.get_connected_nodes().await {
                    Ok(nodes) => {
                        if nodes.is_empty() {
                            info!("⚠️  没有连接的节点，等待连接...");
                        } else {
                            for node in nodes {
                                info!("📡 向节点 {} 请求时间同步", node);
                                match timesync_service.request_sync(node, sync_interval).await {
                                    Ok(request_id) => {
                                        info!("✅ 同步请求已发送 (ID: {})", request_id);
                                    }
                                    Err(e) => {
                                        error!("❌ 同步请求失败: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("获取连接节点失败: {}", e);
                    }
                }

                // 显示统计信息
                if let Ok(stats) = timesync_service.get_sync_stats().await {
                    info!("📊 同步统计: 请求数={}, 响应数={}, 平均响应时间={:.2}ms",
                          stats.total_requests, stats.total_responses, stats.avg_response_time_ms);
                }
            }
            _ = signal::ctrl_c() => {
                break;
            }
        }
    }

    info!("🛑 停止授时客户端...");

    // 停止网络服务
    if let Err(e) = network_service.stop().await {
        error!("停止网络服务失败: {}", e);
    }

    info!("✅ 授时客户端已关闭");
    Ok(())
}

/// 运行演示
async fn run_demo() -> Result<()> {
    info!("🎬 启动演示模式");
    info!("📚 这将展示网络服务的各种功能");

    // 创建网络服务
    let network_service = AnemoNetworkService::new();

    // 创建聊天服务
    let chat_service = Arc::new(ChatService::new(network_service.clone()));
    let chat_handler = ChatMessageHandler::new(chat_service.clone());

    // 创建授时服务
    let timesync_service = Arc::new(TimeSyncService::new(
        network_service.clone(),
        "demo-server".to_string(),
    ));
    let timesync_handler = TimeSyncMessageHandler::new(timesync_service.clone());

    // 注册消息处理器
    network_service
        .register_message_handler(MessageType::chat(), Box::new(chat_handler))
        .await?;
    network_service
        .register_message_handler(MessageType::timesync(), Box::new(timesync_handler))
        .await?;

    // 启动网络服务
    let config = NetworkServiceConfig::default();
    network_service.start(config).await?;

    let local_id = network_service.get_local_node_id().await?;
    info!("🆔 本地节点ID: {}", local_id);

    // 演示聊天功能
    info!("💬 演示聊天功能");
    chat_service
        .join_room(
            local_id.clone(),
            "演示用户".to_string(),
            "演示聊天室".to_string(),
        )
        .await?;
    chat_service
        .send_message(
            local_id.clone(),
            "演示聊天室".to_string(),
            "Hello, World!".to_string(),
        )
        .await?;

    let rooms = chat_service.list_rooms().await?;
    info!("📋 聊天室列表: {:?}", rooms);

    // 演示授时功能
    info!("⏰ 演示授时功能");
    let time_info = timesync_service.get_time_info().await?;
    info!("📅 时间信息: {:?}", time_info);

    timesync_service.start_heartbeat(5000).await?;

    info!("⏱️  等待 10 秒展示心跳...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    let stats = timesync_service.get_sync_stats().await?;
    info!("📊 同步统计: {:?}", stats);

    // 停止服务
    info!("🛑 停止演示");
    timesync_service.stop_heartbeat().await?;
    network_service.stop().await?;

    info!("✅ 演示完成");
    Ok(())
}
