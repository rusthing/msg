use robotech::macros::router;

/// # 投递记录路由
///
/// `crud` 生成实体维护接口（`/msg/delivery`、`/msg/delivery/{id}` 等），
/// `routes` 追加投递记录自己的动作：
///
/// | 方法与路径 | 用途 |
/// |------------|------|
/// | `POST /msg/delivery/redeliver` | 再次投递（按原目标 × 渠道重发一遍） |
///
/// 静态路径排在 `{id}` 之前：`GET /msg/delivery/{id}` 是 CRUD 生成的详情接口，
/// 不能被动作名吃掉。
#[router(crud, routes [
    ("/msg/delivery/redeliver", post(redeliver)),
])]
struct MsgDeliveryRouter;
