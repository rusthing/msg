//! # 投递目标展开与去重
//!
//! `msg_message_target` 里配的是**抽象目标**（用户 / 角色 / 组织 / 大屏 / 定位卡），
//! 这里把它展开成「一个人 + 一个地址」：
//!
//! ```text
//! user  目标  → 用户ID
//! role  目标  → 角色下的全部用户
//! org   目标  → 组织（含下级，tree_code 前缀）下的全部用户
//! locator 目标 → 卡号取自消息 labels（locatorCode / locatorCodes / cardCodes），
//!                配置里的 target_id 不参与；一张卡一条投递目标
//! screen（以及未知类别）→ 不展开，原样作为最终目标
//! ```
//!
//! 两条去重规则：
//!
//! 1. **人**：按用户 ID 去重，顺序 `user > role > org` 先到先得。同一人被多条
//!    目标命中时，只保留第一条命中的渠道配置，`origin` 把全部来源记下来
//!    （`user:2,role:2,org:1001`），便于回溯「这条投递是谁带来的」。
//!    落库时目标类别统一成 `user`、`target_id` 取用户 ID，所以 `msg_delivery_target`
//!    上原有的 `(delivery_id, target_category_id, target_id)` 唯一键就是人去重。
//! 2. **地址**：不同的人可能共用一个值班手机号 / 公共邮箱（source 侧唯一约束
//!    只管「一个人一个号」，管不住「多人同号」），因此再按 `(channel_id, 规范化地址)`
//!    去一次，先到先得，被抑制的渠道只打日志不建行。
//!
//! 展开结果为空（0 人）或某人的渠道都取不到地址时，不静默丢弃：目标行照样落库，
//! 状态记 `Skipped`，原因进日志，避免这类目标永远挂在「投递中」。
//!
//! ## 为什么直接查 Entity 而不是 Svc
//!
//! 展开要按 `Id in (...)` 批量取用户、按 `tree_code` 前缀取组织子树，Svc 的
//! QueryDto 只支持等值条件，一条消息会退化成 N 次往返。这里直接走 SeaORM 的
//! Entity 查询，全部批量，共 4～5 次往返。

use crate::cache::{ChannelCache, MessageCache, TargetCache};
use crate::mo::msg_org::Entity as OrgEntity;
use crate::mo::msg_org_user::{Column as OrgUserCol, Entity as OrgUserEntity};
use crate::mo::msg_person::{Column as PersonCol, Entity as PersonEntity};
use crate::mo::msg_role::{Column as RoleCol, Entity as RoleEntity};
use crate::mo::msg_role_user::{Column as RoleUserCol, Entity as RoleUserEntity};
use crate::mo::msg_user::{Column as UserCol, Entity as UserEntity};
use msg_api::cst::{ChannelCode, TARGET_ID_ALL};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::{HashMap, HashSet};
use tracing::warn;

/// 目标类别编码（`msg_target_category.code` 的种子值）
const TC_USER: &str = "user";
const TC_ROLE: &str = "role";
const TC_ORG: &str = "org";
/// 定位卡类别：卡号只认消息 labels，配置的 `target_id` 已废弃
const TC_LOCATOR: &str = "locator";
/// 定位卡卡号在 labels 中的键，按此顺序取第一个有值的
const LOCATOR_CARD_KEYS: [&str; 3] = ["cardCodes", "locatorCodes", "locatorCode"];
/// 单条消息最多展开多少张卡，与 dtw 报警标签的清单上限（32）对齐
const LOCATOR_MAX_CARDS: usize = 32;
/// `user` 类别的固定 ID（种子 id=1）
///
/// 消息只配了 `role` / `org` 目标时，缓存里没有 `user` 类别的行可借 ID，展开
/// 出来的人仍要落成 `user` 类别，这里用种子值兜底。种子没上线前就定死为 1，
/// 改动种子必须同步改这里。
const TC_USER_ID_FALLBACK: u64 = 1;

/// `origin` 列宽（`msg_delivery_target.origin`，varchar(200)）
const ORIGIN_MAX_CHARS: usize = 200;

