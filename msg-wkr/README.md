# msg-wkr

MSG 消息 Worker 客户端库，为下游 Worker 应用提供简化的消息处理工具。

## 功能

- **Redis Stream 订阅** - 通过 Redis Stream 接收服务端分发的消息
- **消息处理** - 提供统一的消息处理接口
- **状态上报** - 支持上报消息处理状态
- **配置热更新** - 运行时自动检测并重载配置变更
- **消费者组管理** - 自动创建和管理 Redis 消费者组

## 模块结构

```
src/
├── config/
│   ├── msg_worker_config.rs    # Worker 配置
│   └── mod.rs
├── utils/
│   ├── msg_worker_utils.rs     # Worker 核心工具函数
│   └── mod.rs
└── lib.rs                      # 库入口
```

## 使用

### 添加依赖

```toml
[dependencies]
msg-wkr = "1.0.0"
```

### 实现消息处理器

```rust
use async_trait::async_trait;
use msg_wkr::MsgMessageHandler;

struct MyHandler;

#[async_trait]
impl MsgMessageHandler for MyHandler {
    async fn handle(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 处理消息逻辑
        println!("Processing message: {}", message);
        Ok(())
    }
}
```