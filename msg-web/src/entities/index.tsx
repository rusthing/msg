/**
 * # 实体元数据
 *
 * `msg-svr` 的 14 张表都由 `#[router(crud)]` 生成**形状完全一致**的 CRUD 接口，唯一的差别
 * 是字段。因此前端不做 14 个几乎重复的页面，而是把「表 → 字段 → 控件」的对应关系集中
 * 写在这里，由 `EntityPage` 统一渲染列表、筛选、表单与详情。
 *
 * ## 加字段 / 改字段时改哪里
 *
 * 1. `msg-api` 的 DTO、VO、MO（后端三处）；
 * 2. 本文件对应实体的 `fields`（`key` 用 camelCase，与 VO 一致）；
 * 3. `src/api/types.ts` 里该表的接口类型（仅 Dashboard 这类强类型场景用得到）。
 *
 * ## 约定
 *
 * - `required` 对应 DTO 里**非 `Option` 且没有 `#[db_default]`** 的字段：新增时后端会校验，
 *   不传直接返回参数错误。
 * - `searchable` 的字段会出现在列表上方的筛选区，服务端是**等值匹配**；想模糊搜用「关键字」。
 * - `ref` 指向另一个实体的 `key`，表格与详情显示它的名称，表单渲染成下拉。
 */

