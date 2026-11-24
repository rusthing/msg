/**
 * # 渠道类型 → 配置字段
 *
 * `msg_channel.options` 是按渠道类型自定义的 JSON，后端每种渠道用各自的
 * `Deserialize` 结构解析。这里把同一套字段声明成表单 schema，控制台按
 * `channelType` 渲染图形化表单，不再让运营手写 JSON。
 *
 * 约定：
 * - `key` 必须是后端 `options` 里的 camelCase 字段名（与各 sender 的
 *   `#[serde(rename_all = "camelCase")]` 一致）；
 * - 字段全部可选，留空表示「用后端默认值」，所以表单只写用户改过的键；
 * - 未登记的类型（运营自己加的 `channel_type`）没有 schema，回退到原始 JSON
 *   编辑器：后端没有发送器时投递会报「渠道未实现」，前端不拦着配置。
 */

export type ChannelOptionKind =
  | 'text'
  | 'secret'
  | 'number'
  | 'bool'
  | 'select'
  | 'textarea'
  | 'json';

export interface ChannelOptionField {
  /** options JSON 里的键（camelCase） */
  key: string;
  label: string;
  kind: ChannelOptionKind;
  /** 占位符（一般是后端的默认值） */
  placeholder?: string;
  /** 字段说明 */
  tip?: string;
  /** 多行文本行数 */
  rows?: number;
  /** 数字范围 */
  min?: number;
  max?: number;
  /** 下拉选项（`kind = select` 时必填） */
  options?: { label: string; value: string }[];
}

export interface ChannelOptionSchema {
  type: string;
  label: string;
  desc: string;
  fields: ChannelOptionField[];
}

/** 内置渠道类型（与后端 `ChannelCode` 一致） */
export const CHANNEL_TYPES: { label: string; value: string }[] = [
  { label: 'websocket（大屏长连接）', value: 'websocket' },
  { label: 'nats（Core NATS 主题）', value: 'nats' },
  { label: 'sms（短信网关）', value: 'sms' },
  { label: 'email（SMTP 邮件）', value: 'email' },
];

const WEBSOCKET: ChannelOptionSchema = {
  type: 'websocket',
  label: 'websocket',
  desc: '大屏长连接：握手令牌在这里配，报文结构看「载荷模板」；服务哪个目标类别由「目标类别-渠道绑定」决定，这里不重复配',
  fields: [
    {
      key: 'token',
      label: '握手令牌',
      kind: 'secret',
      placeholder: '留空表示不校验（联调用）',
      tip: '大屏连接 `/msg/ws?targetCategory=screen&token=xxx` 时带上；令牌只对「绑定了这条渠道的目标类别」生效，另一种业务的大屏应另建目标类别并另绑一条渠道',
    },
  ],
};

const NATS: ChannelOptionSchema = {
  type: 'nats',
  label: 'nats',
  desc: '把投递任务发到一条 Core NATS 主题，报文帧由「载荷模板」决定，本发送器不认识任何业务协议',
  fields: [
    {
      key: 'subject',
      label: 'NATS 主题',
      kind: 'text',
      placeholder: 'dtw.card.callback',
      tip: '必填。主题与报文帧由订阅方（如 dtw）注册这条渠道时写入；留空时这条渠道的每次投递都会判失败',
    },
  ],
};