/// # 渠道要的地址种类
///
/// 只有真正「打给某个人」的渠道需要地址；大屏、定位卡这类按连接 / 卡号路由的
/// 渠道不参与地址解析，展开时地址留空。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddressKind {
    /// 手机号：用户 `mobile` 优先，人员档案 `mobile` 兜底
    Mobile,
    /// 邮箱：用户 `email` 优先，人员档案 `email` 兜底
    Email,
}

impl AddressKind {
    /// 渠道类型 → 地址种类；`None` 表示该渠道不需要地址
    ///
    /// 只有短信 / 邮件要把消息投到个人，其余渠道按连接或卡号路由，展开时不解析地址。
    ///
    /// 按渠道类型判断，**不是**按渠道名或编码：短信 / 邮件要地址，大屏与 NATS
    /// 不要。渠道类型认不出来时当作不需要地址，正好与「这个类型没有发送器、
    /// 投递会报渠道未实现」一致。
    fn of_channel(channel_type: &str) -> Option<Self> {
        match ChannelCode::from_code(channel_type)? {
            ChannelCode::Sms => Some(Self::Mobile),
            ChannelCode::Email => Some(Self::Email),
            ChannelCode::Websocket | ChannelCode::Nats => None,
        }
    }
}

/// # 展开后的一个落地目标
#[derive(Debug, Clone)]
pub(crate) struct ExpandedTarget {
    /// 目标类别 ID（人一律 `user`；大屏 / 定位卡沿用原类别）
    pub category_id: u64,
    /// 目标类别编码（任务下发用）
    pub category_code: String,
    /// 最终目标标识：用户 ID / 大屏标识 / 卡号
    pub target_id: String,
    /// 命中的原始目标（`user:2,role:2,org:1001`），大屏 / 定位卡为空
    pub origin: Option<String>,
    /// 该目标可投的渠道（地址已解析好）
    pub channels: Vec<ExpandedChannel>,
}

/// # 展开后的一个渠道 + 地址
#[derive(Debug, Clone)]
pub(crate) struct ExpandedChannel {
    pub channel: ChannelCache,
    /// 收件地址；不走地址的渠道为 `None`
    pub address: Option<String>,
}

/// # 没展开出可投目标的原目标
///
/// 落 `msg_delivery_target` 时状态记 `Skipped`，保留「这次投过这个类别」的事实。
#[derive(Debug, Clone)]
pub(crate) struct SkippedTarget {
    pub category_id: u64,
    pub category_code: String,
    /// 原始目标标识（保持配置里的值，便于按配置反查）
    pub target_id: String,
    pub origin: Option<String>,
    /// 落库原因说明（日志与人工排查用，不进库）
    pub reason: String,
}

/// # 一次展开的全部结果
#[derive(Debug, Default)]
pub(crate) struct ExpandOutcome {
    /// 可以投的目标
    pub targets: Vec<ExpandedTarget>,
    /// 0 人或全部无地址的目标
    pub skipped: Vec<SkippedTarget>,
    /// 因地址重复被抑制的渠道数（只做观测）
    pub suppressed: usize,
}

/// # 一个人的命中记录（展开过程中的中间态）
struct Hit<'a> {
    /// 第一条命中它的目标配置的渠道
    channels: &'a [ChannelCache],
    /// 命中的原始目标链
    origins: Vec<String>,
}

/// # 从消息 labels 里取定位卡卡号
///
/// 取值优先级 `cardCodes` → `locatorCodes` → `locatorCode`：区域级报警同时带
/// 单卡与全量清单时，以全量清单为准；只有一个键就按单卡。逗号分隔，去空白、
/// 去重保序，最多 [`LOCATOR_MAX_CARDS`] 张（第二个返回值表示是否发生截断）。
fn locator_cards(labels: &HashMap<String, String>) -> (Vec<String>, bool) {
    let raw = LOCATOR_CARD_KEYS
        .iter()
        .find_map(|key| labels.get(*key).filter(|value| !value.trim().is_empty()))
        .map(String::as_str)
        .unwrap_or_default();

    let mut cards = Vec::new();
    let mut seen = HashSet::new();
    for card in raw.split(',') {
        let card = card.trim();
        if card.is_empty() || !seen.insert(card) {
            continue;
        }
        if cards.len() >= LOCATOR_MAX_CARDS {
            return (cards, true);
        }
        cards.push(card.to_string());
    }
    (cards, false)
}

