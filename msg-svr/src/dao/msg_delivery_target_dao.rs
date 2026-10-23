use robotech::macros::dao;

/// 投递目标
///
/// 两条唯一键：完整的一条是表上的唯一约束，另一条对应 `target_id` 为空时的
/// 部分唯一索引（一次投递里同一类别只能有一条广播目标行）。名字必须按
/// `ak_{列名_and}_{表名}` 的规则登记，重复键识别才能对上（见
/// `unique_keys_utils.rs`）。
#[dao(
    unique_keys: [
        ("delivery_id,target_category_id,target_id", "投递与目标类别与目标"),
        ("delivery_id,target_category_id", "投递与目标类别（广播）"),
    ],
    like_columns: [],
)]
pub struct MsgDeliveryTargetDao;