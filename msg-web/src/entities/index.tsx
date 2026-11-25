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
import { Button } from 'antd';
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
import { deliveryApi } from '../api/delivery';
import { CHANNEL_TYPES } from '../lib/channel-options';
import { deadLetterApi } from '../api/deadLetter';
import type { Row } from '../api/types';
import { deliverChannelStatusDic, deliverStatusDic, deliverTargetStatusDic, type DicItem } from '../dic';
import { durationMs, errorMessage } from '../utils/format';

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
  /** 渠道配置：按渠道类型渲染图形化表单，类型未登记时回退原始 JSON */
  | 'channelOptions'
  /** 静态选项下拉（选项是字符串，不是字典枚举） */
  | 'select'
  /** `u64` 字符串：ID、次数这类不允许做数字运算的整数 */
  | 'u64'
  /** 毫秒时间戳（`u64` 字符串，展示成时间） */
  | 'ms'
  /** 布尔 */
  | 'bool'
  /** 字典枚举（`#[dic]`，线上是整数） */
  | 'dic'
  /** 外键，`ref` 指向另一个实体 */
  | 'ref'
  /** 目标主体：按同表单另一字段的类别编码，从用户/角色/组织目录里选择 */
  | 'subject';

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
  /**
   * 字典字段的筛选控件固定用下拉，不走「少量选项渲染成按钮组」的默认规则。
   *
   * 默认规则看选项个数（0–1 个静态文字、2–5 个按钮组、≥6 个下拉），适合「是/否」
   * 这种二选一；投递状态这类四项枚举在筛选区用按钮组会占掉一整行，统一改下拉。
   */
  filterAsSelect?: boolean;
  /** 字典表，`kind = 'dic'` 时必填 */
  dic?: DicItem[];
  /** 被引用实体的 `key`，`kind = 'ref'` 时必填 */
  ref?: string;
  /** 下拉选项，`kind = 'select'` 时必填 */
  options?: { label: string; value: string }[];
  /** 类别字段名，`kind = 'subject'` 时必填；该字段指向的实体编码决定目录来源 */
  subjectBy?: string;
  /** 类别编码 → 目录来源（用户/角色/组织）；未命中的类别仍按文本输入 */
  subjectSources?: Record<string, 'user' | 'role' | 'org'>;
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
   * 只给「改了就一件事」的标记位开：启用、允许模拟。像 `persisted` 这种会改变
   * 消息链路语义的开关不给，必须进编辑弹窗看一眼上下文；死信的「已处理」走
   * 专用动作按钮，不在列表上直接切换。
   */
  switchable?: boolean;
  /**
   * 归入编辑表单的「模板」区域，与同组字段一起用 Tabs 分页展示。
   *
   * 只给「标题 / 内容 / 附加」这类成组的模板字段开；单个模板（如渠道载荷模板）
   * 仍按普通字段平铺，没必要为了一个 tab 套一层区域。
   */
  templateGroup?: boolean;
  /**
   * 编辑时把「用户清空」提交成 JSON `null`，真正把数据库列置为 NULL。
   *
   * 后端 `#[crud_dto]` 给 `Option<String>` 挂了三态 serde：字段不出现 = 不修改、
   * 显式 `null` = 置空。默认提交逻辑会跳过空串，表现为「留空不改」，已保存的模板
   * 因此删不掉；打开这个开关后，被用户动过且清空的字段会提交 `null`。
   */
  nullable?: boolean;
}

/** # 侧边栏分组 */
export type EntityGroup = 'definition' | 'delivery' | 'queue';

/**
 * # 行内附加操作的上下文
 *
 * 附加操作写在实体元数据里（「这张表上还能做什么」属于表的知识），但它要刷新
 * 列表、弹确认框、出提示——这些都在页面手里。所以页面把这三件事以函数形式递进
 * 来，元数据只负责描述动作本身，不碰 React 状态。
 */
