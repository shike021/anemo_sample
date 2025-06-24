# Anemo 分层网络服务示例

这是一个基于 Anemo 网络库构建的分层网络服务示例项目，展示了如何设计良好的网络服务架构，使业务模块可以方便地使用网络功能，同时保持与具体网络实现的解耦。

## 🏗️ 项目架构

### 分层设计

```
┌─────────────────────────────────────────┐
│             业务层 (Business Layer)      │
├─────────────────┬───────────────────────┤
│   聊天模块      │     授时模块           │
│  (Chat Module)  │  (TimeSync Module)    │
├─────────────────┴───────────────────────┤
│        网络服务层 (Network Service)      │
├─────────────────────────────────────────┤
│       网络实现层 (Anemo Implementation)  │
├─────────────────────────────────────────┤
│     基础设施层 (Infrastructure Layer)    │
└─────────────────────────────────────────┘
```

### 模块组织

项目采用 Cargo workspace 结构，包含以下模块：

- **`network-service`**: 网络服务抽象层
- **`chat-module`**: 聊天业务模块
- **`timesync-module`**: 授时业务模块

## ✨ 核心特性

### 🔧 分层和解耦
- **网络服务抽象**: 统一的网络操作接口
- **业务模块独立**: 聊天和授时模块相互独立
- **实现可替换**: 可以轻松替换底层网络组件

### 🚀 网络功能
- **消息广播**: 向所有连接节点广播消息
- **单点发送**: 向指定节点发送消息
- **事件系统**: 完整的网络事件处理机制
- **错误处理**: 完善的错误处理和恢复机制

### 💬 聊天功能
- **多聊天室**: 支持创建和管理多个聊天室
- **用户管理**: 用户加入/离开聊天室
- **消息历史**: 保存聊天消息历史记录
- **私聊支持**: 支持用户间私聊

### ⏰ 授时功能
- **时间查询**: 查询服务器当前时间
- **时间同步**: 客户端与服务器时间同步
- **心跳机制**: 定期心跳保持连接活跃
- **统计信息**: 同步统计和性能监控

## 🚀 快速开始

### 环境要求

- Rust 1.70+
- 网络环境支持 UDP 通信

### 安装和编译

```bash
# 克隆项目
git clone <repository-url>
cd anemo-example

# 编译项目
cargo build

# 运行测试
cargo test
```

### 基本使用

#### 1. 启动服务器

```bash
# 启动完整功能服务器
cargo run -- server

# 自定义配置
cargo run -- server --addr 0.0.0.0:9000 --name my-server
```

#### 2. 启动聊天客户端

```bash
# 启动聊天客户端
cargo run -- chat-client --username Alice

# 指定聊天室
cargo run -- chat-client --username Bob --room general
```

#### 3. 启动授时客户端

```bash
# 启动授时客户端
cargo run -- time-sync-client

# 自定义同步间隔
cargo run -- time-sync-client --sync-interval 3000
```

#### 4. 运行演示

```bash
# 运行功能演示
cargo run -- demo
```

## 📖 详细使用指南

### 服务器配置

服务器支持以下配置选项：

```bash
cargo run -- server \
  --addr 127.0.0.1:8080 \        # 监听地址
  --name "my-server" \           # 服务器名称
  --enable-chat true \           # 启用聊天服务
  --enable-timesync true \       # 启用授时服务
  --heartbeat-interval 30000     # 心跳间隔（毫秒）
```

### 聊天客户端功能

启动聊天客户端后：

1. **发送消息**: 直接输入文本并按回车
2. **退出聊天**: 输入 `quit` 或 `exit`
3. **查看状态**: 客户端会显示连接状态和消息发送结果

```
================== 聊天室: general ==================
[Alice] > 大家好！
✓ 消息已发送 (ID: 123e4567-e89b-12d3-a456-426614174000)
[Alice] > 今天天气不错
✓ 消息已发送 (ID: 456e7890-e89b-12d3-a456-426614174001)
[Alice] > quit
```

### 授时客户端功能

授时客户端会：

1. **显示当前时间信息**
2. **定期向服务器请求时间同步**
3. **显示同步统计信息**
4. **计算时间偏差和网络延迟**

## 🛠️ 开发指南

### 添加新的业务模块

1. **创建新的 crate**:
```bash
mkdir crates/my-module
cargo init crates/my-module --lib
```

