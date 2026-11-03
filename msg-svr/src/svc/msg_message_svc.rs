use robotech::macros::svc;

/// 消息模板
///
/// 三份模板都在保存前做语法校验（`utils::validate_template_field`）：旧语法
/// `{key}`、未知过滤器、未闭合的 `{{` / `${` 都会在写库前被拒掉，避免上线后
/// 才在渲染日志里发现。`${key}` 与 `{{key}}` 是等价写法，不会被误判。
#[svc(
    before_add = Self::before_add,
    before_modify = Self::before_modify,
    after_write = crate::cache::on_message_cache_refresh
)]
pub struct MsgMessageSvc;

impl MsgMessageSvc {
    async fn before_add<C: ConnectionTrait>(
        add_dto: MsgMessageAddDto,
        _db: &C,
    ) -> Result<MsgMessageAddDto, SvcError> {
        crate::utils::validate_template_field(
            "title_template",
            add_dto.title_template.as_ref().and_then(Option::as_deref),
        )?;
        crate::utils::validate_template_field(
            "content_template",
            add_dto.content_template.as_ref().and_then(Option::as_deref),
        )?;
        crate::utils::validate_template_field(
            "ext_template",
            add_dto.ext_template.as_ref().and_then(Option::as_deref),
        )?;
        Ok(add_dto)
    }

    async fn before_modify<C: ConnectionTrait>(
        modify_dto: MsgMessageModifyDto,
        _db: &C,
    ) -> Result<MsgMessageModifyDto, SvcError> {
        crate::utils::validate_template_field(
            "title_template",
            modify_dto.title_template.as_ref().and_then(Option::as_deref),
        )?;
        crate::utils::validate_template_field(
            "content_template",
            modify_dto.content_template.as_ref().and_then(Option::as_deref),
        )?;
        crate::utils::validate_template_field(
            "ext_template",
            modify_dto.ext_template.as_ref().and_then(Option::as_deref),
        )?;
        Ok(modify_dto)
    }
}