export interface RowActionContext {
  /** 重新拉当前页（保留筛选与页码） */
  reload: () => void;
  /** 轻提示 */
  notify: (type: 'success' | 'error', text: string) => void;
  /** 二次确认；`onOk` 抛错时弹窗保持打开，调用方可以重试 */
  confirm: (options: {
    title: string;
    content: string;
    okText?: string;
    onOk: () => Promise<void>;
  }) => void;
}

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
  /**
   * 列表的默认排序，形如 `update_ms:desc`（服务端 `_orderBy`，列名用数据库列名）。
   *
   * 只写在这张表上：查询、重置、刷新都会带上它；不填则由数据库自然顺序返回。
   * 投递记录这类「一直在追加」的表必须倒序，否则默认看到的是最旧的一批。
   */
  defaultOrderBy?: string;
  /** 表格里是否展示主键（雪花ID 很长，默认不展示） */
  showId?: boolean;
  /**
   * 是否在列表里隐藏「更新时间」列（默认展示）。
   *
   * 字典 / 配置类表（事件来源、各类别、渠道、队列等）改完即止，更新时间对排查
   * 没有帮助；投递链路这类持续追加的记录表保留，方便看出哪条刚动过。详情抽屉
   * 里的「更新时间」不受影响，任何表都能看到。
   */
  hideUpdateTime?: boolean;
  /**
   * 只读实体：隐藏「新增 / 编辑 / 删除」，只保留「详情」与 `extraActions`。
   *
   * 只给由链路自动维护、手工增删改会破坏状态一致性的表开（投递、投递目标、
   * 渠道任务、渠道日志、死信）。服务端这些表也只暴露查询接口，前端隐藏按钮
   * 只是不让用户点到必然失败的入口。
   */
  readOnly?: boolean;
  /** 新增/编辑弹窗宽度 */
  modalWidth?: number;
  /**
   * 不在侧边栏显示（路由仍保留）。
   *
   * 用于「维护入口已经合并进别的页面、但接口和深链还要留着」的表。
   */
  hideInMenu?: boolean;
  /**
   * 下拉/表格里的可读名称。
   *
   * 默认取 `name` / `code` / 主键；投递链路这类没有 name 的表需要自己拼一行
   * （如「标题 · 事件编码」「目标 · 类别」），否则下拉里只剩一串雪花 ID。
   */
  displayLabel?: (row: Row) => string;
  /** 额外列（放计算出来的耗时之类，不能直接对应某个字段） */
  extraColumns?: ColumnsType<Row>;
  /**
   * 行内附加操作，渲染在「操作」列最前面
   *
   * 只给「CRUD 之外的业务动作」用（如投递记录的「再次投递」）。不放这里的动作
   * 一律走通用的详情 / 编辑 / 删除，别再往 EntityPage 里加特例。
   */
  extraActions?: (record: Row, ctx: RowActionContext) => ReactNode;
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
  hideUpdateTime: true,
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
  hideUpdateTime: true,
  fields: codeNameRemarkFields(),
};

/** # 目标类别 */
const targetCategory: EntityConfig = {
  key: 'target-category',
  path: '/msg/target-category',
  name: '目标类别',
  title: '目标类别',
  desc: '收消息的对象类型（用户 / 角色 / 分组 / 大屏 / 定位卡）；是否允许广播、目标标识从哪取由类别声明，渠道绑定决定最终从哪发出去',
  icon: <TeamOutlined />,
  group: 'definition',
  hideUpdateTime: true,
  fields: [
    ...codeNameRemarkFields(),
    {
      key: 'allowBroadcast',
      label: '允许广播',
      kind: 'bool',
      width: 96,
      defaultValue: true,
      tip: 'false 表示该类别必须给具体目标标识，空目标在投递时直接跳过并记原因；由类别 owner（如 dtw）注册时声明，消息中心不做特例代码',
    },
    {
      key: 'targetKeys',
      label: '目标标识键',
      kind: 'text',
      width: 220,
      maxLength: 100,
      tip: '从发布方事件标签取目标标识的键，逗号分隔按序取第一个有值者，值内逗号分隔多个目标；空表示只用配置的目标标识',
    },
  ],
};

