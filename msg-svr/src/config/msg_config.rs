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
//! ```
//!
//! 配置在 `setup()` 入口写入全局，建立 NATS 订阅时读取：
//!
//! - 死信订阅每次 `setup()` 都会重建，改配置下一轮即生效；
//! - 业务队列订阅按「已应用的重投上限」比对，值不一致就重建订阅，
//!   热更新同样无需重启进程。

use arc_swap::ArcSwapOption;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

/// 业务队列重投上限默认值
const DEFAULT_MAX_DELIVER: i64 = 5;

/// 死信队列重投上限默认值
const DEFAULT_DEAD_LETTER_MAX_DELIVER: i64 = 3;

/// 重投上限下限
///
/// NATS 用 `-1` 表示「不限制」，`0` 在服务端是非法值；这里统一按下限 1 处理，
/// 避免配置笔误把订阅变成无限重投（毒消息会在 stream 里反复刷）。
const MIN_MAX_DELIVER: i64 = 1;

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
}

impl Default for MsgConfig {
    fn default() -> Self {
        Self {
            max_deliver: DEFAULT_MAX_DELIVER,
            dead_letter_max_deliver: DEFAULT_DEAD_LETTER_MAX_DELIVER,
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
}
