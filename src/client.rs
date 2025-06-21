use crate::message::{ChatMessage, ChatRequest, ChatResponse};
use anemo::{Network, Request, Router};
use anyhow::Result;
use std::io::{self, Write};
use std::net::SocketAddr;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};

pub async fn run_client(server_addr: SocketAddr, username: String) -> Result<()> {
    info!("正在启动聊天客户端，用户名: {}", username);

    // 创建网络
    let network = Network::bind("127.0.0.1:0")
        .server_name("chat-client")
        .private_key([1u8; 32])
        .start(Router::new())?;

    info!("客户端启动在地址: {}", network.local_addr());

    // 连接到服务器
    info!("正在连接到服务器 {}...", server_addr);
    let peer_id = match timeout(Duration::from_secs(10), network.connect(server_addr)).await {
        Ok(Ok(peer_id)) => {
            info!("已连接到服务器，peer_id: {}", peer_id);
            peer_id
        }
        Ok(Err(e)) => {
            error!("连接服务器失败: {}", e);
            return Err(e.into());
        }
        Err(_) => {
            error!("连接服务器超时");
            return Err(anyhow::anyhow!("连接超时"));
        }
    };

    // 发送用户加入消息
    let join_message = ChatMessage::new_user_joined(username.clone());
    let join_request_data = ChatRequest {
        message: join_message,
    };
    let join_request_bytes = serde_json::to_vec(&join_request_data)?;
    let join_request = Request::new(join_request_bytes.into());

    match network.rpc(peer_id, join_request).await {
        Ok(response) => {
            let response_bytes = response.into_inner();
            let chat_response: ChatResponse = serde_json::from_slice(&response_bytes)?;
            if chat_response.success {
                info!("成功加入聊天室");
            } else {
                error!("加入聊天室失败: {:?}", chat_response.error_msg);
                return Err(anyhow::anyhow!("加入聊天室失败"));
            }
        }
        Err(e) => {
            error!("发送加入消息失败: {}", e);
            return Err(e.into());
        }
    }

    // 开始聊天循环
    info!("欢迎来到聊天室! 输入消息后按回车发送，输入 'quit' 退出");
    println!("=== 聊天室 ===");
    println!("用户名: {}", username);
    println!("输入消息后按回车发送，输入 'quit' 退出");
    println!("================");

    let mut input_buffer = String::new();
    loop {
        // 显示提示符
        print!("[{}] > ", username);
        io::stdout().flush().unwrap();

        // 读取用户输入
        input_buffer.clear();
        match io::stdin().read_line(&mut input_buffer) {
            Ok(_) => {
                let message = input_buffer.trim().to_string();
                if message.is_empty() {
                    continue;
                }

                // 检查退出命令
                if message.to_lowercase() == "quit" || message.to_lowercase() == "exit" {
                    info!("用户选择退出聊天室");
                    break;
                }

                // 发送聊天消息
                let chat_message = ChatMessage::new_text(username.clone(), message.clone());
                let chat_request_data = ChatRequest {
                    message: chat_message,
                };
                let chat_request_bytes = serde_json::to_vec(&chat_request_data).unwrap();
                let chat_request = Request::new(chat_request_bytes.into());

                match timeout(Duration::from_secs(5), network.rpc(peer_id, chat_request)).await {
                    Ok(Ok(response)) => {
                        let response_bytes = response.into_inner();
                        match serde_json::from_slice::<ChatResponse>(&response_bytes) {
                            Ok(chat_response) => {
                                if !chat_response.success {
                                    warn!("发送消息失败: {:?}", chat_response.error_msg);
                                    println!(
                                        "❌ 发送失败: {:?}",
                                        chat_response.error_msg.unwrap_or_default()
                                    );
                                } else {
                                    // 消息发送成功，不需要特别提示
                                }
                            }
                            Err(e) => {
                                warn!("解析响应失败: {}", e);
                                println!("❌ 响应解析失败");
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("发送消息出错: {}", e);
                        println!("❌ 发送出错: {}", e);
                    }
                    Err(_) => {
                        warn!("发送消息超时");
                        println!("❌ 发送超时");
                    }
                }
            }
            Err(e) => {
                error!("读取输入失败: {}", e);
                break;
            }
        }
    }

    // 发送用户离开消息
    let leave_message = ChatMessage::new_user_left(username.clone());
    let leave_request_data = ChatRequest {
        message: leave_message,
    };
    let leave_request_bytes = serde_json::to_vec(&leave_request_data).unwrap();
    let leave_request = Request::new(leave_request_bytes.into());

    if let Err(e) = network.rpc(peer_id, leave_request).await {
        warn!("发送离开消息失败: {}", e);
    }

    info!("客户端正在断开连接...");
    Ok(())
}

/// 简化版客户端，仅用于测试连接
pub async fn run_simple_client(server_addr: SocketAddr) -> Result<()> {
    info!("运行简化版客户端进行连接测试");

    // 创建网络
    let network = Network::bind("127.0.0.1:0")
        .server_name("simple-client")
        .private_key([2u8; 32])
        .start(Router::new())?;

    info!("简化客户端启动在地址: {}", network.local_addr());

    // 连接到服务器
    let peer_id = network.connect(server_addr).await?;
    info!("已连接到服务器，peer_id: {}", peer_id);

    // 保持连接一段时间
    tokio::time::sleep(Duration::from_secs(5)).await;

    info!("简化客户端断开连接");
    Ok(())
}
