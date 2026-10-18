use anyhow::anyhow;
use config::Value;
use idworker::setup_id_worker;
use msg_api::cst::{MSG_MESSAGE_REFRESH_SUBJECT, MSG_QUEUE_REFRESH_SUBJECT};
use msg_svr::cache::{MSG_MESSAGE_CACHE, refresh_message_cache};
use msg_svr::config::AppConfig;
use msg_svr::sub::reload_queue_subscriptions;
use robotech::cache;
use robotech::db::setup_db_conn;
use robotech::macros::{bootstrap, db_migrate, log_call};
use robotech::mq::nats::setup_nats_client;
use robotech::refresh;
use robotech::web::setup_web_server;
use std::collections::HashMap;
use std::sync::Arc;

bootstrap!(AppConfig);

/// # 初始化或更新应用配置
#[log_call]
async fn setup(
    app_config: &Arc<AppConfig>,
    changed: &Option<HashMap<String, Value>>,
    port: Option<u16>,
    old_pid: Option<u32>,
) -> Result<(), anyhow::Error> {
    let db_url = app_config.db.get_url();
    db_migrate!(db_url);

    setup_id_worker(app_config.id_worker.clone(), changed)?;
    setup_db_conn(app_config.db.clone(), changed).await?;

    // NATS 消息中间件连接
    if let Some(ref nats_config) = app_config.nats {
        setup_nats_client(nats_config.clone(), changed).await?;
    }

    // 消息缓存：收到刷新信号后全量重建缓存快照
    // 队列订阅：收到刷新信号后直接查库并同步 NATS 订阅（不经过缓存）
    {
        cache::setup_local_cache(&MSG_MESSAGE_REFRESH_SUBJECT, &MSG_MESSAGE_CACHE, || async {
            refresh_message_cache().await
        })
        .await?;

        refresh::subscribe(&MSG_QUEUE_REFRESH_SUBJECT, || async {
            reload_queue_subscriptions().await
        })
        .await?;

        // 启动后首次全量加载：消息模板必须先就绪，订阅到消息时才有模板可用
        refresh_message_cache().await?;
        reload_queue_subscriptions().await?;
    }

    setup_web_server(app_config.web.clone(), port, old_pid, changed).await?;

    Ok(())
}