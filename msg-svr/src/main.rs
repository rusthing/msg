use anyhow::anyhow;
use config::Value;
use msg_svr::cache::setup_cache;
use msg_svr::config::{AppConfig, set_msg_config};
use msg_svr::sub::setup_dead_letter_subscription;
use msg_svr::utils::setup_queue_refresh;
use robotech::app::setup_id_worker;
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

    // 业务配置先落到全局：订阅参数（重投上限）在建立订阅时读取，
    // 必须早于死信订阅与业务队列订阅
    set_msg_config(app_config.msg.clone());

    setup_id_worker(app_config.id_worker.clone(), changed).await?;
    setup_db_conn(app_config.db.clone(), changed).await?;

    // NATS 消息中间件连接
    setup_nats_client(app_config.nats.clone(), changed).await?;

    // 死信订阅：消费 dlq.> 并落库，必须先于业务队列就绪
    setup_dead_letter_subscription().await?;

    setup_web_server(app_config.web.clone(), port, old_pid, changed).await?;

    // 消息缓存：注册刷新信号订阅 + 启动后首次全量加载
    setup_cache().await?;

    // 队列刷新：注册刷新信号订阅 + 入口队列保底 + 首次同步订阅
    setup_queue_refresh().await?;


    Ok(())
}