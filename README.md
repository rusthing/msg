# MSG

通用消息中心，基于 Rust 生态构建的消息服务系统。

## 项目结构

```
msg/
├── Cargo.toml              # 工作区配置
├── msg-api/                # 共享数据类型 (DTO / VO / MO)
│   └── src/
│       ├── dto/            # 数据传输对象 (Data Transfer Object)
│       ├── vo/             # 视图对象 (View Object)
│       └── mo/             # 模型对象 (Model Object, SeaORM 实体)
├── msg-api-client/         # API 客户端
│   └── src/
│       └── api_client/     # 消息相关 API 客户端
├── msg-svr/                # 服务端应用
│   ├── migrations/         # 数据库迁移文件
│   │   ├── mysql/
│   │   ├── pgsql/
│   │   └── sqlite/
│   └── src/
│       ├── config/         # 应用配置
│       ├── dao/            # 数据访问层
│       ├── svc/            # 业务逻辑层
│       ├── web/            # Web 层 (路由、控制器、API 文档)
│       └── utils/          # 工具函数
└── msg-wkr/                # Worker 客户端库
    └── src/
        ├── config/         # Worker 配置
        └── utils/          # Worker 工具函数
```