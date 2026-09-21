# msg-api

MSG 项目的共享数据类型库，为 `msg-svr`（服务端）和 `msg-api-client`（客户端）提供统一的 DTO、VO 和 MO 类型定义。

## 功能

- **DTO** (Data Transfer Object) - 数据传输对象，用于 API 请求参数
- **VO** (View Object) - 视图对象，用于 API 响应数据
- **MO** (Model Object) - SeaORM 数据库实体模型 (需开启 `server` feature)

## 模块结构

```
src/
├── dto/          # 请求参数定义
├── vo/           # 响应数据定义
├── mo/           # 数据库实体 (仅 server feature)
└── lib.rs
```

## Features

| Feature | 说明 | 引入的依赖 |
|---------|------|-----------|
| `server` | 启用服务端相关功能，包括 SeaORM 实体、对象映射和验证 | `sea-orm`, `o2o`, `validator` |

## 使用

### 作为客户端依赖 (默认)

```toml
[dependencies]
msg-api = "1.0.0"
```

此时仅包含 `dto` 和 `vo` 模块，适用于 API 客户端场景。

### 作为服务端依赖

```toml
[dependencies]
msg-api = { version = "1.0.0", features = ["server"] }
```

此时额外包含 `mo` 模块和 SeaORM 相关功能，适用于服务端场景。