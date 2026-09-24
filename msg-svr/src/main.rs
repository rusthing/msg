use anyhow::anyhow;
use config::Value;
use idworker::setup_id_worker;
use msg_svr::config::AppConfig;
use msg_svr::svc::setup_msg_cache;
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

    // 消息缓存：DB 重连后自动热更新
    setup_msg_cache(changed).await;

    setup_web_server(app_config.web.clone(), port, old_pid, changed).await?;

    Ok(())
}