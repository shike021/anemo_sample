use anemo::{Network, Router};
use anyhow::Result;
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod chat_service;
mod client;
mod message;
mod network_service;
mod simple_chat;

use simple_chat::{run_simple_client, run_simple_server};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "client" {
        // 运行客户端
        let server_addr: SocketAddr = "127.0.0.1:8080".parse()?;
        let username = args.get(2).unwrap_or(&"匿名用户".to_string()).clone();

        info!("启动简化聊天客户端模式");
        run_simple_client(server_addr, username).await?;
    } else if args.len() > 1 && args[1] == "advanced" {
        // 运行高级版本（原来的复杂实现）
        if args.len() > 2 && args[2] == "client" {
            let server_addr: SocketAddr = "127.0.0.1:8080".parse()?;
            let username = args.get(3).unwrap_or(&"匿名用户".to_string()).clone();
            client::run_client(server_addr, username).await?;
        } else {
            run_advanced_server().await?;
        }
    } else {
        // 默认运行简化服务器
        info!("启动简化聊天服务器模式");
        run_simple_server().await?;
    }

    Ok(())
}

/// 运行高级版本的服务器（保留原有的复杂实现）
async fn run_advanced_server() -> Result<()> {
    info!("正在启动高级聊天服务器...");
    info!("注意：高级版本目前可能存在RPC相关的兼容性问题");

    // 创建路由器
    let router = Router::new();

    // 创建网络
    let network = Network::bind("127.0.0.1:8080")
        .server_name("advanced-chat-server")
        .private_key([0u8; 32])
        .start(router)?;

    info!("高级聊天服务器启动在地址: {}", network.local_addr());
    info!("高级服务器正在运行，等待客户端连接...");
    info!("按 Ctrl+C 停止服务器");

    // 保持服务器运行
    tokio::signal::ctrl_c().await?;
    info!("正在关闭高级聊天服务器...");

    Ok(())
}
