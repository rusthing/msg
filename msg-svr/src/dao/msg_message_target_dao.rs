use robotech::macros::dao;

/// 消息目标
///
/// 两条唯一键：完整的一条是表上的唯一约束，另一条对应 `target_id` 为空时的
/// 部分唯一索引（广播行只能有一条）。重复键识别按约束名回查字段列表，
/// 所以部分索引的名字也必须按 `ak_{列名_and}_{表名}` 的规则登记在这里，
/// 否则插入重复的广播行会被当成运行时错误，接口直接 500。
#[dao(
    unique_keys: [
        ("message_id,target_category_id,target_id", "消息与目标类别与目标"),
        ("message_id,target_category_id", "消息与目标类别（广播）"),
    ],
    like_columns: [],
)]
pub struct MsgMessageTargetDao;