2. **实现业务逻辑**:
```rust
use network_service::{NetworkServiceTrait, MessageHandler};

pub struct MyService<N: NetworkServiceTrait> {
    network_service: N,
}

impl<N: NetworkServiceTrait> MyService<N> {
    pub fn new(network_service: N) -> Self {
        Self { network_service }
    }
}
```

3. **创建消息处理器**:
```rust
use async_trait::async_trait;
use network_service::{MessageHandler, NetworkMessage, NodeId};

pub struct MyMessageHandler {
    // 处理器状态
}

#[async_trait]
impl MessageHandler for MyMessageHandler {
    async fn handle_message(&self, from: NodeId, message: NetworkMessage) -> Result<Option<NetworkMessage>> {
        // 处理消息逻辑
        Ok(None)
    }
}
```

4. **在主程序中注册**:
```rust
let my_service = Arc::new(MyService::new(network_service.clone()));
let my_handler = MyMessageHandler::new(my_service);

network_service
    .register_message_handler(MessageType::new("my_message"), Box::new(my_handler))
    .await?;
```

### 替换网络底层实现

要替换 Anemo 为其他网络库：

1. **实现 `NetworkServiceTrait`**:
```rust
pub struct MyNetworkService {
    // 实现细节
}

#[async_trait]
impl NetworkServiceTrait for MyNetworkService {
    async fn start(&self, config: NetworkServiceConfig) -> Result<()> {
        // 启动网络服务
    }
    
    async fn broadcast(&self, message: NetworkMessage, options: Option<BroadcastOptions>) -> Result<MessageId> {
        // 实现广播
    }
    
    // 其他方法...
}
```

2. **更新主程序**:
```rust
let network_service = MyNetworkService::new();
```

### 自定义消息类型

1. **定义消息结构**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MyMessageType {
    Request { data: String },
    Response { result: i32 },
}
```

2. **创建网络消息**:
```rust
let my_msg = MyMessageType::Request { data: "test".to_string() };
let payload = serde_json::to_value(&my_msg)?;
let network_msg = NetworkMessage::new(
    MessageType::new("my_message"),
    sender_id,
    payload,
);
```

## 📊 性能和监控

### 网络统计

服务会自动收集以下统计信息：

- **消息发送/接收计数**
- **网络延迟测量**
- **连接状态监控**
- **错误率统计**

### 日志系统

使用 `tracing` 框架提供结构化日志：

```bash
# 启用调试日志
RUST_LOG=debug cargo run -- server

# 启用特定模块的日志
RUST_LOG=network_service=debug,chat_module=info cargo run -- server
```

## 🧪 测试

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定模块测试
cargo test -p network-service
cargo test -p chat-module
cargo test -p timesync-module

# 运行集成测试
cargo test --test integration
```

### 测试覆盖率

```bash
# 安装 tarpaulin (如果还没有安装)
cargo install cargo-tarpaulin

# 生成测试覆盖率报告
cargo tarpaulin --out html
```

## 🐛 故障排除

### 常见问题

1. **编译错误**
   ```bash
   cargo clean
   cargo build
   ```

2. **网络连接问题**
   - 检查防火墙设置
   - 确认端口未被占用
   - 查看网络权限

3. **消息发送失败**
   - 确认服务器已启动
   - 检查网络连接状态
   - 查看错误日志

### 调试技巧

1. **启用详细日志**:
   ```bash
   RUST_LOG=trace cargo run -- server
   ```

2. **使用网络工具**:
   ```bash
   # 检查端口占用
   netstat -tulpn | grep 8080
   
   # 测试网络连接
   telnet 127.0.0.1 8080
   ```

3. **查看进程状态**:
   ```bash
   ps aux | grep anemo-example
   ```

## 🤝 贡献指南

我们欢迎贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启 Pull Request

### 代码规范

- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查代码质量
- 编写单元测试和文档
- 遵循 Rust API 设计指南

## 📄 许可证

本项目基于 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Anemo](https://github.com/mystenlabs/anemo) - 优秀的 Rust 网络库
- [Tokio](https://tokio.rs/) - 异步运行时
- [Tracing](https://tracing.rs/) - 结构化日志
- [Clap](https://clap.rs/) - 命令行解析

## 📚 更多资源

- [Anemo 文档](https://docs.rs/anemo/)
- [Rust 异步编程指南](https://rust-lang.github.io/async-book/)
- [网络编程最佳实践](https://doc.rust-lang.org/book/ch20-00-final-project-a-web-server.html)

---

如果您有任何问题或建议，请创建 [Issue](https://github.com/your-repo/anemo-example/issues) 或联系维护者。 