import type { ReactNode } from 'react';
import type { ColumnsType } from 'antd/es/table';
import {
  AimOutlined,
  ApiOutlined,
  ApartmentOutlined,
  BarsOutlined,
  CloudUploadOutlined,
  FileTextOutlined,
  LinkOutlined,
  MessageOutlined,
  NotificationOutlined,
  ProfileOutlined,
  TagsOutlined,
  TeamOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import type { Row } from '../api/types';
import { deliverChannelStatusDic, deliverStatusDic, deliverTargetStatusDic, type DicItem } from '../dic';
import { durationMs } from '../utils/format';

/** # 字段控件类型 */
export type FieldKind =
  /** 单行文本 */
  | 'text'
  /** 多行文本 */
  | 'textarea'
  /** 纯文本模板：校验占位符、旧语法与过滤器链 */
  | 'template'
  /** JSON 模板：额外校验占位符代入后的整体 JSON */
  | 'jsonTemplate'
  /** JSON 文本（表单里带格式校验与一键格式化） */
  | 'json'
  /** `u64` 字符串：ID、次数这类不允许做数字运算的整数 */
  | 'u64'
  /** 毫秒时间戳（`u64` 字符串，展示成时间） */
  | 'ms'
  /** 布尔 */
  | 'bool'
  /** 字典枚举（`#[dic]`，线上是整数） */
  | 'dic'
  /** 外键，`ref` 指向另一个实体 */
  | 'ref';

/** # 一个字段的展示与编辑方式 */
export interface FieldConfig {
  /** VO 里的字段名（camelCase） */
  key: string;
  /** 中文标签 */
  label: string;
  kind: FieldKind;
  /** 列宽 */
  width?: number;
  /** 是否出现在列表（默认 `true`） */
  inTable?: boolean;
  /** 是否出现在新增/编辑表单（默认 `true`） */
  inForm?: boolean;
  /** 是否出现在详情抽屉（默认 `true`） */
  inDetail?: boolean;
  /** 新增时必填 */
  required?: boolean;
  /**
   * 空串有业务含义（如目标标识为空表示「该类别下全部目标」）。
   *
   * 默认提交时会跳过空串，交给服务端的 `Option` 语义；打开这个开关后空串会照常提交，
   * 否则「按类别广播」的目标根本存不进去。
   */
  allowEmptySubmit?: boolean;
  /** 空串的展示文案（如「全部目标」），不填则统一显示 `--` */
  emptyLabel?: string;
  /** 是否作为列表上方的等值筛选条件 */
  searchable?: boolean;
  /** 字典表，`kind = 'dic'` 时必填 */
  dic?: DicItem[];
  /** 被引用实体的 `key`，`kind = 'ref'` 时必填 */
  ref?: string;
  /** 表单最大长度（与 DTO 的 `@len` 对齐） */
  maxLength?: number;
  /** 多行文本的行数 */
  rows?: number;
  /** 表单里的说明文字 */
  tip?: string;
  /** 新增时的默认值 */
  defaultValue?: unknown;
  /**
   * 该列在数据库里有 `DEFAULT`（DTO 标了 `#[db_default]`）。
   *
   * 新增时若用户没动这个字段，提交体里就不带它，交给数据库默认值兜底；
   * 用户改过才提交，避免把界面上的初始值当成用户的选择。
   */
  dbDefault?: boolean;
  /** 表格里超长省略 */
  ellipsis?: boolean;
  /**
   * 表格里是否用开关直接切换（仅布尔字段有效）
   *
   * 状态类标记位只在列表操作，不再进编辑表单：开启、已处理、持久化。
   */
  switchable?: boolean;
}

/** # 侧边栏分组 */
export type EntityGroup = 'definition' | 'delivery' | 'queue';

/** # 一个实体的完整元数据 */
export interface EntityConfig {
  /** 路由段，同时是 `/msg/{key}` 的最后一段 */
  key: string;
  /** 资源根路径（含服务端路由前缀） */
  path: string;
  /** 单条记录的中文名，用于「新增消息」这类按钮与提示 */
  name: string;
  /** 页面标题 */
  title: string;
  /** 页面说明，放在标题下方 */
  desc: string;
  /** 侧边栏图标 */
  icon: ReactNode;
  /** 侧边栏分组 */
  group: EntityGroup;
  /** 业务字段（审计列由 EntityPage 统一追加，不写在这里） */
  fields: FieldConfig[];
  /** 表格里是否展示主键（雪花ID 很长，默认不展示） */
  showId?: boolean;
  /** 新增/编辑弹窗宽度 */
  modalWidth?: number;
  /** 额外列（放计算出来的耗时之类，不能直接对应某个字段） */
  extraColumns?: ColumnsType<Row>;
}

/** 编码 + 名称 + 备注：事件来源、消息类别、目标类别三张表结构一致 */
function codeNameRemarkFields(): FieldConfig[] {
  return [
    { key: 'code', label: '编码', kind: 'text', width: 160, required: true, searchable: true, maxLength: 50 },
    { key: 'name', label: '名称', kind: 'text', width: 180, required: true, searchable: true, maxLength: 50 },
    { key: 'remark', label: '备注', kind: 'text', width: 220, maxLength: 50, ellipsis: true },
  ];
}

/** # 事件来源 */
const eventSource: EntityConfig = {
  key: 'event-source',
  path: '/msg/event-source',
  name: '事件来源',
  title: '事件来源',
  desc: '谁在发消息：按应用登记来源，消息定义必须挂在某个来源下',
  icon: <ApiOutlined />,
  group: 'definition',
  fields: codeNameRemarkFields(),
};

/** # 消息类别 */
const messageCategory: EntityConfig = {
  key: 'message-category',
  path: '/msg/message-category',
  name: '消息类别',
  title: '消息类别',
  desc: '消息的业务分类，用于统计与按类别检索',
  icon: <TagsOutlined />,
  group: 'definition',
  fields: codeNameRemarkFields(),
};

/** # 目标类别 */
const targetCategory: EntityConfig = {
  key: 'target-category',
  path: '/msg/target-category',
  name: '目标类别',
  title: '目标类别',
  desc: '收消息的对象类型（用户 / 角色 / 分组 / 大屏），与渠道的绑定决定最终从哪发出去',
  icon: <TeamOutlined />,
  group: 'definition',
  fields: codeNameRemarkFields(),
};

/** # 渠道 */
const channel: EntityConfig = {
  key: 'channel',
  path: '/msg/channel',
  name: '渠道',
  title: '投递渠道',
  desc: '真正发消息的通道（站内 / 短信 / 邮件 / websocket 大屏…），编码同时是投递主题的末段',
  icon: <NotificationOutlined />,
  group: 'definition',
  modalWidth: 780,
  fields: [
    {
      key: 'code',
      label: '编码',
      kind: 'text',
      width: 140,
      required: true,
      searchable: true,
      maxLength: 50,
      tip: '同时是投递主题 `msg.delivery.<code>` 的末段，渠道写操作会触发投递队列缓存重建',
    },
    { key: 'name', label: '名称', kind: 'text', width: 160, required: true, searchable: true, maxLength: 50 },
    {
      key: 'options',
      label: '渠道配置',
      kind: 'json',
      rows: 6,
      tip: '按渠道自定义的 JSON，如 websocket 的 {"token":"..."}；握手时现查，改令牌不用重启',
    },
    {
      key: 'payloadTemplate',
      label: '载荷模板',
      kind: 'jsonTemplate',
      width: 260,
      rows: 8,
      ellipsis: true,
      tip: 'JSON 报文模板，保存前校验 JSON 与占位符。可用变量：{{eventCode}} {{title}} {{content}} {{ext}} {{firstReceiveMs}} {{businessTriggerMs}}；字符串变量写 {{var:json}}（自带引号），数字/时间直接写 {{var}}；过滤器链示例 {{title:trim|truncate(20)}}；${var} 等价',
    },
    { key: 'remark', label: '备注', kind: 'text', width: 200, maxLength: 50, ellipsis: true },
    { key: 'enabled', label: '启用', kind: 'bool', width: 90, defaultValue: true, switchable: true, inForm: false },
  ],
};

/** # 消息定义 */
const message: EntityConfig = {
  key: 'message',
  path: '/msg/message',
  name: '消息定义',
  title: '消息定义',
  desc: '一个事件编码对应一条消息定义：模板决定最终文案，目标决定发给谁',
  icon: <MessageOutlined />,
  group: 'definition',
  modalWidth: 820,
  fields: [
    {
      key: 'eventSourceId',
      label: '事件来源',
      kind: 'ref',
      ref: 'event-source',
      width: 150,
      required: true,
      searchable: true,
    },
    {
      key: 'categoryId',
      label: '消息类别',
      kind: 'ref',
      ref: 'message-category',
      width: 150,
      required: true,
      searchable: true,
    },
    { key: 'eventCode', label: '事件编码', kind: 'text', width: 180, required: true, searchable: true, maxLength: 50 },
    { key: 'name', label: '名称', kind: 'text', width: 180, required: true, searchable: true, maxLength: 50 },
    { key: 'persisted', label: '持久化', kind: 'bool', width: 100, defaultValue: true, dbDefault: true, switchable: true, inForm: false, tip: '不持久化时投递不落库，出问题只能看日志' },
    {
      key: 'titleTemplate',
      label: '标题模板',
      kind: 'template',
      rows: 3,
      maxLength: 150,
      inTable: false,
      tip: '纯文本模板，变量取 labels 与 annotations。直接写 {{var}}，过滤器链示例 {{var:trim|truncate(20)}}、{{var:default(未知)}}；${var} 等价；留空用发布方给的标题',
    },
    {
      key: 'contentTemplate',
      label: '内容模板',
      kind: 'template',
      rows: 6,
      inTable: false,
      tip: '纯文本模板。过滤器示例 {{amount:fixed(2)}}、{{phone:mask(3,4)}}、{{createAt:time}}；过滤器链示例 {{var:trim|upper}}；${var} 等价；留空用发布方给的内容',
    },
    {
      key: 'extTemplate',
      label: '附加内容模板',
      kind: 'jsonTemplate',
      rows: 4,
      inTable: false,
      tip: 'JSON 模板（渲染结果原样下发给消费方）。字符串变量写 {{var:json}}（自带引号），数字/时间直接写 {{var}}；如 {"name": {{userName:json}}, "at": {{createAt:time}}}；${var} 等价',
    },
    { key: 'remark', label: '备注', kind: 'text', inTable: false, maxLength: 50 },
    { key: 'enabled', label: '启用', kind: 'bool', width: 90, defaultValue: true, switchable: true, inForm: false },
  ],
};

/** # 消息目标 */
const messageTarget: EntityConfig = {
  key: 'message-target',
  path: '/msg/message-target',
  name: '消息目标',
  title: '消息目标',
  desc: '这条消息发给哪些目标类别；目标标识留空表示该类别下的全部目标',
  icon: <ApartmentOutlined />,
  group: 'definition',
  fields: [
    { key: 'messageId', label: '消息', kind: 'ref', ref: 'message', width: 200, required: true, searchable: true },
    {
      key: 'targetCategoryId',
      label: '目标类别',
      kind: 'ref',
      ref: 'target-category',
      width: 160,
      required: true,
      searchable: true,
    },
    {
      key: 'targetId',
      label: '目标标识',
      kind: 'text',
      width: 200,
      maxLength: 100,
      searchable: true,
      allowEmptySubmit: true,
      emptyLabel: '全部目标',
      tip: '用户 / 角色 / 分组 / 大屏的业务标识；空串表示该类别下的全部目标（大屏这类动态接入的目标只能这么声明）',
    },
  ],
};

/** # 目标类别与渠道的绑定 */
const targetCategoryChannel: EntityConfig = {
  key: 'target-category-channel',
  path: '/msg/target-category-channel',
  name: '类别渠道绑定',
  title: '类别渠道绑定',
  desc: '某类目标可以从哪些渠道收到消息：绑定几行，这个类别就走几个渠道',
  icon: <LinkOutlined />,
  group: 'definition',
  fields: [
    {
      key: 'targetCategoryId',
      label: '目标类别',
      kind: 'ref',
      ref: 'target-category',
      width: 200,
      required: true,
      searchable: true,
    },
    { key: 'channelId', label: '渠道', kind: 'ref', ref: 'channel', width: 200, required: true, searchable: true },
  ],
};

/** # 投递 */
const delivery: EntityConfig = {
  key: 'delivery',
  path: '/msg/delivery',
  name: '投递',
  title: '投递记录',
  desc: '一条消息的一次业务触发；`businessId + businessTriggerMs` 唯一，用于幂等去重',
  icon: <CloudUploadOutlined />,
  group: 'delivery',
  modalWidth: 860,
  fields: [
    { key: 'title', label: '标题', kind: 'text', width: 220, required: true, searchable: true, maxLength: 150, ellipsis: true },
    { key: 'eventCode', label: '事件编码', kind: 'text', width: 180, required: true, searchable: true, maxLength: 50 },
    {
      key: 'deliverStatus',
      label: '投递状态',
      kind: 'dic',
      dic: deliverStatusDic,
      width: 120,
      searchable: true,
      defaultValue: 0,
      inForm: false,
    },
    {
      key: 'simulated',
      label: '模拟',
      kind: 'bool',
      width: 90,
      searchable: true,
      inForm: false,
      tip: '发布方带 simulated=true 或 source=sim 标记时为是',
    },
    {
      key: 'businessTriggerMs',
      label: '业务触发时间',
      kind: 'ms',
      width: 180,
      required: true,
      tip: '与 businessId 组成幂等键，必须是业务时刻，不要用服务端当前时间',
    },
    { key: 'receiveMs', label: '接收时间', kind: 'ms', width: 180, required: true, inTable: false, tip: '消息中心收到并开始落库的时刻，服务端时钟' },
    { key: 'firstReceiveMs', label: '首收时间', kind: 'ms', width: 180, inTable: false, tip: '链路第一跳收到原始事件的时刻，由发布方透传' },
    { key: 'firstTriggerMs', label: '首次触发时间', kind: 'ms', width: 180, inTable: false },
    { key: 'messageId', label: '消息', kind: 'ref', ref: 'message', width: 180, required: true, searchable: true, inTable: false },
    {
      key: 'eventSourceId',
      label: '事件来源',
      kind: 'ref',
      ref: 'event-source',
      width: 150,
      required: true,
      searchable: true,
      inTable: false,
    },
    {
      key: 'messageCategoryId',
      label: '消息类别',
      kind: 'ref',
      ref: 'message-category',
      width: 150,
      required: true,
      inTable: false,
    },
    {
      key: 'businessId',
      label: '业务ID',
      kind: 'u64',
      width: 180,
      required: true,
      searchable: true,
      inTable: false,
      tip: '业务触发时生成，或用业务 ID 本身；与 businessTriggerMs 一起做幂等去重',
    },
    { key: 'labels', label: '路由标签', kind: 'json', rows: 5, inTable: false, tip: '结构化 key-value，所有规则匹配只看它，不看自由文本' },
    { key: 'annotations', label: '标注', kind: 'json', rows: 5, inTable: false, tip: '标题、详细描述、建议处理步骤这类可读文本，不参与匹配' },
    { key: 'content', label: '内容', kind: 'textarea', rows: 5, required: true, inTable: false },
    { key: 'ext', label: '附加内容', kind: 'textarea', rows: 4, inTable: false },
    { key: 'remark', label: '备注', kind: 'text', maxLength: 50, inTable: false },
  ],
  extraColumns: [
    {
      title: '接收延迟',
      key: 'receiveLatency',
      width: 110,
      align: 'right',
      render: (_, record) => durationMs(record.businessTriggerMs as string, record.receiveMs as string),
    },
  ],
};

/** # 投递目标 */
const deliveryTarget: EntityConfig = {
  key: 'delivery-target',
  path: '/msg/delivery-target',
  name: '投递目标',
  title: '投递目标',
  desc: '一次投递按目标类别展开出的具体目标；目标标识为空表示按类别广播',
  icon: <AimOutlined />,
  group: 'delivery',
  fields: [
    { key: 'deliveryId', label: '投递ID', kind: 'u64', width: 190, required: true, searchable: true },
    {
      key: 'targetCategoryId',
      label: '目标类别',
      kind: 'ref',
      ref: 'target-category',
      width: 160,
      required: true,
      searchable: true,
    },
    {
      key: 'targetId',
      label: '目标标识',
      kind: 'text',
      width: 200,
      maxLength: 100,
      searchable: true,
      allowEmptySubmit: true,
      emptyLabel: '全部目标',
      tip: '空串表示本次投递按目标类别广播，没有具体目标',
    },
    {
      key: 'deliverTargetStatus',
      label: '状态',
      kind: 'dic',
      dic: deliverTargetStatusDic,
      width: 120,
      searchable: true,
      defaultValue: 0,
      inForm: false,
    },
  ],
};

/** # 投递渠道 */
const deliveryChannel: EntityConfig = {
  key: 'delivery-channel',
  path: '/msg/delivery-channel',
  name: '投递渠道任务',
  title: '投递渠道任务',
  desc: '一个投递目标 × 一个渠道 = 一条发送任务；每次发送尝试在「渠道投递日志」里留一行',
  icon: <ApiOutlined />,
  group: 'delivery',
  fields: [
    { key: 'deliverTargetId', label: '投递目标ID', kind: 'u64', width: 190, required: true, searchable: true },
    { key: 'channelId', label: '渠道', kind: 'ref', ref: 'channel', width: 160, required: true, searchable: true },
    {
      key: 'deliverChannelStatus',
      label: '状态',
      kind: 'dic',
      dic: deliverChannelStatusDic,
      width: 120,
      searchable: true,
      defaultValue: 0,
      inForm: false,
    },
    { key: 'address', label: '投递地址', kind: 'text', width: 240, maxLength: 80, searchable: true, ellipsis: true },
  ],
};

/** # 渠道投递日志 */
const deliveryChannelLog: EntityConfig = {
  key: 'delivery-channel-log',
  path: '/msg/delivery-channel-log',
  name: '渠道投递日志',
  title: '渠道投递日志',
  desc: '每次发送尝试一条：接收 → 发送 → 完成，失败原因写在详情里',
  icon: <FileTextOutlined />,
  group: 'delivery',
  modalWidth: 820,
  fields: [
    { key: 'deliveryChannelId', label: '投递渠道ID', kind: 'u64', width: 190, required: true, searchable: true },
    {
      key: 'deliverChannelStatus',
      label: '状态',
      kind: 'dic',
      dic: deliverChannelStatusDic,
      width: 120,
      searchable: true,
      defaultValue: 0,
      inForm: false,
    },
    { key: 'receiveMs', label: '接收时间', kind: 'ms', width: 180, required: true, tip: '渠道消费到投递任务、开始发送的时刻，服务端时钟' },
    { key: 'finishMs', label: '完成时间', kind: 'ms', width: 180 },
    {
      key: 'businessTriggerMs',
      label: '业务触发时间',
      kind: 'ms',
      width: 180,
      required: true,
      inTable: false,
      tip: '与 msg_delivery.business_trigger_ms 同值，用它算端到端耗时',
    },
    { key: 'firstReceiveMs', label: '首收时间', kind: 'ms', width: 180, inTable: false },
    { key: 'firstTriggerMs', label: '首次触发时间', kind: 'ms', width: 180, inTable: false },
    { key: 'address', label: '投递地址', kind: 'text', width: 220, maxLength: 80, searchable: true, ellipsis: true },
    { key: 'detail', label: '投递详情', kind: 'json', rows: 6, inTable: false, tip: '失败原因或渠道返回的原始响应' },
  ],
  extraColumns: [
    {
      title: '发送耗时',
      key: 'sendLatency',
      width: 110,
      align: 'right',
      render: (_, record) => durationMs(record.receiveMs as string, record.finishMs as string),
    },
    {
      title: '端到端耗时',
      key: 'e2eLatency',
      width: 120,
      align: 'right',
      render: (_, record) => durationMs(record.businessTriggerMs as string, record.finishMs as string),
    },
  ],
};

/** # 死信 */
const deadLetter: EntityConfig = {
  key: 'dead-letter',
  path: '/msg/dead-letter',
  name: '死信',
  title: '死信',
  desc: '重投达到上限后转入 dlq.<原主题> 再落库的负载；排查完把「已处理」勾上',
  icon: <WarningOutlined />,
  group: 'delivery',
  modalWidth: 820,
  fields: [
    { key: 'subject', label: '原消息主题', kind: 'text', width: 240, required: true, searchable: true, maxLength: 150 },
    { key: 'reason', label: '转入原因', kind: 'textarea', rows: 3, required: true, width: 260, ellipsis: true },
    { key: 'delivered', label: '投递次数', kind: 'u64', width: 110, defaultValue: '0' },
    { key: 'handled', label: '已处理', kind: 'bool', width: 90, searchable: true, defaultValue: false, switchable: true, inForm: false },
    { key: 'payload', label: '原始负载', kind: 'textarea', rows: 8, required: true, inTable: false },
    { key: 'remark', label: '备注', kind: 'text', maxLength: 50, inTable: false },
  ],
};

/** # 消息队列 */
const messageQueue: EntityConfig = {
  key: 'message-queue',
  path: '/msg/message-queue',
  name: '消息队列',
  title: '消息队列',
  desc: '消息中心订阅的业务主题：发布方往这里发事件，服务端按订阅消费',
  icon: <ProfileOutlined />,
  group: 'queue',
  fields: [
    { key: 'code', label: '编码（主题）', kind: 'text', width: 240, required: true, searchable: true, maxLength: 50 },
    { key: 'name', label: '名称', kind: 'text', width: 200, required: true, searchable: true, maxLength: 50 },
    { key: 'persisted', label: '持久化', kind: 'bool', width: 100, defaultValue: true, dbDefault: true, switchable: true, inForm: false, tip: '持久化走 JetStream（等 ACK、重启补消费）；不持久化走 Core NATS，发出即不管送达' },
    { key: 'remark', label: '备注', kind: 'text', maxLength: 50 },
  ],
};

/** # 投递队列 */
const deliveryQueue: EntityConfig = {
  key: 'delivery-queue',
  path: '/msg/delivery-queue',
  name: '投递队列',
  title: '投递队列',
  desc: '每个渠道一条投递主题，编码固定为 `msg.delivery.<渠道编码>`，服务端会校验前缀与渠道是否存在',
  icon: <BarsOutlined />,
  group: 'queue',
  fields: [
    {
      key: 'code',
      label: '编码（主题）',
      kind: 'text',
      width: 280,
      required: true,
      searchable: true,
      maxLength: 50,
      tip: '末段必须等于某个渠道编码，渠道编码从主题反解，本表不存渠道 ID',
    },
    { key: 'name', label: '名称', kind: 'text', width: 200, required: true, searchable: true, maxLength: 50 },
    { key: 'persisted', label: '持久化', kind: 'bool', width: 100, defaultValue: false, dbDefault: true, switchable: true, inForm: false },
    { key: 'remark', label: '备注', kind: 'text', maxLength: 50 },
  ],
};

/** # 全部实体，顺序即侧边栏顺序 */
export const ENTITIES: EntityConfig[] = [
  eventSource,
  messageCategory,
  targetCategory,
  channel,
  message,
  messageTarget,
  targetCategoryChannel,
  delivery,
  deliveryTarget,
  deliveryChannel,
  deliveryChannelLog,
  deadLetter,
  messageQueue,
  deliveryQueue,
];

/** # 按 key 取实体 */
export function entityOf(key: string): EntityConfig | undefined {
  return ENTITIES.find((item) => item.key === key);
}

/** # 分组标题 */
export const GROUP_LABELS: Record<EntityGroup, string> = {
  definition: '消息定义',
  delivery: '投递链路',
  queue: '队列',
};
