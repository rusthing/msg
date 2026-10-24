//! # DTO 字段校验（`@len` / `@range` 注释规范）
//!
//! 大屏这类动态接入的目标只能在投递时按类别展开，发布方登记时用 `target_id`
//! 空串表示「该类别下的全部目标」（[`msg_api::cst::TARGET_ID_ALL`]）。
//!
//! `#[crud_dto]` 的字符串默认口径是「不许空串」：`required` + `length(min = 1)`，
//! 只有注释里写了 `@len 0..=N` 才放开最小长度。上限只认 `@len` 声明的值，
//! 不写就不限长。规范见 `robotech-macros/doc/dto-field-check.md`。
//!
//! 这几个用例把口径钉住：空串合法、边界值、超长被拒、未声明上限的不受限。

use msg_api::cst::TARGET_ID_ALL;
use msg_api::dto::{MsgMessageAddDto, MsgMessageTargetAddDto};
use robotech::api::U64;
use validator::Validate;

fn target_dto(target_id: String) -> MsgMessageTargetAddDto {
    MsgMessageTargetAddDto {
        message_id: Some(U64(1)),
        target_category_id: Some(U64(2)),
        target_id: Some(target_id),
        _current_user_id: U64(0),
        ..Default::default()
    }
}

fn message_dto(name: String, content_template: String) -> MsgMessageAddDto {
    MsgMessageAddDto {
        event_source_id: Some(U64(1)),
        category_id: Some(U64(2)),
        event_code: Some("dtw.position.updated".to_string()),
        name: Some(name),
        content_template: Some(Some(content_template)),
        _current_user_id: U64(0),
        ..Default::default()
    }
}

/// 广播登记：`target_id` 为空串必须合法（`@len 0..=100`）
#[test]
fn empty_target_id_is_valid() {
    target_dto(TARGET_ID_ALL.to_string())
        .validate()
        .expect("target_id 为空串（发给该类别下全部目标）应通过校验");
}

/// 注释里声明的是「最长 100 字符」，边界值照过
#[test]
fn target_id_at_declared_max_is_valid() {
    target_dto("9".repeat(100))
        .validate()
        .expect("正好 100 字符应通过校验");
}

/// 超过注释声明的 100 字符照样拒
#[test]
fn overly_long_target_id_is_rejected() {
    assert!(
        target_dto("9".repeat(101)).validate().is_err(),
        "超过 100 字符的 target_id 应被拒"
    );
}

/// 没写 `@len 0..` 的字符串字段：空串仍然不许
#[test]
fn empty_required_string_is_rejected() {
    let err = message_dto(String::new(), "{}".to_string())
        .validate()
        .expect_err("name 声明的是 1..=50，空串应被拒");
    assert!(format!("{err:?}").contains("name"), "{err:?}");
}

/// 声明了上限的字段：超一位就拒（`name` 是 `@len 1..=50`）
#[test]
fn declared_max_is_enforced() {
    assert!(
        message_dto("n".repeat(51), "{}".to_string())
            .validate()
            .is_err(),
        "超过 50 字符的 name 应被拒"
    );
}

/// 只声明了下界（`@len 0..`）的字段不限长，多长都能过
#[test]
fn unbounded_field_is_not_length_limited() {
    message_dto("名称".to_string(), "x".repeat(300))
        .validate()
        .expect("content_template 声明的是 0..，不该被限长");
}