/// # 展开消息的全部目标
///
/// 只读：全部查询都在调用方给的事务 / 连接上跑，不写库。失败返回 `Err`，
/// 调用方按「可重投」处理（查询失败不是配置问题，重投有意义）。
///
/// `labels` 是发布方随消息上报的结构化标签：定位卡目标的卡号只从这里取，
/// 与模板渲染用的是同一份数据。
pub(crate) async fn expand_targets<C>(
    db: &C,
    msg: &MessageCache,
    labels: &HashMap<String, String>,
) -> Result<ExpandOutcome, String>
where
    C: ConnectionTrait,
{
    let mut outcome = ExpandOutcome::default();

    // 落人用的类别：优先借缓存里已有的 user 目标 ID，没有就用种子值
    let user_cat_id = msg
        .targets
        .iter()
        .find(|t| t.target_category.code == TC_USER)
        .map(|t| t.target_category.id)
        .unwrap_or(TC_USER_ID_FALLBACK);

    // 先到先得：Vec 保序（决定投递顺序与「地址重复留谁」），HashMap 做索引
    let mut hit_order: Vec<u64> = Vec::new();
    let mut hits: HashMap<u64, Hit<'_>> = HashMap::new();

    // ── 1. user 目标：target_id 就是用户 ID ──
    for target in msg
        .targets
        .iter()
        .filter(|t| t.target_category.code == TC_USER)
    {
        match target.target_id.trim().parse::<u64>() {
            Ok(user_id) => add_hit(
                &mut hits,
                &mut hit_order,
                user_id,
                &target.channels,
                origin_of(&target.target_category.code, &target.target_id),
            ),
            Err(_) => outcome.skipped.push(SkippedTarget {
                category_id: target.target_category.id,
                category_code: target.target_category.code.clone(),
                target_id: target.target_id.clone(),
                origin: Some(origin_of(&target.target_category.code, &target.target_id)),
                reason: format!(
                    "用户目标的 target_id 不是合法用户ID（{:?}）",
                    target.target_id
                ),
            }),
        }
    }

    // ── 2. role 目标：角色下的全部用户 ──
    let role_targets: Vec<&TargetCache> = msg
        .targets
        .iter()
        .filter(|t| t.target_category.code == TC_ROLE)
        .collect();
    if !role_targets.is_empty() {
        let mut role_ids: Vec<i64> = Vec::new();
        for target in &role_targets {
            match target.target_id.trim().parse::<i64>() {
                Ok(id) => role_ids.push(id),
                Err(_) => outcome.skipped.push(SkippedTarget {
                    category_id: target.target_category.id,
                    category_code: target.target_category.code.clone(),
                    target_id: target.target_id.clone(),
                    origin: Some(origin_of(&target.target_category.code, &target.target_id)),
                    reason: format!(
                        "角色目标的 target_id 不是合法角色ID（{:?}）",
                        target.target_id
                    ),
                }),
            }
        }
        role_ids.sort_unstable();
        role_ids.dedup();

        let roles = RoleEntity::find()
            .filter(RoleCol::Id.is_in(role_ids.clone()))
            .all(db)
            .await
            .map_err(|e| format!("查询角色失败: {e}"))?;
        let role_enabled: HashMap<i64, bool> = roles.iter().map(|r| (r.id, r.enabled)).collect();

        // 不启用的角色整条跳过：配置里点过停用就是不想再发给它
        let active_role_ids: Vec<i64> = role_ids
            .iter()
            .copied()
            .filter(|id| role_enabled.get(id).copied().unwrap_or(false))
            .collect();
        for target in &role_targets {
            let Ok(id) = target.target_id.trim().parse::<i64>() else {
                continue;
            };
            if !role_enabled.contains_key(&id) {
                outcome.skipped.push(SkippedTarget {
                    category_id: target.target_category.id,
                    category_code: target.target_category.code.clone(),
                    target_id: target.target_id.clone(),
                    origin: Some(origin_of(&target.target_category.code, &target.target_id)),
                    reason: format!("角色 {id} 不在投影中（adm 未同步）"),
                });
            } else if !role_enabled[&id] {
                outcome.skipped.push(SkippedTarget {
                    category_id: target.target_category.id,
                    category_code: target.target_category.code.clone(),
                    target_id: target.target_id.clone(),
                    origin: Some(origin_of(&target.target_category.code, &target.target_id)),
                    reason: format!("角色 {id} 已停用"),
                });
            }
        }

        if !active_role_ids.is_empty() {
            let members = RoleUserEntity::find()
                .filter(RoleUserCol::RoleId.is_in(active_role_ids.clone()))
                .all(db)
                .await
                .map_err(|e| format!("查询角色成员失败: {e}"))?;

            // 归属到配置里的角色 ID（不是成员行），origin 才指向原始配置
            let mut member_by_role: HashMap<i64, Vec<i64>> = HashMap::new();
            for m in members {
                member_by_role.entry(m.role_id).or_default().push(m.user_id);
            }
            for target in &role_targets {
                let Ok(id) = target.target_id.trim().parse::<i64>() else {
                    continue;
                };
                let Some(users) = member_by_role.get(&id) else {
                    continue;
                };
                for user_id in users {
                    if *user_id < 0 {
                        continue;
                    }
                    add_hit(
                        &mut hits,
                        &mut hit_order,
                        *user_id as u64,
                        &target.channels,
                        origin_of(&target.target_category.code, &target.target_id),
                    );
                }
            }
        }
    }

    // ── 3. org 目标：组织及其下级 ──
    let org_targets: Vec<&TargetCache> = msg
        .targets
        .iter()
        .filter(|t| t.target_category.code == TC_ORG)
        .collect();
    if !org_targets.is_empty() {
        // 整棵组织树一次取回：禁用的连带（上级停用 → 子树都不投）必须在祖先链上
        // 判断，只查子树拿不到祖先的开关。
        let orgs = OrgEntity::find()
            .all(db)
            .await
            .map_err(|e| format!("查询组织树失败: {e}"))?;
        let disabled_codes: HashSet<&str> = orgs
            .iter()
            .filter(|o| !o.enabled)
            .map(|o| o.tree_code.as_str())
            .collect();

        let mut member_org_ids: Vec<i64> = Vec::new();
        for target in &org_targets {
            let Ok(id) = target.target_id.trim().parse::<i64>() else {
                outcome.skipped.push(SkippedTarget {
                    category_id: target.target_category.id,
                    category_code: target.target_category.code.clone(),
                    target_id: target.target_id.clone(),
                    origin: Some(origin_of(&target.target_category.code, &target.target_id)),
                    reason: format!(
                        "组织目标的 target_id 不是合法组织ID（{:?}）",
                        target.target_id
                    ),
                });
                continue;
            };
            let Some(org) = orgs.iter().find(|o| o.id == id) else {
                outcome.skipped.push(SkippedTarget {
                    category_id: target.target_category.id,
                    category_code: target.target_category.code.clone(),
                    target_id: target.target_id.clone(),
                    origin: Some(origin_of(&target.target_category.code, &target.target_id)),
                    reason: format!("组织 {id} 不在投影中（adm 未同步）"),
                });
                continue;
            };
            if blocked(&org.tree_code, &disabled_codes) {
                outcome.skipped.push(SkippedTarget {
                    category_id: target.target_category.id,
                    category_code: target.target_category.code.clone(),
                    target_id: target.target_id.clone(),
                    origin: Some(origin_of(&target.target_category.code, &target.target_id)),
                    reason: format!("组织 {id} 或其上级已停用"),
                });
                continue;
            }

            let parent_code = org.tree_code.clone();
            let target_orgs: Vec<i64> = orgs
                .iter()
                .filter(|o| o.tree_code.starts_with(&parent_code))
                .filter(|o| !blocked(&o.tree_code, &disabled_codes))
                .map(|o| o.id)
                .collect();
            if target_orgs.is_empty() {
                outcome.skipped.push(SkippedTarget {
                    category_id: target.target_category.id,
                    category_code: target.target_category.code.clone(),
                    target_id: target.target_id.clone(),
                    origin: Some(origin_of(&target.target_category.code, &target.target_id)),
                    reason: format!("组织 {id} 及其下级已全部停用"),
                });
                continue;
            }
            member_org_ids.extend(target_orgs);
        }

        member_org_ids.sort_unstable();
        member_org_ids.dedup();

        if !member_org_ids.is_empty() {
            let members = OrgUserEntity::find()
                .filter(OrgUserCol::OrgId.is_in(member_org_ids))
                .all(db)
                .await
                .map_err(|e| format!("查询组织成员失败: {e}"))?;

            // 成员行只记「哪个组织」，origin 要指回配置里的组织目标
            for target in &org_targets {
                let Ok(id) = target.target_id.trim().parse::<i64>() else {
                    continue;
                };
                let Some(org) = orgs.iter().find(|o| o.id == id) else {
                    continue;
                };
                if blocked(&org.tree_code, &disabled_codes) {
                    continue;
                }
                let parent_code = org.tree_code.clone();
                let sub_ids: HashSet<i64> = orgs
                    .iter()
                    .filter(|o| o.tree_code.starts_with(&parent_code))
                    .map(|o| o.id)
                    .collect();
                for m in members.iter().filter(|m| sub_ids.contains(&m.org_id)) {
                    if m.user_id < 0 {
                        continue;
                    }
                    add_hit(
                        &mut hits,
                        &mut hit_order,
                        m.user_id as u64,
                        &target.channels,
                        origin_of(&target.target_category.code, &target.target_id),
                    );
                }
            }
        }
    }

    // ── 4. 定位卡：卡号取自消息 labels，配置的 target_id 不参与 ──
    //
    //   历史上 target_id 用来表示「全部卡（空串）/ 固定卡」，广播禁用后这个
    //   配置已经没有意义：一条区域报警要通知哪些卡，只有发布方的 labels 知道。
    //   这里忽略配置值，按 labels 展开成逐卡目标；多条 locator 目标只合并渠道。
    let locator_targets: Vec<&TargetCache> = msg
        .targets
        .iter()
        .filter(|t| t.target_category.code == TC_LOCATOR)
        .collect();
    if let Some(first) = locator_targets.first() {
        // 渠道按 ID 去重合并：多条 locator 目标只是历史配置残留
        let mut seen_channels = HashSet::new();
        let mut channels = Vec::new();
        for target in &locator_targets {
            for channel in &target.channels {
                if !seen_channels.insert(channel.id) {
                    continue;
                }
                channels.push(ExpandedChannel {
                    channel: channel.clone(),
                    address: None,
                });
            }
        }

        if channels
            .iter()
            .any(|c| AddressKind::of_channel(&c.channel.channel_type).is_some())
        {
            // 定位卡不该配短信 / 邮件渠道：没有地址可取，落 Skipped 而不是静默跳过
            outcome.skipped.push(SkippedTarget {
                category_id: first.target_category.id,
                category_code: first.target_category.code.clone(),
                target_id: TARGET_ID_ALL.to_string(),
                origin: None,
                reason: format!(
                    "目标类别 {} 配了需要地址的渠道，但该类目标没有地址",
                    first.target_category.code
                ),
            });
        } else {
            let (cards, truncated) = locator_cards(labels);
            if truncated {
                warn!(
                    "定位卡卡号超过 {} 张，只投前 {} 张: event_code={}",
                    LOCATOR_MAX_CARDS, LOCATOR_MAX_CARDS, msg.event_code
                );
            }
            if cards.is_empty() {
                outcome.skipped.push(SkippedTarget {
                    category_id: first.target_category.id,
                    category_code: first.target_category.code.clone(),
                    target_id: TARGET_ID_ALL.to_string(),
                    origin: None,
                    reason: "定位卡目标按事件标签投递，但消息 labels 中没有 \
                             locatorCode / locatorCodes / cardCodes"
                        .to_string(),
                });
            } else {
                for card in cards {
                    outcome.targets.push(ExpandedTarget {
                        category_id: first.target_category.id,
                        category_code: first.target_category.code.clone(),
                        target_id: card,
                        origin: None,
                        channels: channels.clone(),
                    });
                }
            }
        }
    }

    // ── 4.1 屏幕等其它不展开的类别：原样作为落地目标 ──
    for target in msg.targets.iter().filter(|t| {
        !matches!(t.target_category.code.as_str(), TC_USER | TC_ROLE | TC_ORG)
            && t.target_category.code != TC_LOCATOR
    }) {
        let needs_address = target
            .channels
            .iter()
            .any(|c| AddressKind::of_channel(&c.channel_type).is_some());
        if needs_address {
            // 大屏 / 定位卡不该配短信 / 邮件渠道：没有地址可取，落 Skipped 而不是静默跳过
            outcome.skipped.push(SkippedTarget {
                category_id: target.target_category.id,
                category_code: target.target_category.code.clone(),
                target_id: target.target_id.clone(),
                origin: None,
                reason: format!(
                    "目标类别 {} 配了需要地址的渠道，但该类目标没有地址",
                    target.target_category.code
                ),
            });
            continue;
        }
        outcome.targets.push(ExpandedTarget {
            category_id: target.target_category.id,
            category_code: target.target_category.code.clone(),
            target_id: target.target_id.clone(),
            origin: None,
            channels: target
                .channels
                .iter()
                .map(|c| ExpandedChannel {
                    channel: c.clone(),
                    address: None,
                })
                .collect(),
        });
    }

    // ── 5. 解析用户地址，生成落地目标 ──
    if !hit_order.is_empty() {
        let user_ids: Vec<i64> = hit_order.iter().map(|id| *id as i64).collect();
        let users = UserEntity::find()
            .filter(UserCol::Id.is_in(user_ids))
            .all(db)
            .await
            .map_err(|e| format!("查询用户失败: {e}"))?;
        let user_by_id: HashMap<i64, &_> = users.iter().map(|u| (u.id, u)).collect();

        let person_ids: Vec<i64> = users.iter().filter_map(|u| u.person_id).collect();
        let persons = if person_ids.is_empty() {
            Vec::new()
        } else {
            PersonEntity::find()
                .filter(PersonCol::Id.is_in(person_ids))
                .all(db)
                .await
                .map_err(|e| format!("查询人员档案失败: {e}"))?
        };
        let person_by_id: HashMap<i64, &_> = persons.iter().map(|p| (p.id, p)).collect();

        // 地址去重：同一个人已定，不同的人共用同一个号才可能撞
        let mut taken: HashMap<(u64, String), u64> = HashMap::new();

        for user_id in hit_order {
            let hit = hits
                .remove(&user_id)
                .expect("hit_order 与 hits 同步维护，这里必存在");
            let origin = Some(truncate_origin(hit.origins.join(",")));

            let Some(user) = user_by_id.get(&(user_id as i64)) else {
                outcome.skipped.push(SkippedTarget {
                    category_id: user_cat_id,
                    category_code: TC_USER.to_string(),
                    target_id: user_id.to_string(),
                    origin,
                    reason: format!("用户 {user_id} 不在投影中（adm 未同步）"),
                });
                continue;
            };
            if !user.enabled {
                outcome.skipped.push(SkippedTarget {
                    category_id: user_cat_id,
                    category_code: TC_USER.to_string(),
                    target_id: user_id.to_string(),
                    origin,
                    reason: format!("用户 {user_id} 已停用"),
                });
                continue;
            }
            let person = user.person_id.and_then(|id| person_by_id.get(&id));

            let mut routes: Vec<ExpandedChannel> = Vec::new();
            for channel in hit.channels {
                let address = match AddressKind::of_channel(&channel.channel_type) {
                    None => None,
                    Some(kind) => {
                        let raw = match kind {
                            AddressKind::Mobile => user
                                .mobile
                                .as_deref()
                                .filter(|v| !v.trim().is_empty())
                                .or_else(|| {
                                    person.and_then(|p| {
                                        p.mobile.as_deref().filter(|v| !v.trim().is_empty())
                                    })
                                }),
                            AddressKind::Email => user
                                .email
                                .as_deref()
                                .filter(|v| !v.trim().is_empty())
                                .or_else(|| {
                                    person.and_then(|p| {
                                        p.email.as_deref().filter(|v| !v.trim().is_empty())
                                    })
                                }),
                        };
                        match raw {
                            Some(value) => Some(normalize_address(value)),
                            None => {
                                // 这个渠道取不到地址：跳过这一路，但这个人还有别的渠道可投
                                warn!(
                                    "用户 {user_id} 无 {:?} 地址，跳过渠道 {}（消息 {}）",
                                    kind, channel.channel_type, msg.event_code
                                );
                                continue;
                            }
                        }
                    }
                };

                if let Some(addr) = &address {
                    match taken.get(&(channel.id, addr.clone())) {
                        Some(owner) => {
                            // 不同人共用同一个号：先到先得，后者这一路不发
                            warn!(
                                "地址重复，抑制后到的渠道: channel={}, address={addr}, 已归用户 {owner}, 本次用户 {user_id}（消息 {}）",
                                channel.channel_type, msg.event_code
                            );
                            outcome.suppressed += 1;
                            continue;
                        }
                        None => {
                            taken.insert((channel.id, addr.clone()), user_id);
                        }
                    }
                }

                routes.push(ExpandedChannel {
                    channel: channel.clone(),
                    address,
                });
            }

            if routes.is_empty() {
                outcome.skipped.push(SkippedTarget {
                    category_id: user_cat_id,
                    category_code: TC_USER.to_string(),
                    target_id: user_id.to_string(),
                    origin,
                    reason: format!("用户 {user_id} 在本次目标配置的渠道上都没有可用地址"),
                });
                continue;
            }

            outcome.targets.push(ExpandedTarget {
                category_id: user_cat_id,
                category_code: TC_USER.to_string(),
                target_id: user_id.to_string(),
                origin,
                channels: routes,
            });
        }
    }

    Ok(outcome)
}

