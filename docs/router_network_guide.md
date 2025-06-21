# Anemo Router 和 Network 使用指南

## 概述

在 Anemo 网络库中，`Router` 和 `Network` 是两个核心组件，它们协同工作来构建分布式网络应用。

## Network（网络）

`Network` 是 Anemo 的核心抽象，代表一个网络节点。

### 基本用法

```rust
use anemo::{Network, Router};

// 创建网络
let network = Network::bind("127.0.0.1:8080")
    .server_name("my-server")
    .private_key([0u8; 32])
    .start(Router::new())?;
```

### Network 的主要功能

#### 1. 网络绑定和监听
```rust
// 绑定到特定地址
let network = Network::bind("127.0.0.1:8080")
    .start(router)?;

// 绑定到随机端口
let network = Network::bind("127.0.0.1:0")
    .start(router)?;

// 获取实际监听地址
let addr = network.local_addr();
println!("监听地址: {}", addr);
```

#### 2. 节点连接管理
```rust
// 连接到其他节点
let peer_id = network.connect("127.0.0.1:9090".parse()?).await?;

// 断开连接
network.disconnect(peer_id).await?;

// 获取所有连接的节点
let peers = network.peers();
```

#### 3. RPC 调用
```rust
use anemo::{Request, Response};

// 发送 RPC 请求
let request = Request::new(data);
let response = network.rpc(peer_id, request).await?;
```

#### 4. 网络配置选项
```rust
let network = Network::bind("127.0.0.1:8080")
    .server_name("my-server")           // 设置服务器名称
    .private_key([0u8; 32])            // 设置私钥
    .start(router)?;
```

### Network 的关键特性

1. **异步操作** - 所有网络操作都是异步的
2. **TLS 加密** - 默认使用 TLS 加密连接
3. **连接池** - 自动管理连接池
4. **错误处理** - 提供详细的错误信息
5. **节点发现** - 支持动态节点发现

## Router（路由器）

`Router` 负责处理入站请求并将其路由到适当的服务处理器。

### 基本用法

```rust
use anemo::Router;

// 创建路由器
let mut router = Router::new();

// 添加 RPC 服务
// router.add_rpc_service(MyService::new());
```

### Router 的主要功能

#### 1. 服务注册
```rust
// 注册 RPC 服务
let mut router = Router::new();
// router.add_rpc_service(EchoService::new());
// router.add_rpc_service(ChatService::new());
```

#### 2. 请求路由
Router 自动将入站 RPC 请求路由到正确的服务处理器：

```rust
// 当收到 RPC 请求时，Router 会：
// 1. 解析请求的服务名称
// 2. 查找对应的服务处理器
// 3. 调用处理器的方法
// 4. 返回响应
```

#### 3. 中间件支持
```rust
// Router 可能支持中间件（具体取决于 Anemo 版本）
// router.add_middleware(LoggingMiddleware::new());
// router.add_middleware(AuthMiddleware::new());
```

### Router 的设计模式

Router 使用了几种重要的设计模式：

1. **服务定位器模式** - 根据服务名称查找处理器
2. **责任链模式** - 通过中间件链处理请求
3. **工厂模式** - 创建和管理服务实例

## 完整示例

### 服务器端
```rust
use anemo::{Network, Router};
use anyhow::Result;

async fn start_server() -> Result<()> {
    // 1. 创建路由器
    let mut router = Router::new();
    
    // 2. 注册服务
    // router.add_rpc_service(MyService::new());
    
    // 3. 创建网络
    let network = Network::bind("127.0.0.1:8080")
        .server_name("my-server")
        .private_key([0u8; 32])
        .start(router)?;
    
    println!("服务器启动在: {}", network.local_addr());
    
    // 4. 保持运行
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

### 客户端
```rust
use anemo::{Network, Router};
use anyhow::Result;

async fn start_client() -> Result<()> {
    // 1. 创建客户端网络
    let network = Network::bind("127.0.0.1:0")
        .server_name("my-client")
        .private_key([1u8; 32])
        .start(Router::new())?;
    
    // 2. 连接到服务器
    let server_addr = "127.0.0.1:8080".parse()?;
    let peer_id = network.connect(server_addr).await?;
    
    // 3. 发送 RPC 请求
    // let request = Request::new(my_data);
    // let response = network.rpc(peer_id, request).await?;
    
    // 4. 清理
    network.disconnect(peer_id).await?;
    Ok(())
}
```

## 最佳实践

### 1. 错误处理
```rust
match network.connect(addr).await {
    Ok(peer_id) => {
        println!("连接成功: {}", peer_id);
    }
    Err(e) => {
        eprintln!("连接失败: {}", e);
        // 处理错误，可能需要重试
    }
}
```

### 2. 资源管理
```rust
// 确保正确清理资源
let network = Network::bind("127.0.0.1:0")
    .start(router)?;

// 使用完毕后断开连接
for peer_id in network.peers() {
    let _ = network.disconnect(peer_id).await;
}
```

### 3. 配置管理
```rust
// 使用配置结构体
struct NetworkConfig {
    bind_addr: String,
    server_name: String,
    private_key: [u8; 32],
}

impl NetworkConfig {
    fn create_network(&self, router: Router) -> Result<Network> {
        Network::bind(&self.bind_addr)
            .server_name(&self.server_name)
            .private_key(self.private_key)
            .start(router)
    }
}
```

## 常见问题

### 1. TLS 握手失败
- 确保私钥正确配置
- 检查证书设置
- 考虑使用测试配置

### 2. 连接超时
- 检查网络连通性
- 调整超时设置
- 实现重连机制

### 3. 端口冲突
- 使用随机端口（":0"）
- 检查端口占用情况
- 实现端口重试逻辑

## 总结

- **Network** 是网络节点的抽象，负责连接管理和通信
- **Router** 是请求路由器，负责将请求分发到正确的服务
- 两者协同工作，构建完整的分布式网络应用
- 重点关注异步编程、错误处理和资源管理 