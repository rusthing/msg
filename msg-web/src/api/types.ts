/**
 * # 接口数据类型
 *
 * 与 `msg-api` 中的 DTO / VO 一一对应。字段命名遵循服务端 `#[vo]` / `#[crud_dto]`
 * 生成的 `#[serde(rename_all = "camelCase")]`，因此 Rust 的 `business_trigger_ms`
 * 在这里是 `businessTriggerMs`。
 *
 * ## 四个容易踩的点
 *
 * 1. `#[vo]` 生成的 VO 带 `#[skip_serializing_none]`，`Option` 字段为 `None` 时**整个字段不出现**，
 *    所以 `remark` / `ext` / `firstReceiveMs` 这类都必须按「可能不存在」处理。
 * 2. 字典枚举（`#[dic]`）序列化为**整数判别式**而不是字符串，取值见 `src/dic`。
 * 3. 所有 `u64` 字段（ID、审计 ID、毫秒时间戳）在 JSON 里是**字符串**，见 {@link Id}。
 * 4. `PageRx.total` 也是 `u64`，但它是数字形态的 `U64`（`"12"`），见 {@link PageRx}。
 */

/**
 * # 服务端 `u64` 字段的线上形态
 *
 * `#[vo]` / DTO 里的 `u64` 会被框架替换成 `robotech::api::U64`，序列化时输出字符串
 * （`"id": "1928374650123456789"`），为的是让 19 位雪花 ID 躲开 `JSON.parse` 的 `2^53` 精度上限。
 *
 * 由此带来两条硬约束：
 *
 * - **不要 `Number()`、不要算术**：`Number("1928374650123456789")` 直接变成 `...789000`，
 *   拿它去删记录会删不到任何东西；需要展示或传给 `dayjs` 时，毫秒时间戳（约 `1.7e12`）
 *   转数字是安全的，ID 不行。
 * - **反序列化只认字符串**：`U64` 的 visitor 只实现了 `visit_str`，把 ID 当 JSON 数字发回去
 *   会被后端判成参数错误，所以请求体里的 ID 也必须是 `string`。
 */
export type Id = string;

/** # 框架统一响应包装 */
export interface Ro<T> {
  /** 结果码：1 成功 / -1 参数错误 / -2 警告 / -3 失败 */
  result: number;
  /** 结果说明 */
  msg: string;
  /** 服务端生成响应时的毫秒时间戳（序列化为字符串） */
  timestamp: Id;
  /** 业务数据 */
  extra?: T;
  /** 详细错误信息 */
  detail?: string;
  /** 业务编码，如 `RTW00001` 表示唯一键重复 */
  code?: string;
}

/**
 * # 分页响应
 *
 * 注意 `pageNum` / `total` 是 `U64`，线上是**字符串**（`"1"`、`"37"`）。
 * 展示或参与计算前用 `Number()` 转一次——这两个值不会超过 `2^53`，转换是安全的，
 * 但直接把它们丢进算术或 `>` 比较会退化成字符串比较（`"9" > "10"`）。
 */
export interface PageRx<T> {
  /** 当前页码（从 1 开始） */
  pageNum: Id;
  /** 记录总数 */
  total: Id;
  /** 当前页记录 */
  list: T[];
}

/** # 审计字段（`u64`，线上是字符串） */
export interface AuditFields {
  createrId: Id;
  /** 毫秒时间戳字符串，展示前用 `formatMs` 转数字 */
  createMs: Id;
  updaterId: Id;
  updateMs: Id;
}

/** # 投递状态（`msg_delivery.deliver_status`） */
export type DeliverStatus = 0 | 1 | 2 | 3;

/** # 投递目标状态（`msg_delivery_target.deliver_target_status`） */
export type DeliverTargetStatus = 0 | 1 | 2 | 3;

/** # 渠道投递状态（`msg_delivery_channel.deliver_channel_status`） */
export type DeliverChannelStatus = 0 | 1 | 2;

/** # 一条记录的最小约束：CRUD 表格都按 id 操作 */
export type Row = { id: Id } & Record<string, unknown>;

/** # 事件来源 */
export interface MsgEventSourceVo extends AuditFields {
  id: Id;
  /** 编码 */
  code: string;
  /** 名称 */
  name: string;
  /** 备注 */
  remark?: string;
}