/// # 祖先链上（含自身）有没有停用的组织
///
/// 组织停用是连带的：上级关了，整棵子树都不投。只判断目标组织自己的 `enabled`
/// 会把「上级停用但下级还开着」的组织照发出去，所以沿树编码逐段回溯。
fn blocked(tree_code: &str, disabled_codes: &HashSet<&str>) -> bool {
    (3..=tree_code.len()).step_by(3).any(|len| {
        tree_code
            .get(..len)
            .map(|prefix| disabled_codes.contains(prefix))
            .unwrap_or(false)
    })
}

/// 记一次命中
/// 记一次命中：同一个人只保留第一条命中的渠道配置，来源全部记下来
fn add_hit<'a>(
    hits: &mut HashMap<u64, Hit<'a>>,
    hit_order: &mut Vec<u64>,
    user_id: u64,
    channels: &'a [ChannelCache],
    origin: String,
) {
    if let Some(hit) = hits.get_mut(&user_id) {
        hit.origins.push(origin);
        return;
    }
    hit_order.push(user_id);
    hits.insert(
        user_id,
        Hit {
            channels,
            origins: vec![origin],
        },
    );
}

/// `origin` 片段：`类别:原始目标标识`
fn origin_of(category_code: &str, target_id: &str) -> String {
    format!("{category_code}:{target_id}")
}

