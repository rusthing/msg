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
├── msg-pub/                # 发布方 SDK（事件发布与配置）
└── msg-web/                # 管理控制台（React + Vite + antd）
    └── src/
        ├── api/            # HTTP 客户端与通用 CRUD 封装
        ├── entities/       # 14 张表的字段元数据（表格/表单/筛选的唯一来源）
        ├── pages/          # 总览与通用 CRUD 页
        ├── dic/            # 字典（投递状态等 `#[dic]` 枚举）
        ├── hooks/          # 分页、异步、外键选项
        └── layouts/        # 应用骨架
```

## 管理控制台（msg-web）

14 张表都由 `#[router(crud)]` 生成形状一致的接口，所以前端没有 14 个重复页面：
字段与控件的对应关系集中在 `src/entities/index.tsx`，由 `src/pages/EntityPage.tsx` 统一渲染。

```bash
cd msg-web
pnpm install
pnpm dev        # http://localhost:5175，/msg/** 反代到 msg-svr:9002
pnpm build      # 产物在 dist/，部署到 /msg-web/ 子路径
```

后端地址与用户态来自 `.env.development` / `.env.production`：
`VITE_PROXY_TARGET` 指向 `msg-svr` 的 `web.port`，`VITE_API_BASE` 是网关上的服务前缀
（生产态 `VITE_API_BASE=/msg-svr`、`VITE_PUBLIC_BASE=/msg-web`，两者同源部署，不需要 CORS）。

## 前端与后端的三个约定

1. `u64` 在 JSON 里是**字符串**（`id`、`businessId`、各种 `*Ms`），前端不做 `Number()` 转换；
2. 字典枚举（`#[dic]`）线上是**整数判别式**，文案表在 `src/dic`；
3. 查询条件**全是等值匹配**，模糊搜索只有 `_keyword`；分页参数是 `_page` / `_size`。
