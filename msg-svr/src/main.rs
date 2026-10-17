use anyhow::anyhow;
use config::Value;
use idworker::setup_id_worker;
use msg_api::cst::{MSG_CACHE_REFRESH_SUBJECT, MSG_QUEUE_CACHE_REFRESH_SUBJECT};
use msg_svr::cache::{MSG_CACHE, MSG_QUEUE_CACHE, refresh_msg_cache, refresh_queue_cache};
use msg_svr::config::AppConfig;
use robotech::cache;
use robotech::db::setup_db_conn;
use robotech::macros::{bootstrap, db_migrate, log_call};
use robotech::mq::nats::setup_nats_client;
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

    // 消息缓存：安装 JetStream 刷新信号消费者（对齐 dtw-rs 本地缓存规范）
    {
        cache::setup_local_cache(MSG_CACHE_REFRESH_SUBJECT, &MSG_CACHE, || async {
            refresh_msg_cache().await
        })
        .await?;

        cache::setup_local_cache(MSG_QUEUE_CACHE_REFRESH_SUBJECT, &MSG_QUEUE_CACHE, || async {
            refresh_queue_cache().await
        })
        .await?;

        // 启动后首次全量加载
        refresh_msg_cache().await?;
        refresh_queue_cache().await?;
    }

    setup_web_server(app_config.web.clone(), port, old_pid, changed).await?;

    Ok(())
}