/// 地址规范化：去首尾空白、手机号去掉中间的空格与连字符
///
/// 只做「同号不同写法」的归一，不做号码合法性校验（那是 adm 侧的活）。
fn normalize_address(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, ' ' | '-' | '+'))
    {
        trimmed
            .chars()
            .filter(|c| !matches!(c, ' ' | '-'))
            .collect()
    } else {
        trimmed.to_lowercase()
    }
}

/// `origin` 按列宽截断（按字符，避免切出坏字符）
fn truncate_origin(mut origin: String) -> String {
    if origin.chars().count() <= ORIGIN_MAX_CHARS {
        return origin;
    }
    origin = origin.chars().take(ORIGIN_MAX_CHARS).collect();
    origin
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 手机号去空格 / 连字符；邮箱转小写；非号码不做字符清洗
    #[test]
    fn normalize_address_keeps_same_number_in_one_form() {
        assert_eq!(normalize_address(" 138 0013-8000 "), "13800138000");
        assert_eq!(normalize_address("+86 138-0013-8000"), "+8613800138000");
        assert_eq!(normalize_address(" Foo@Example.COM "), "foo@example.com");
    }

    /// origin 按字符截断，超长不留坏字符
    #[test]
    fn truncate_origin_counts_chars() {
        let short = "user:2,role:2".to_string();
        assert_eq!(truncate_origin(short.clone()), short);

        let long = "org:1001,".repeat(40);
        let cut = truncate_origin(long.clone());
        assert_eq!(cut.chars().count(), ORIGIN_MAX_CHARS);
        assert!(long.starts_with(&cut));
    }

    /// 组织停用要连带整棵子树：只判自己是漏的，最典型的是「上级停用、下级还开着」
    #[test]
    fn blocked_covers_ancestors() {
        let disabled: HashSet<&str> = HashSet::from(["001", "001002"]);
        assert!(blocked("001", &disabled), "自身停用");
        assert!(blocked("001002", &disabled), "自身停用");
        assert!(blocked("001002003", &disabled), "上级停用要连带");
        assert!(blocked("001002003004", &disabled), "隔代上级停用也要连带");
        assert!(!blocked("002", &disabled), "无关分支");
        assert!(!blocked("002001", &disabled), "无关分支的下级");
    }

    /// 渠道 → 地址种类：只有短信 / 邮件要地址，大屏与 NATS 不要
    #[test]
    fn only_personal_channels_need_address() {
        assert_eq!(AddressKind::of_channel("sms"), Some(AddressKind::Mobile));
        assert_eq!(AddressKind::of_channel("email"), Some(AddressKind::Email));
        assert_eq!(AddressKind::of_channel("websocket"), None);
        assert_eq!(AddressKind::of_channel("nats"), None);
        // 定位卡渠道的类型已从 locator 改成 nats；旧值不再认识
        assert_eq!(AddressKind::of_channel("locator"), None);
        // 任何未登记的类型都不解析地址
        assert_eq!(AddressKind::of_channel("unknown"), None);
    }

    fn labels(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// 卡号优先级：cardCodes 全量清单优先，其次 locatorCodes，最后单卡 locatorCode
    #[test]
    fn locator_cards_prefer_full_lists() {
        let all = labels(&[
            ("locatorCode", "C001"),
            ("locatorCodes", "C001,C002"),
            ("cardCodes", "C009,C010"),
        ]);
        let (cards, truncated) = locator_cards(&all);
        assert_eq!(cards, vec!["C009".to_string(), "C010".to_string()]);
        assert!(!truncated);

        let without_card_codes = labels(&[("locatorCode", "C001"), ("locatorCodes", "C001,C002")]);
        let (cards, _) = locator_cards(&without_card_codes);
        assert_eq!(cards, vec!["C001".to_string(), "C002".to_string()]);

        let single = labels(&[("locatorCode", "C001")]);
        let (cards, _) = locator_cards(&single);
        assert_eq!(cards, vec!["C001".to_string()]);
    }

    /// 逗号分隔、空白与重复剔除、保序；空 labels / 空串不产生卡号
    #[test]
    fn locator_cards_split_trim_and_dedupe() {
        let value = labels(&[("cardCodes", " C1 ,, C2,C1,  ,C3 ")]);
        let (cards, truncated) = locator_cards(&value);
        assert_eq!(
            cards,
            vec!["C1".to_string(), "C2".to_string(), "C3".to_string()]
        );
        assert!(!truncated);

        assert!(locator_cards(&labels(&[])).0.is_empty());
        assert!(locator_cards(&labels(&[("cardCodes", "   ")])).0.is_empty());
    }

    /// 超过 32 张截断，并标记 truncated
    #[test]
    fn locator_cards_cap_at_limit() {
        let raw = (0..40)
            .map(|i| format!("C{i:02}"))
            .collect::<Vec<_>>()
            .join(",");
        let (cards, truncated) = locator_cards(&labels(&[("cardCodes", raw.as_str())]));
        assert_eq!(cards.len(), LOCATOR_MAX_CARDS);
        assert!(truncated);
        assert_eq!(cards.first().map(String::as_str), Some("C00"));
        assert_eq!(cards.last().map(String::as_str), Some("C31"));

        // 刚好 32 张不算截断
        let raw = (0..32)
            .map(|i| format!("C{i:02}"))
            .collect::<Vec<_>>()
            .join(",");
        let (cards, truncated) = locator_cards(&labels(&[("cardCodes", raw.as_str())]));
        assert_eq!(cards.len(), LOCATOR_MAX_CARDS);
        assert!(!truncated);
    }
}
