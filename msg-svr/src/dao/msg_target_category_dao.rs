use robotech::macros::dao;

/// 目标类别
#[dao(
    like_columns: [
        Column::Code,
        Column::Name,
        Column::Remark,
    ],
)]
pub struct MsgTargetCategoryDao;