/** # 消息类别 */
export interface MsgMessageCategoryVo extends AuditFields {
  id: Id;
  code: string;
  name: string;
  remark?: string;
}

/** # 目标类别（用户 / 角色 / 分组 / 大屏） */
export interface MsgTargetCategoryVo extends AuditFields {
  id: Id;
  code: string;
  name: string;
  remark?: string;
}

/** # 渠道 */
export interface MsgChannelVo extends AuditFields {
  id: Id;
  /** 编码，同时是投递主题 `msg.delivery.<code>` 的末段 */
  code: string;
  name: string;
  /** 按渠道自定义的 JSON，如 websocket 的 `{"token":""}` */
  options?: string;
  /** 载荷模板，决定渠道报文正文长什么样 */
  payloadTemplate?: string;
  remark?: string;
  enabled: boolean;
}

/** # 消息定义 */
export interface MsgMessageVo extends AuditFields {
  id: Id;
  eventSourceId: Id;
  categoryId: Id;
  eventCode: string;
  name: string;
  persisted: boolean;
  titleTemplate?: string;
  contentTemplate?: string;
  extTemplate?: string;
  remark?: string;
  enabled: boolean;
}

/** # 消息目标：这条消息发给哪些目标类别 */
export interface MsgMessageTargetVo extends AuditFields {
  id: Id;
  messageId: Id;
  targetCategoryId: Id;
  /** 目标标识，空串表示该类别下的全部目标 */
  targetId: string;
}

/** # 目标类别与渠道的绑定 */
export interface MsgTargetCategoryChannelVo extends AuditFields {
  id: Id;
  targetCategoryId: Id;
  channelId: Id;
}

/** # 投递（一条消息的一次业务触发） */
export interface MsgDeliveryVo extends AuditFields {
  id: Id;
  eventCode: string;
  messageId: Id;
  eventSourceId: Id;
  messageCategoryId: Id;
  businessId: Id;
  businessTriggerMs: Id;
  receiveMs: Id;
  firstReceiveMs?: Id;
  firstTriggerMs?: Id;
  deliverStatus: DeliverStatus;
  labels?: string;
  annotations?: string;
  title: string;
  content: string;
  ext?: string;
  remark?: string;
  /** 是否模拟：发布方带 simulated=true 或 source=sim 标记时为是 */
  simulated: boolean;
}

/** # 投递目标 */
export interface MsgDeliveryTargetVo extends AuditFields {
  id: Id;
  deliveryId: Id;
  targetCategoryId: Id;
  /** 目标标识，空串表示该类别下的全部目标 */
  targetId: string;
  deliverTargetStatus: DeliverTargetStatus;
}

/** # 投递渠道（投递目标 × 渠道） */
export interface MsgDeliveryChannelVo extends AuditFields {
  id: Id;
  deliverTargetId: Id;
  channelId: Id;
  deliverChannelStatus: DeliverChannelStatus;
  address?: string;
}

/** # 渠道投递日志（每次发送尝试一条） */
export interface MsgDeliveryChannelLogVo extends AuditFields {
  id: Id;
  deliveryChannelId: Id;
  firstTriggerMs?: Id;
  firstReceiveMs?: Id;
  receiveMs: Id;
  finishMs?: Id;
  businessTriggerMs: Id;
  deliverChannelStatus: DeliverChannelStatus;
  address?: string;
  detail?: string;
}

/** # 消息队列（业务主题） */
export interface MsgMessageQueueVo extends AuditFields {
  id: Id;
  code: string;
  name: string;
  persisted: boolean;
  remark?: string;
}

/** # 投递队列（`msg.delivery.<渠道编码>`） */
export interface MsgDeliveryQueueVo extends AuditFields {
  id: Id;
  code: string;
  name: string;
  persisted: boolean;
  remark?: string;
}

/** # 死信 */
export interface MsgDeadLetterVo extends AuditFields {
  id: Id;
  /** 原消息主题 */
  subject: string;
  /** 转入死信的原因 */
  reason: string;
  /** 原始消息负载 */
  payload: string;
  /** 已经投递的次数 */
  delivered: Id;
  /** 是否已人工处理 */
  handled: boolean;
  remark?: string;
}
