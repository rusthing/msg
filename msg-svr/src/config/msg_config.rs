//! # MSG 业务配置
//!
//! 对应配置文件中的 `[msg]` 段，整段缺省时全部取默认值：
//!
//! ```toml
//! [msg]
//! # 业务队列（persisted = true）消费失败的最大重投次数，达到上限后转入死信
//! max-deliver = 5
//! # 死信队列自身落库失败的最大重投次数
//! dead-letter-max-deliver = 3
//! # 模板里 `{key:time}` 使用的时区偏移
//! template-time-offset = "+08:00"
//!
//! [msg.ws]
//! # 大屏长连接的挂载路径
//! path = "/msg/ws"
//! # 心跳间隔（秒）
//! ping-interval-secs = 30
//! # 每条连接的待发队列长度，满了丢新消息
//! send-queue-size = 128
//! ```
//!
//! 配置在 `setup()` 入口写入全局，建立 NATS 订阅时读取：
//!
//! - 死信订阅每次 `setup()` 都会重建，改配置下一轮即生效；
//! - 业务队列订阅按「已应用的重投上限」比对，值不一致就重建订阅，
//!   热更新同样无需重启进程。

use arc_swap::ArcSwapOption;
use chrono::FixedOffset;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

/// 业务队列重投上限默认值
const DEFAULT_MAX_DELIVER: i64 = 5;

/// 死信队列重投上限默认值
const DEFAULT_DEAD_LETTER_MAX_DELIVER: i64 = 3;

/// 模板时间戳的默认时区偏移
///
/// 取东八区而不是 `chrono::Local`：容器里的 `TZ` 通常是 UTC，靠环境变量才能
/// 显示成本地时间太脆；需要别的时区改配置即可。
const DEFAULT_TEMPLATE_TIME_OFFSET: &str = "+08:00";

/// 重投上限下限
///
/// NATS 用 `-1` 表示「不限制」，`0` 在服务端是非法值；这里统一按下限 1 处理，
/// 避免配置笔误把订阅变成无限重投（毒消息会在 stream 里反复刷）。
const MIN_MAX_DELIVER: i64 = 1;

/// 大屏长连接默认挂载路径
const DEFAULT_WS_PATH: &str = "/msg/ws";
/// 心跳默认间隔（秒）
const DEFAULT_WS_PING_INTERVAL_SECS: u64 = 30;
/// 单连接默认待发队列长度
const DEFAULT_WS_SEND_QUEUE_SIZE: usize = 128;

/// # MSG 业务配置
///
/// 字段缺省时取 [`Default`] 的取值，因此配置文件里只写关心的字段即可。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct MsgConfig {
    /// 业务队列消费失败的最大重投次数
    ///
    /// 超过该次数说明消息本身有问题，由 robotech 的订阅层转入死信
    /// `dlq.{subject}`，再由死信订阅落库到 `msg_dead_letter` 供人工排查。
    pub max_deliver: i64,
    /// 死信队列落库失败的最大重投次数
    ///
    /// 死信消费只是写一张表，重试几次仍失败基本说明表/库本身有问题，
    /// 再投也没有意义，所以默认值比业务队列小。
    pub dead_letter_max_deliver: i64,
    /// 模板里 `{key:time}` 使用的时区偏移
    ///
    /// 形如 `+08:00`（也接受 `+0800`）。固定偏移不带夏令时，中国没有夏令时，
    /// 够用；真要跟夏令时国家对齐，就填它当下的偏移。解析失败按默认值处理。
    pub template_time_offset: String,
    /// websocket 渠道（大屏长连接）配置
    pub ws: MsgWsConfig,
}

impl Default for MsgConfig {
    fn default() -> Self {
        Self {
            max_deliver: DEFAULT_MAX_DELIVER,
            dead_letter_max_deliver: DEFAULT_DEAD_LETTER_MAX_DELIVER,
            template_time_offset: DEFAULT_TEMPLATE_TIME_OFFSET.to_string(),
            ws: MsgWsConfig::default(),
        }
    }
}

/// # websocket 渠道配置
///
/// 只放连接本身的参数。握手用的 token 不在这里——它是渠道的配置，
/// 存在 `msg_channel.options`（`{"token":"..."}`），运营改渠道表即可换。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct MsgWsConfig {
    /// 长连接挂载路径
    ///
    /// 在 Web 服务器启动时注册，改这个值需要重启（Web 路由不会因热更新重建）。
    pub path: String,
    /// 心跳间隔（秒）
    ///
    /// 服务端按这个间隔发 Ping；写失败说明连接已断，直接关闭并清理。
    pub ping_interval_secs: u64,
    /// 每条连接的待发队列长度
    ///
    /// 大屏消费不过来时队列会堆积，满了就丢新消息（大屏只要最新状态，
    /// 不追求一条不漏），不能让一个慢连接拖住整个投递线程。
    pub send_queue_size: usize,
}

impl Default for MsgWsConfig {
    fn default() -> Self {
        Self {
            path: DEFAULT_WS_PATH.to_string(),
            ping_interval_secs: DEFAULT_WS_PING_INTERVAL_SECS,
            send_queue_size: DEFAULT_WS_SEND_QUEUE_SIZE,
        }
    }
}

