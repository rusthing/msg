use async_trait::async_trait;

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
    // TODO: 实现 Worker 初始化逻辑
    Ok(())
}