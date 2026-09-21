# msg-api-client

MSG 服务的 Rust API 客户端库，基于 [robotech](https://github.com/rusthing/robotech-rs) 的 Feign 客户端实现，支持通过服务发现或直连方式调用远程 MSG 服务。

## 功能

- 完整的 MSG RESTful API 客户端封装
- 支持 Consul 服务发现自动寻址
- 支持配置热更新，无需重启即可切换服务地址

## 模块结构

```
src/
├── api_client/
│   ├── msg_api_client_utils.rs    # 客户端初始化与配置工具
│   └── mod.rs
└── lib.rs
```

## 使用

### 添加依赖

```toml
[dependencies]
msg-api-client = "1.0.0"
```

### 初始化客户端

```rust
use msg_api_client::api_client::{setup_msg_api_client, get_msg_api_client};
use robotech::api_client::ApiClientConfig;
use std::collections::HashMap;

// 配置 API 客户端
let mut apis_config = HashMap::new();
apis_config.insert(
    "msg".to_string(),
    ApiClientConfig {
        base_url: "http://localhost:8080".to_string(),
        ..Default::default()
    },
);

let _ = setup_msg_api_client(&apis_config).await;
let client = get_msg_api_client().unwrap();
```