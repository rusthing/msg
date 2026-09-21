# msg-svr

MSG 服务端应用，基于 Rust 生态构建的通用消息中心服务系统。

## 功能特性

- **消息管理** - 消息的 CRUD 操作，支持模板、通道和优先级
- **消息发送** - 支持多种消息通道（如邮件、短信、站内信、Webhook 等）
- **消息追踪** - 消息状态跟踪与发送结果记录
- **RESTful API** - 基于 Axum 的 HTTP API，自动生成 CRUD 接口
- **Swagger UI** - 集成 OpenAPI 文档，方便调试
- **配置热更新** - 运行时自动检测并重载配置变更
- **信号控制** - 支持 start / restart / stop / kill 信号
- **服务注册** - 支持 Consul 服务注册与配置中心
- **多数据库** - 支持 PostgreSQL / MySQL / SQLite

## 项目结构

```
src/
├── config/              # 应用配置
│   ├── app_config.rs    # 应用配置结构体
│   ├── msg_config.rs    # MSG 业务配置
│   └── mod.rs
├── dao/                 # 数据访问层 (基于 SeaORM)
│   └── mod.rs
├── svc/                 # 业务逻辑层
│   └── mod.rs
├── web/                 # Web 层
│   ├── api_doc/         # Swagger 文档定义
│   ├── ctrl/            # 控制器 (请求处理)
│   ├── router/          # 路由定义
│   └── mod.rs
├── lib.rs               # 库入口
└── main.rs              # 应用入口
```

## 快速开始

### 配置文件

```toml
# msg-svr.toml
profile = "dev"

[db]
url = "postgres://msg:msg@pgsql:5432/msg"

[web]
port = 9002

[redis]
url = "redis://:xxxxxxxx@redis:6379"
```

### 运行

```bash
cargo run -- -c msg-svr.toml
```