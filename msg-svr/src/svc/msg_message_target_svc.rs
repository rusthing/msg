use crate::dao::{MsgMessageTargetChannelDao, MsgTargetCategoryChannelDao};
use crate::mo::msg_message_target_channel::{
    ActiveModel as TargetChannelActiveModel, Column as TargetChannelColumn,
};
use crate::mo::msg_target_category_channel::Column as CategoryChannelColumn;
use crate::vo::MsgTargetCategoryChannelVo;
use anyhow::anyhow;
use robotech::macros::svc;
use sea_orm::{ActiveValue::Set, ColumnTrait, Condition};
use std::collections::HashSet;

/// # 消息目标服务
///
/// 渠道绑定在写入这一行时一起落库：`add` / `modify` 的后置钩子与主体写在
/// 同一个事务里，先删掉该目标原有的绑定行，再按提交的 `channelIds` 重建，
/// 事务提交后由 `after_write` 统一刷新消息缓存。投递侧只认这些显式绑定，
/// 不再在运行时回退到目标类别的默认渠道。
#[svc(
    after_add = Self::sync_channels_on_add,
    after_modify = Self::sync_channels_on_modify,
    after_del_by_id = Self::cleanup_channels_on_delete,
    after_write = crate::cache::on_message_cache_refresh
)]
pub struct MsgMessageTargetSvc;

impl MsgMessageTargetSvc {
    /// # 解析目标最终要绑的渠道
    ///
    /// 显式传了 `channel_ids` 就去重后原样使用；没传（发布方自动注册）才取
    /// 目标类别的默认渠道：`is_default` 优先，一条都没标时取该类别全部绑定。
    /// 两端都没有时直接报错，避免建出一条永远投不出去的目标。
    async fn resolve_channel_ids<C>(
        channel_ids: Option<Vec<U64>>,
        category_id: u64,
        db: &C,
    ) -> Result<Vec<u64>, SvcError>
    where
        C: ConnectionTrait + Sync,
    {
        let explicit: Vec<u64> = channel_ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| id.0)
            .collect();
        if !explicit.is_empty() {
            let unique: Vec<u64> = explicit
                .into_iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            return Ok(unique);
        }

        let links = MsgTargetCategoryChannelDao::list_by_condition::<_, MsgTargetCategoryChannelVo>(
            Condition::all().add(CategoryChannelColumn::TargetCategoryId.eq(category_id as i64)),
            &None,
            db,
        )
        .await?;
        let defaults: Vec<u64> = links
            .iter()
            .filter(|link| link.is_default)
            .map(|link| link.channel_id.0)
            .collect();
        let chosen = if defaults.is_empty() {
            links
                .into_iter()
                .map(|link| link.channel_id.0)
                .collect::<Vec<_>>()
        } else {
            defaults
        };
        if chosen.is_empty() {
            return Err(SvcError::Runtime(anyhow!(
                "目标类别 {category_id} 没有配置渠道，无法登记消息目标"
            )));
        }
        Ok(chosen)
    }

    /// # 覆盖目标的渠道绑定
    ///
    /// 删除与插入共用调用方的事务：任何一条渠道无效都会让整次新增/修改回滚，
    /// 不会留下「目标建好了但渠道没绑全」的半状态。
    async fn replace_bindings<C>(
        target_id: u64,
        channel_ids: Vec<u64>,
        operator_id: u64,
        db: &C,
    ) -> Result<(), SvcError>
    where
        C: ConnectionTrait + Sync,
    {
        MsgMessageTargetChannelDao::delete_by_condition(
            Condition::all().add(TargetChannelColumn::MessageTargetId.eq(target_id as i64)),
            db,
        )
        .await?;
        for channel_id in channel_ids {
            MsgMessageTargetChannelDao::insert(
                TargetChannelActiveModel {
                    message_target_id: Set(target_id as i64),
                    channel_id: Set(channel_id as i64),
                    creater_id: Set(operator_id as i64),
                    ..Default::default()
                },
                db,
            )
            .await?;
        }
        Ok(())
    }

    /// # 新增后写入渠道绑定
    async fn sync_channels_on_add<C>(
        dto: MsgMessageTargetAddDto,
        one: MsgMessageTargetVo,
        db: &C,
    ) -> Result<MsgMessageTargetVo, SvcError>
    where
        C: ConnectionTrait + Sync,
    {
        let channels =
            Self::resolve_channel_ids(dto.channel_ids, one.target_category_id.0, db).await?;
        Self::replace_bindings(one.id.0, channels, dto._current_user_id.0, db).await?;
        Ok(one)
    }

    /// # 修改后重写渠道绑定
    ///
    /// 没带 `channelIds` 的局部修改（如只改开关）不碰绑定行。
    async fn sync_channels_on_modify<C>(
        dto: MsgMessageTargetModifyDto,
        one: MsgMessageTargetVo,
        db: &C,
    ) -> Result<MsgMessageTargetVo, SvcError>
    where
        C: ConnectionTrait + Sync,
    {
        if let Some(channel_ids) = dto.channel_ids {
            let channels =
                Self::resolve_channel_ids(Some(channel_ids), one.target_category_id.0, db).await?;
            Self::replace_bindings(one.id.0, channels, dto._current_user_id.0, db).await?;
        }
        Ok(one)
    }

    /// # 删除目标时清理绑定
    ///
    /// 绑定没有独立入口后，这条清理必须跟着目标走，避免留下永远读不到的孤儿行。
    async fn cleanup_channels_on_delete<C>(
        _id: U64,
        one: MsgMessageTargetVo,
        db: &C,
    ) -> Result<MsgMessageTargetVo, SvcError>
    where
        C: ConnectionTrait + Sync,
    {
        MsgMessageTargetChannelDao::delete_by_condition(
            Condition::all().add(TargetChannelColumn::MessageTargetId.eq(one.id.0 as i64)),
            db,
        )
        .await?;
        Ok(one)
    }
}
