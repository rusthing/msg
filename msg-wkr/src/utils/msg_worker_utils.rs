use async_trait::async_trait;
use crate::config::get_msg_worker_config;
use tracing::info;

/// 消息处理 trait
///
/// Worker 从 Redis Stream 收到消息后，调用此 trait 进行处理。
/// 下游客户端需实现此 trait 来完成实际的消息处理逻辑。
#[async_trait]
pub trait MsgMessageHandler: Send + Sync {
    /// 处理消息
    async fn handle(&self, message: &str) -> Result<(), Box<dyn std::error::Error>>;
}

/// 初始化 MSG Worker，启动消息监听
pub async fn setup_msg_worker(
    _handler: std::sync::Arc<dyn MsgMessageHandler>,
) -> Result<(), Box<dyn std::error::Error>> {
    let worker_config = get_msg_worker_config()?;
    info!(
        "MSG Worker initialized: code={}, key={}",
        worker_config.worker_code, worker_config.worker_key
    );
    // TODO: 实现 Worker 初始化逻辑，从 Redis Stream 订阅消息并分发给 handler
    Ok(())
}