/** # 渠道 */
const channel: EntityConfig = {
  key: 'channel',
  path: '/msg/channel',
  name: '渠道',
  title: '投递渠道',
  desc: '真正发消息的通道（大屏 / 定位卡 / 短信 / 邮件）；渠道类型就是渠道的身份，决定发送器与配置字段',
  icon: <NotificationOutlined />,
  group: 'definition',
  hideUpdateTime: true,
  modalWidth: 780,
  fields: [
    {
      key: 'channelType',
      label: '渠道类型',
      kind: 'select',
      width: 120,
      required: true,
      searchable: true,
      options: CHANNEL_TYPES,
      tip: '渠道的唯一标识：决定用哪个发送器、「渠道配置」显示哪些字段，也是投递主题 `msg.delivery.<类型>` 的末段。一个类型一条渠道，建好后一般不改；业务叫法写在名称里',
    },
    { key: 'name', label: '名称', kind: 'text', width: 160, required: true, searchable: true, maxLength: 50 },
    {
      key: 'options',
      label: '渠道配置',
      kind: 'channelOptions',
      rows: 6,
      tip: '按渠道类型渲染字段；留空的项用后端默认值。改配置不用重启，渠道写操作会重建消息缓存',
    },
    {
      key: 'payloadTemplate',
      label: '载荷模板',
      kind: 'jsonTemplate',
      width: 260,
      rows: 8,
      ellipsis: true,
      tip: '报文模板，保存前校验 JSON 与占位符。通用变量：{{targetId}} {{eventCode}} {{title}} {{content}} {{ext}} {{firstReceiveMs}} {{businessTriggerMs}}；业务协议（如定位卡的 deviceId/code/message）由注册渠道的一方在模板里写死。字符串变量写 {{var:json}}（自带引号）；过滤器链示例 {{title:trim|truncate(20)}}；${var} 等价',
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
  hideUpdateTime: true,
  modalWidth: 820,
  fields: [
    // 列表列顺序与 fields 一致：事件编码、名称是最先看的身份列，放最前；
    // 两个外键来源退到它们后面（筛选区与详情抽屉也跟着这个顺序）
    // 事件编码与名称都已在服务端的 `_keyword` 模糊列里（含标题/内容模板、备注），
    // 再加两个等值筛选只会把筛选区拉长，按关键字搜一次就够
    { key: 'eventCode', label: '事件编码', kind: 'text', width: 180, required: true, maxLength: 50 },
    { key: 'name', label: '名称', kind: 'text', width: 180, required: true, maxLength: 50 },
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
    { key: 'persisted', label: '持久化', kind: 'bool', defaultValue: true, dbDefault: true, inTable: false, tip: '不持久化时投递不落库，出问题只能看日志' },
    {
      key: 'titleTemplate',
      label: '标题模板',
      kind: 'template',
      rows: 3,
      maxLength: 150,
      inTable: false,
      templateGroup: true,
      nullable: true,
      tip: '纯文本模板，变量取 labels 与 annotations。直接写 {{var}}，过滤器链示例 {{var:trim|truncate(20)}}、{{var:default(未知)}}；${var} 等价；留空用发布方给的标题',
    },
    {
      key: 'contentTemplate',
      label: '内容模板',
      kind: 'template',
      rows: 6,
      inTable: false,
      templateGroup: true,
      nullable: true,
      tip: '纯文本模板。过滤器示例 {{amount:fixed(2)}}、{{phone:mask(3,4)}}、{{createAt:time}}；过滤器链示例 {{var:trim|upper}}；${var} 等价；留空用发布方给的内容',
    },
    {
      key: 'extTemplate',
      label: '附加内容模板',
      kind: 'jsonTemplate',
      rows: 4,
      inTable: false,
      templateGroup: true,
      nullable: true,
      tip: 'JSON 模板（渲染结果原样下发给消费方）。字符串变量写 {{var:json}}（自带引号），数字/时间直接写 {{var}}；如 {"name": {{userName:json}}, "at": {{createAt:time}}}；${var} 等价',
    },
    { key: 'remark', label: '备注', kind: 'text', inTable: false, maxLength: 50 },
    {
      key: 'simEnabled',
      label: '允许模拟',
      kind: 'bool',
      width: 110,
      defaultValue: true,
      dbDefault: true,
      switchable: true,
      inForm: false,
      tip: '默认开启；打开时带 simulated=true 标记的通知才允许投递，关闭后模拟通知在投递前被丢弃，不会上大屏',
    },
    { key: 'enabled', label: '启用', kind: 'bool', width: 90, defaultValue: true, switchable: true, inForm: false },
  ],
};

/** # 消息目标 */
const messageTarget: EntityConfig = {
  key: 'message-target',
  path: '/msg/message-target',
  name: '消息目标',
  title: '消息目标',
  desc: '这条消息发给哪些目标类别；目标标识留空表示该类别下的全部目标，定位卡的卡号由事件标签决定',
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
      kind: 'subject',
      // 目标类别编码 → 目录来源；大屏 / 定位卡这类动态目标仍按文本输入或由标签决定
      subjectBy: 'targetCategoryId',
      subjectSources: { user: 'user', role: 'role', org: 'org' },
      width: 200,
      maxLength: 100,
      searchable: true,
      allowEmptySubmit: true,
      emptyLabel: '全部目标',
      tip: '用户 / 角色 / 组织从平台目录里选择；大屏等动态目标按业务标识填写；类别配了「目标标识键」时标识由事件标签决定、此字段不生效；留空表示该类别下的全部目标（类别禁用广播时会被跳过）',
    },
  ],
};

