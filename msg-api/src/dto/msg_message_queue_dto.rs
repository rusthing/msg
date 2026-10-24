use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageQueueDto {
    /// 编码，用于订阅消息中间件队列的名称
    /// @len 1..=50
    pub code: String,
    /// 名称
    /// @len 1..=50
    pub name: String,
    /// 是否持久化
    #[db_default]
    pub persisted: bool,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
}