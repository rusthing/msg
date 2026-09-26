use robotech::macros::dao;

#[dao(
    like_columns: [
        Column::Code,
        Column::Name,
        Column::Remark,
    ],
)]
pub struct MsgEventSourceDao;