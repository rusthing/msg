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
    stop - 停止程序，向已运行的程序发送 SIGTERM 信号
    kill - 强制停止程序，向已运行的程序发送 SIGKILL 信号
    "#)]
    signal: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 初始化环境变量
    init_env();

    // 处理信号
    let signal_manager = SignalManager::new("msg-svr".to_string());
    if let Err(e) = signal_manager.handle_signal(args.signal.clone()).await {
        if args.signal != "start" {
            return Err(anyhow!("signal error: {}", e));
        }
    }

    // 加载配置
    let config_file = args.config_file.unwrap_or_else(|| {
        std::env::var("MSG_CONFIG_FILE").unwrap_or_else(|_| "msg-svr.toml".to_string())
    });
    let app_config = AppWatcher::<AppConfig>::new(&config_file)
        .expect("init config error")
        .get_config();

    info!("Starting msg-svr with config file: {}", config_file);

    // 初始化 ID Worker
    setup_id_worker(&app_config.id_worker)?;

    // 初始化数据库连接
    let db_conn = setup_db_conn(&app_config.db)
        .await
        .expect("setup db error");

    // 执行数据库迁移
    db_migrate!();

    // 初始化 DAO
    init_dao(db_conn).await;

    // 初始化 Redis 连接
    if let Some(redis_config) = &app_config.redis {
        setup_redis_conn(redis_config).await.expect("setup redis error");
    }

    // 注册微服务到 Consul
    if let Err(e) = register_micro_svc(&config_file).await {
        info!("Register micro service error (ignored): {}", e);
    }

    // 启动 Web 服务
    let web_server = setup_web_server(app_config.web.port).await?;

    // 初始化日志监听
    LogWatcher::init(&config_file)?;

    info!("MSG server started successfully on port {}", app_config.web.port);

    // 等待退出信号
    wait_app_exit().await;

    // 优雅关闭
    info!("Shutting down MSG server...");
    stop_web_service(web_server).await;
    drop_hub_client().await;

    info!("MSG server stopped");
    Ok(())
}