/** # 消息目标与渠道的绑定（已并入消息目标表单，仅保留路由供接口/深链使用） */
const messageTargetChannel: EntityConfig = {
  key: 'message-target-channel',
  path: '/msg/message-target-channel',
  name: '目标渠道绑定',
  title: '目标渠道绑定',
  desc: '投递只认这里的显式绑定；维护入口已合并进「消息目标」的新增/编辑弹窗，本页不再出现在侧边栏',
  icon: <LinkOutlined />,
  group: 'definition',
  hideInMenu: true,
  readOnly: true,
  fields: [
    {
      key: 'messageTargetId',
      label: '消息目标',
      kind: 'ref',
      ref: 'message-target',
      width: 220,
      required: true,
      searchable: true,
    },
    { key: 'channelId', label: '渠道', kind: 'ref', ref: 'channel', width: 200, required: true, searchable: true },
  ],
};

/** # 目标类别与渠道的绑定 */
const targetCategoryChannel: EntityConfig = {
  key: 'target-category-channel',
  path: '/msg/target-category-channel',
  name: '默认渠道集',
  title: '默认渠道集',
  desc: '新建消息目标时的渠道预填模板；投递只认目标上显式绑定的渠道，这里不再做运行时兜底',
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
    {
      key: 'isDefault',
      label: '默认渠道',
      kind: 'bool',
      width: 96,
      defaultValue: false,
      tip: '同一目标类别下最多一条：新建消息目标时界面按它预填；保存后渠道写进目标自己的绑定，投递不再回读这里',
    },
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
  readOnly: true,
  modalWidth: 860,
  displayLabel: (row) =>
    [row.title, row.eventCode].filter((part) => typeof part === 'string' && part).join(' · ') || String(row.id),
  // 投递记录只增不减，默认按更新时间倒序：刷新后最上面一条就是刚动过的那条
  defaultOrderBy: 'update_ms:desc',
  fields: [
    // 标题 / 事件编码 / 业务ID 只展示不参与筛选：它们要么是自由文本，要么是长数字，
    // 精确等值匹配在投递记录上没什么用（模糊搜索走「关键字」，模糊不到就去投递目标里下钻）
    { key: 'title', label: '标题', kind: 'text', width: 220, required: true, maxLength: 150, ellipsis: true },
    { key: 'eventCode', label: '事件编码', kind: 'text', width: 180, required: true, maxLength: 50 },
    {
      key: 'deliverStatus',
      label: '投递状态',
      kind: 'dic',
      dic: deliverStatusDic,
      width: 120,
      searchable: true,
      filterAsSelect: true,
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
      tip: '发布方带 simulated=true 标记时为是',
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
  /**
   * 再次投递
   *
   * 渠道挂过、大屏重启过、或者当时那一路失败了：运营不必重新触发一次业务事件
   * （那会造出一条新的投递记录、业务时刻也变了），直接按原目标与渠道补发一遍。
   *
   * 不加「投递中不可点」的限制：广播目标在所有实例都没连接时状态会停在「投递中」，
   * 那恰恰是最需要补发的场景。
   */
  extraActions: (record, ctx) => (
    <Button
      type="link"
      size="small"
      onClick={() =>
        ctx.confirm({
          title: '再次投递这条消息？',
          content:
            '按这条投递原来的目标与渠道重新发一遍：状态会回到「投递中」，已经成功的渠道也会再收一次；渠道投递日志里会多一行「手工重投」。',
          okText: '再次投递',
          onOk: async () => {
            try {
              const result = await deliveryApi.redeliver(record.id);
              ctx.notify(
                'success',
                `已提交 ${result.publishedCount}/${result.taskCount} 路投递任务` +
                  (result.failedCount > 0 ? `，其中 ${result.failedCount} 路发布失败` : ''),
              );
              ctx.reload();
            } catch (reason) {
              ctx.notify('error', errorMessage(reason));
              // 抛回给 Modal：失败时保持弹窗，用户可以重试
              throw reason;
            }
          },
        })
      }
    >
      再次投递
    </Button>
  ),
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
  readOnly: true,
  displayLabel: (row) => `${row.targetId ? String(row.targetId) : '全部目标'} · #${row.id}`,
  fields: [
    { key: 'deliveryId', label: '投递ID', kind: 'ref', ref: 'delivery', width: 190, required: true, searchable: true },
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
      kind: 'subject',
      subjectBy: 'targetCategoryId',
      subjectSources: { user: 'user', role: 'role', org: 'org' },
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
  readOnly: true,
  displayLabel: (row) => `渠道 #${String(row.channelId)} · ${String(row.address ?? '')}`.trim(),
  fields: [
    { key: 'deliverTargetId', label: '投递目标ID', kind: 'ref', ref: 'delivery-target', width: 190, required: true, searchable: true },
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
  readOnly: true,
  modalWidth: 820,
  fields: [
    { key: 'deliveryChannelId', label: '投递渠道ID', kind: 'ref', ref: 'delivery-channel', width: 190, required: true, searchable: true },
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
    // 详情是自由文本（「发送成功」/异常原因/渠道原始响应），不是 JSON：
    // 用 json 控件会让编辑这条日志时因存量纯文本过不了 JSON 校验而无法保存
    { key: 'detail', label: '投递详情', kind: 'textarea', rows: 6, inTable: false, tip: '失败原因或渠道返回的原始响应' },
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
  desc: '重投达到上限后转入 dlq.<原主题> 再落库的负载；排查完用行内「标记已处理」收口',
  icon: <WarningOutlined />,
  group: 'delivery',
  readOnly: true,
  modalWidth: 820,
  fields: [
    { key: 'subject', label: '原消息主题', kind: 'text', width: 240, required: true, searchable: true, maxLength: 150 },
    { key: 'reason', label: '转入原因', kind: 'textarea', rows: 3, required: true, width: 260, ellipsis: true },
    { key: 'delivered', label: '投递次数', kind: 'u64', width: 110, defaultValue: '0' },
    { key: 'handled', label: '已处理', kind: 'bool', width: 90, searchable: true, defaultValue: false, inForm: false },
    {
      key: 'simulated',
      label: '模拟',
      kind: 'bool',
      width: 90,
      searchable: true,
      inForm: false,
      tip: '原始负载带 simulated=true 标记时为是',
    },
    { key: 'payload', label: '原始负载', kind: 'textarea', rows: 8, required: true, inTable: false },
    { key: 'remark', label: '备注', kind: 'text', maxLength: 50, inTable: false },
  ],
  /**
   * 标记已处理
   *
   * 死信只读，排查完后不再需要改其它列。这个动作只写 `handled = true`，
   * 原始负载与转入原因都保持原样。
   */
  extraActions: (record, ctx) =>
    record.handled ? null : (
      <Button
        type="link"
        size="small"
        onClick={() =>
          ctx.confirm({
            title: '把这条死信标记为已处理？',
            content: '只把「已处理」置为是，原始负载、转入原因与投递次数都保持不变。',
            okText: '标记已处理',
            onOk: async () => {
              try {
                await deadLetterApi.markHandled(record.id);
                ctx.notify('success', '已标记为已处理');
                ctx.reload();
              } catch (reason) {
                ctx.notify('error', errorMessage(reason));
                // 抛回给 Modal：失败时保持弹窗，用户可以重试
                throw reason;
              }
            },
          })
        }
      >
        标记已处理
      </Button>
    ),
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
  hideUpdateTime: true,
  fields: [
    { key: 'code', label: '编码（主题）', kind: 'text', width: 240, required: true, searchable: true, maxLength: 50 },
    { key: 'name', label: '名称', kind: 'text', width: 200, required: true, searchable: true, maxLength: 50 },
    { key: 'persisted', label: '持久化', kind: 'bool', defaultValue: true, dbDefault: true, inTable: false, tip: '持久化走 JetStream（等 ACK、重启补消费）；不持久化走 Core NATS，发出即不管送达' },
    { key: 'remark', label: '备注', kind: 'text', maxLength: 50 },
  ],
};

/** # 投递队列 */
const deliveryQueue: EntityConfig = {
  key: 'delivery-queue',
  path: '/msg/delivery-queue',
  name: '投递队列',
  title: '投递队列',
  desc: '每个渠道一条投递主题，编码固定为 `msg.delivery.<渠道类型>`，服务端会校验前缀与渠道是否存在',
  icon: <BarsOutlined />,
  group: 'queue',
  hideUpdateTime: true,
  fields: [
    {
      key: 'code',
      label: '编码（主题）',
      kind: 'text',
      width: 280,
      required: true,
      searchable: true,
      maxLength: 50,
      tip: '末段必须等于某个渠道类型，渠道从主题反解，本表不存渠道 ID',
    },
    { key: 'name', label: '名称', kind: 'text', width: 200, required: true, searchable: true, maxLength: 50 },
    { key: 'persisted', label: '持久化', kind: 'bool', defaultValue: false, dbDefault: true, inTable: false },
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
  messageTargetChannel,
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
