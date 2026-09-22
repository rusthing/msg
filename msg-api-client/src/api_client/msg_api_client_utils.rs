use robotech::api_client::ApiClientConfig;
use std::collections::HashMap;

pub const MSG_API_CLIENT_CONFIG_KEY: &str = "msg.api-client";

pub fn setup_msg_api_client(
    _config: &HashMap<String, ApiClientConfig>,
) -> Result<(), anyhow::Error> {
    // TODO: 实现 MSG API 客户端初始化逻辑
    Ok(())
}