use anyhow::anyhow;
use clap::Parser;
use config::Value;
use idworker::setup_id_worker;
use msg_svr::config::AppConfig;
use robotech;
use robotech::app::{wait_app_exit, AppWatcher};
use robotech::dao::init_dao;
use robotech::db::setup_db_conn;
use robotech::env::init_env;
use robotech::log::LogWatcher;
use robotech::macros::{db_migrate, log_call};
use robotech::micro_svc::{drop_hub_client, register_micro_svc};
use robotech::redis::setup_redis_conn;
use robotech::signal::SignalManager;
use robotech::web::{setup_web_server, stop_web_service};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// msg - 通用消息中心
///
/// SUMMARY: msg-svr 是一个通用消息中心服务，提供消息的创建、发送、追踪等功能。
/// 支持多种消息通道，通过 RESTful API 接口提供服务，支持 HTTP 和 HTTPS 协议。
///
#[derive(Parser, Debug, Clone)]
#[command(
    author = env!("CARGO_PKG_AUTHORS"),
    version,
    about,
    help_template = "{name} v{version} - {about}\n\nAUTHOR: {author}\n\nUSAGE: {usage}\n\nOPTIONS:\n{options}"
)]
struct Args {
    /// 配置文件的路径
    #[arg(short, long)]
    config_file: Option<String>,

    /// Web服务器的端口号
    #[arg(short, long)]
    port: Option<u16>,

    /// 监听信号
    #[arg(
        short,
        long,
        default_value = "start",
        long_help = r#"监听信号，支持指令如下:
    start - 默认值，先发送 SIGCONT 信号(kill -0)，检查程序是否已运行，然后启动程序
    restart - 重启程序，向已运行的程序发送 SIGUSR2 信号
    stop/s - 停止程序，向已运行的程序发送 SIGTERM 信号
    kill/k - 强制停止程序，向已运行的程序发送 SIGKILL 信号
    "#
    )]
    signal: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数
    let Args {
        signal,
        config_file: config_file_path,
        port,
    } = Args::parse();

    // 初始化环境变量
    init_env()?;
    let log_watcher = LogWatcher::new().await?;
    init_dao()?;

    let (mut signal_manager, old_pid) = SignalManager::new(signal)?;

    let app_watcher: AppWatcher<AppConfig> = AppWatcher::new(
        config_file_path,
        log_watcher.config_changed_tx.clone(),
        move |app_config: Arc<AppConfig>, changed| async move {
            let changed = Some(changed);
            setup(&app_config, &changed, port, old_pid).await?;
            info!("重新加载配置成功");
            Ok(())
        },
    )
    .await?;

    let changed = None;
    setup(&app_watcher.app_config, &changed, port, old_pid).await?;

    register_micro_svc().await;

    let signal_receiver = signal_manager.watch_signal()?;
    Ok(wait_app_exit(signal_receiver, || async move {
        drop_hub_client().await;
        stop_web_service().await.expect("无法停止旧的Web服务");
        Ok(())
    })
    .await?)
}

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

    setup_id_worker(app_config.id_worker.clone(), &changed)?;
    setup_db_conn(app_config.db.clone(), &changed).await?;

    setup_web_server(app_config.web.clone(), port, old_pid, &changed).await?;

    Ok(())
}