const SMS: ChannelOptionSchema = {
  type: 'sms',
  label: 'sms',
  desc: '短信网关逐条下发：报文与成功判定都配在这里，换网关不用发版',
  fields: [
    {
      key: 'gatewayUrl',
      label: '网关地址',
      kind: 'text',
      placeholder: 'http://sms-gw:8080/api/send',
      tip: '必填，留空时这条渠道的每次投递都会判失败',
    },
    {
      key: 'method',
      label: 'HTTP 方法',
      kind: 'select',
      options: [
        { label: 'POST', value: 'POST' },
        { label: 'GET', value: 'GET' },
        { label: 'PUT', value: 'PUT' },
      ],
      placeholder: 'POST',
    },
    {
      key: 'contentType',
      label: 'Content-Type',
      kind: 'text',
      placeholder: 'application/json',
    },
    {
      key: 'headers',
      label: '附加请求头',
      kind: 'json',
      rows: 3,
      placeholder: '{"Authorization":"Bearer xxx"}',
      tip: 'JSON 对象，鉴权、租户标识这类头；留空表示不加',
    },
    {
      key: 'bodyTemplate',
      label: '请求体模板',
      kind: 'textarea',
      rows: 3,
      placeholder: '留空即 {"mobile":"…","content":"…"}',
      tip: '变量：{{mobile}} {{content}} {{title}} {{eventCode}} {{sign}} {{appId}} {{appKey}} {{templateId}}，缺失的变量注入空串',
    },
    {
      key: 'successField',
      label: '成功判定字段',
      kind: 'text',
      placeholder: 'code',
      tip: '响应里表示结果的字段，支持 data.status 这类点路径；留空表示 HTTP 2xx 即成功',
    },
    {
      key: 'successValue',
      label: '成功判定值',
      kind: 'text',
      placeholder: '0',
      tip: '与响应字段做字符串比较（数字、布尔也会转成字符串）',
    },
    {
      key: 'timeoutMs',
      label: '超时（毫秒）',
      kind: 'number',
      min: 1000,
      max: 60000,
      placeholder: '5000',
    },
    { key: 'appId', label: '网关账号', kind: 'text', placeholder: '网关分配的账号' },
    { key: 'appKey', label: '网关密钥', kind: 'secret', placeholder: '网关分配的密钥' },
    { key: 'sign', label: '短信签名', kind: 'text', placeholder: '【XX系统】' },
    {
      key: 'templateId',
      label: '模板编号',
      kind: 'text',
      placeholder: '留空表示按正文直发',
      tip: '模板短信的模板编号，报文里用 {{templateId}} 取',
    },
  ],
};

const EMAIL: ChannelOptionSchema = {
  type: 'email',
  label: 'email',
  desc: 'SMTP 直发明文邮件：当前构建不支持 STARTTLS / 465 隐式 TLS',
  fields: [
    {
      key: 'smtpHost',
      label: 'SMTP 主机',
      kind: 'text',
      placeholder: 'mail.example.com',
      tip: '必填，留空时每次投递都会判失败',
    },
    {
      key: 'smtpPort',
      label: 'SMTP 端口',
      kind: 'number',
      min: 1,
      max: 65535,
      placeholder: '25',
      tip: '常见 25 或 587（明文）；465 是隐式 TLS，配上去会直接判失败',
    },
    { key: 'username', label: '认证账号', kind: 'text', placeholder: 'notice@example.com' },
    { key: 'password', label: '认证密码', kind: 'secret', placeholder: '留空表示不需要认证' },
    {
      key: 'fromAddress',
      label: '发件地址',
      kind: 'text',
      placeholder: '留空取认证账号',
      tip: '信封 MAIL FROM 与头 From 都用它',
    },
    { key: 'fromName', label: '发件人显示名', kind: 'text', placeholder: '消息中心' },
    { key: 'contentType', label: '正文类型', kind: 'text', placeholder: 'text/plain', tip: '发 HTML 邮件填 text/html' },
    {
      key: 'subjectTemplate',
      label: '主题模板',
      kind: 'textarea',
      rows: 2,
      placeholder: '留空即消息标题',
      tip: '变量：{{title}} {{content}} {{eventCode}}；字符串变量写 {{var:json}} 才有引号',
    },
    {
      key: 'bodyTemplate',
      label: '正文模板',
      kind: 'textarea',
      rows: 4,
      placeholder: '留空即消息内容',
      tip: '可用 HTML；变量同上',
    },
    { key: 'timeoutMs', label: '超时（毫秒）', kind: 'number', min: 1000, max: 60000, placeholder: '10000' },
    {
      key: 'requireTls',
      label: '要求 TLS',
      kind: 'bool',
      placeholder: '否',
      tip: '当前构建没有 TLS 栈，打开后这条渠道的投递会直接失败（用来显式暴露配置错误）',
    },
  ],
};

/** 类型 → schema；未登记的类型返回 `undefined`（回退原始 JSON） */
export function channelOptionSchema(type: string | undefined): ChannelOptionSchema | undefined {
  switch (type) {
    case 'websocket':
      return WEBSOCKET;
    case 'nats':
      return NATS;
    case 'sms':
      return SMS;
    case 'email':
      return EMAIL;
    default:
      return undefined;
  }
}