impl MsgConfig {
    /// 实际生效的业务队列重投上限（低于下限时按下限返回）
    pub fn effective_max_deliver(&self) -> i64 {
        normalize_max_deliver("msg.max-deliver", self.max_deliver)
    }

    /// 实际生效的死信队列重投上限（低于下限时按下限返回）
    pub fn effective_dead_letter_max_deliver(&self) -> i64 {
        normalize_max_deliver("msg.dead-letter-max-deliver", self.dead_letter_max_deliver)
    }

    /// 实际生效的模板时区偏移（解析失败时回落默认值）
    ///
    /// 渲染模板时每条消息都会调一次：配置热更新后下一次渲染即生效。
    pub fn effective_time_offset(&self) -> FixedOffset {
        match self.template_time_offset.parse::<FixedOffset>() {
            Ok(offset) => offset,
            Err(_) => {
                warn!(
                    "msg.template-time-offset={} 不是合法的时区偏移（形如 +08:00），按 {} 处理",
                    self.template_time_offset, DEFAULT_TEMPLATE_TIME_OFFSET
                );
                // 常量写死在这里，解析一定成功
                DEFAULT_TEMPLATE_TIME_OFFSET
                    .parse()
                    .unwrap_or_else(|_| FixedOffset::east_opt(8 * 3600).expect("东八区偏移合法"))
            }
        }
    }
}

/// 全局 MSG 业务配置
///
/// 由 `setup()` 入口写入，订阅建立时读取；未加载前取默认值。
static MSG_CONFIG: ArcSwapOption<MsgConfig> = ArcSwapOption::const_empty();

/// # 设置 MSG 业务配置
///
/// 应用配置加载与热更新时调用（每次 `setup()` 都会走到）。
pub fn set_msg_config(config: MsgConfig) {
    MSG_CONFIG.store(Some(Arc::new(config)));
}

/// # 获取 MSG 业务配置
///
/// 尚未加载配置（如单元测试）时返回默认值。
pub fn get_msg_config() -> Arc<MsgConfig> {
    MSG_CONFIG
        .load_full()
        .unwrap_or_else(|| Arc::new(MsgConfig::default()))
}

/// 把重投上限收敛到合法区间，越界时打 warn 并按边界处理
fn normalize_max_deliver(key: &str, value: i64) -> i64 {
    if value < MIN_MAX_DELIVER {
        warn!("{key}={value} 小于下限 {MIN_MAX_DELIVER}，按 {MIN_MAX_DELIVER} 处理");
        MIN_MAX_DELIVER
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(toml_str: &str) -> MsgConfig {
        ::config::Config::builder()
            .add_source(::config::File::from_str(
                toml_str,
                ::config::FileFormat::Toml,
            ))
            .build()
            .unwrap()
            .get("msg")
            .unwrap()
    }

    #[test]
    fn default_values() {
        let config = MsgConfig::default();
        assert_eq!(config.max_deliver, 5);
        assert_eq!(config.dead_letter_max_deliver, 3);
        // 只配段名、不配字段时取默认值
        assert_eq!(parse("[msg]").max_deliver, 5);
    }

    #[test]
    fn fields_are_independent() {
        let config = parse("[msg]\nmax-deliver = 8\n");
        assert_eq!(config.max_deliver, 8);
        assert_eq!(config.dead_letter_max_deliver, 3);
    }

    #[test]
    fn max_deliver_has_lower_bound() {
        let config = parse("[msg]\nmax-deliver = 0\ndead-letter-max-deliver = -1\n");
        assert_eq!(config.effective_max_deliver(), MIN_MAX_DELIVER);
        assert_eq!(config.effective_dead_letter_max_deliver(), MIN_MAX_DELIVER);
    }

    #[test]
    fn template_time_offset_defaults_and_parses() {
        // 不配时取东八区
        let config = parse("[msg]");
        assert_eq!(config.template_time_offset, "+08:00");
        assert_eq!(
            config.effective_time_offset(),
            FixedOffset::east_opt(8 * 3600).unwrap()
        );

        let config = parse("[msg]\ntemplate-time-offset = \"+00:00\"\n");
        assert_eq!(
            config.effective_time_offset(),
            FixedOffset::east_opt(0).unwrap()
        );

        // 填了 IANA 名这种非法值：回落默认，不影响渲染
        let config = parse("[msg]\ntemplate-time-offset = \"Asia/Shanghai\"\n");
        assert_eq!(
            config.effective_time_offset(),
            FixedOffset::east_opt(8 * 3600).unwrap()
        );
    }

    #[test]
    fn ws_defaults() {
        // 整段不写时走默认值，只写一个字段时其余仍回落默认
        let config = parse("[msg]");
        assert_eq!(config.ws.path, "/msg/ws");
        assert_eq!(config.ws.ping_interval_secs, 30);
        assert_eq!(config.ws.send_queue_size, 128);

        let config = parse("[msg.ws]\npath = \"/screen/ws\"\n");
        assert_eq!(config.ws.path, "/screen/ws");
        assert_eq!(config.ws.ping_interval_secs, 30);
    }
}
