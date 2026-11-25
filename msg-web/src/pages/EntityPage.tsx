/**
 * # 通用 CRUD 页面
 *
 * 14 张表的路由、分页参数、表单形状完全一致，差别只在字段，所以这里按 {@link EntityConfig}
 * 渲染一整套「筛选 + 列表 + 表单 + 详情」，每个实体的 `pages/*` 只剩一行配置。
 *
 * ## 三个和后端语义相关的注意点
 *
 * 1. **可选字段默认清空无效**：ModifyDto 用「字段不出现」表达「不改」。只有标了
 *    `nullable` 的模板字段会用「清空」按钮提交 JSON `null`，把列真正置为 NULL
 *    （后端 `#[crud_dto]` 给 `Option<String>` 挂了三态 serde）。其余字段留空仍是「不修改」。
 * 2. **`u64` 一律按字符串提交**：ID、`businessId`、时间戳在 JSON 里必须是字符串，
 *    输入框用文本 + 数字校验，不用 `InputNumber`（它会给出 number）。
 * 3. **筛选是等值匹配**：只有「关键字」走服务端模糊搜索，其余条件都是 `=`。
 *    时间范围筛选需要 QueryDto 支持区间参数，当前后端契约未提供，前端不伪造。
 */

import {
  App as AntdApp,
  Alert,
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Empty,
  Form,
  Grid,
  Input,
  Row as AntRow,
  Select,
  Space,
  Switch,
  Table,
  Tabs,
  Tag,
  Tooltip,
  Typography,
} from 'antd';
import { Modal, Drawer } from '../lib/federated-overlay';
import type { FormRule } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { PlusOutlined, SearchOutlined, UndoOutlined } from '@ant-design/icons';
import dayjs, { type Dayjs } from 'dayjs';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { MouseEvent as ReactMouseEvent, ReactNode } from 'react';
import { crudOf } from '../api/crud';
import type { QueryParams } from '../api/client';
import type { Row as MsgRow } from '../api/types';
import BoolSwitch from '../components/BoolSwitch';
import ChannelOptionsEditor from '../components/ChannelOptionsEditor';
import JsonEditor from '../components/JsonEditor';
import JsonText from '../components/JsonText';
import OptionButtons from '../components/OptionButtons';
import StatusTag from '../components/StatusTag';
import TemplateEditor from '../components/TemplateEditor';
import type { DirectoryBundle } from '../api/directory';
import type { EntitySelectOption } from '../components/EntitySelect';
import { ManagedPicker } from '../components/ManagedPicker';
import type { EntityConfig, FieldConfig, RowActionContext } from '../entities';
import { dicOptions } from '../dic';
import { usePagedQuery } from '../hooks/usePagedQuery';
import { useDirectoryOptions } from '../hooks/useDirectoryOptions';
import { useRefOptions, type RefOptions } from '../hooks/useRefOptions';
import {
  EMPTY,
  errorMessage,
  formatMs,
  orDash,
  prettyJson,
  shortId,
} from '../utils/format';
import { validateJsonTemplate, validateJsonText, validateTemplate } from '../utils/validation';

/** 表单值：日期控件给 Dayjs，其余按原样存 */
type FormValues = Record<string, unknown>;

/** 筛选草稿：值是输入框/下拉给的原始类型 */
type FilterDraft = Record<string, string | number | boolean | undefined>;

/** 宽控件占满一行：多行文本、模板与 JSON 挤在半栏里没法编辑 */
function isWideField(field: FieldConfig): boolean {
  return (
    field.kind === 'textarea' ||
    field.kind === 'template' ||
    field.kind === 'jsonTemplate' ||
    field.kind === 'json' ||
    field.kind === 'channelOptions'
  );
}

/** 是否需要按字符串做 trim 校验（数字字典、布尔、时间不适用） */
function isTextLike(field: FieldConfig): boolean {
  return (
    field.kind === 'text' ||
    field.kind === 'textarea' ||
    field.kind === 'template' ||
    field.kind === 'jsonTemplate' ||
    field.kind === 'json' ||
    field.kind === 'channelOptions' ||
    field.kind === 'subject' ||
    field.kind === 'u64'
  );
}

/**
 * # 记录 → 表单值
 *
 * `ms` 字段从 `u64` 字符串转成 Dayjs（DatePicker 只认 Dayjs），布尔补齐默认值，
 * 其余原样塞进去；`Option` 字段缺失时留 `undefined`，控件显示占位符。
 */
function toFormValues(fields: FieldConfig[], record: MsgRow | null): FormValues {
  const values: FormValues = {};
  for (const field of fields) {
    if (field.inForm === false) {
      continue;
    }
    const raw = record?.[field.key];
    if (field.kind === 'ms') {
      values[field.key] = raw ? dayjs(Number(raw)) : undefined;
    } else if (field.kind === 'bool') {
      values[field.key] = raw === undefined ? Boolean(field.defaultValue) : Boolean(raw);
    } else if (raw === undefined || raw === null) {
      values[field.key] = field.defaultValue;
    } else {
      values[field.key] = raw;
    }
  }
  return values;
}

/** 提交体的组装选项：区分新增/编辑，并判断用户是否动过某个字段 */
interface SubmitOptions {
  isAdd: boolean;
  isTouched: (key: string) => boolean;
}

/**
 * # 表单值 → 提交体
 *
 * - 新增时，`dbDefault` 字段没被用户动过就不提交，交给数据库默认值兜底；
 * - `ms`：Dayjs → 毫秒字符串（`u64`），日期控件清空时不提交；
 * - `bool`：始终提交（`false` 是有意义的值）；
 * - `nullable` 字段被用户动过且清空时提交 `null`（服务端会写成 SQL NULL）；
 * - 其余空串/`undefined` 跳过，交给服务端的 `Option` 语义。
 */
function toSubmitBody(
  fields: FieldConfig[],
  values: FormValues,
  options: SubmitOptions,
): Record<string, unknown> {
  const body: Record<string, unknown> = {};
  for (const field of fields) {
    if (field.inForm === false) {
      continue;
    }
    // 新增时状态字段保持缺省，让数据库的 DEFAULT 生效；用户改过才提交
    if (options.isAdd && field.dbDefault && !options.isTouched(field.key)) {
      continue;
    }
    const value = values[field.key];
    if (field.kind === 'ms') {
      if (value) {
        body[field.key] = String((value as Dayjs).valueOf());
      }
      continue;
    }
    if (field.kind === 'bool') {
      body[field.key] = Boolean(value);
      continue;
    }
    // 三态：没动过的空值跳过（= 不修改）；动过且清空 → 显式 null（= 置 NULL）。
    // 新增时列本来就是 NULL，不必多写一次。
    if (field.nullable && (value === null || value === '')) {
      if (!options.isAdd && options.isTouched(field.key)) {
        body[field.key] = null;
      }
      continue;
    }
    if (value === undefined || value === null || (value === '' && !field.allowEmptySubmit)) {
      continue;
    }
    body[field.key] = value;
  }
  return body;
}

/**
 * # 「模板」区域里单个 tab 的内容
 *
 * 外层 Tabs 用 `forceRender` 让三个 Form.Item 始终挂载，否则没点开过的那一页
 * 不会注册校验规则，保存时 `validateFields()` 会漏掉它。
 *
 * 「清空」直接把表单值设成 `null` 并标记 touched：`toSubmitBody` 据此区分
 * 「用户主动删除」与「字段没动过」，前者提交 `null` 由后端写成 SQL NULL。
 */
function TemplateTabPane({
  field,
  rules,
  children,
}: {
  field: FieldConfig;
  rules: FormRule[];
  children: ReactNode;
}) {
  const form = Form.useFormInstance();
  const value = Form.useWatch(field.key, form);
  const configured = value !== undefined && value !== null && String(value).trim() !== '';
  return (
    <>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap', marginBottom: 8 }}>
        <Tag color={configured ? 'blue' : 'default'} style={{ marginInlineEnd: 0 }}>
          {configured ? '已配置' : '未配置（NULL）'}
        </Tag>
        <Button
          size="small"
          disabled={!configured}
          onClick={() => {
            form.setFields([{ name: field.key, value: null, touched: true, errors: [] }]);
          }}
        >
          清空（置 NULL）
        </Button>
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          清空后保存会删除已保存的模板
        </Typography.Text>
      </div>
      <Form.Item
        name={field.key}
        rules={rules}
        // 输入过程中不校验，避免刚敲一个 `{` 就整片标红；失焦和保存时仍会校验
        validateTrigger={['onBlur']}
        extra={field.tip}
        style={{ marginBottom: 0 }}
      >
        {children}
      </Form.Item>
    </>
  );
}

/** # 长文本单元格：单行省略，悬停看全文 */
function LongText({ value }: { value: string }) {
  return (
    <Typography.Text className="msg-long-text" ellipsis={{ tooltip: value }}>
      {value}
    </Typography.Text>
  );
}

/** # 外键 ID → 名称（查不到就退回短 ID，完整 ID 放 tooltip） */
function refLabel(ref: RefOptions | undefined, value: unknown) {
  const id = value === undefined || value === null ? '' : String(value);
  const label = ref?.labels.get(id);
  if (label) {
    return <Tooltip title={`${label}（${id}）`}>{label}</Tooltip>;
  }
  const row = ref?.byId.get(id);
  const name = row?.name;
  if (typeof name === 'string' && name) {
    return <Tooltip title={`${name}（${id}）`}>{name}</Tooltip>;
  }
  const code = row?.code;
  if (typeof code === 'string' && code) {
    return <Tooltip title={`${code}（${id}）`}>{code}</Tooltip>;
  }
  if (!id) {
    return <span className="msg-mono">{EMPTY}</span>;
  }
  return (
    <Tooltip title={id}>
      <span className="msg-mono">{shortId(id)}</span>
    </Tooltip>
  );
}

/** # 单元格渲染 */
function renderCell(
  field: FieldConfig,
  record: MsgRow,
  refOptions: Record<string, RefOptions>,
): ReactNode {
  const value = record[field.key];
  switch (field.kind) {
    case 'ms':
      return <span className="msg-mono msg-nowrap">{formatMs(value as string)}</span>;
    case 'bool':
      return value ? <Tag color="success">是</Tag> : <Tag>否</Tag>;
    case 'dic':
      return <StatusTag dic={field.dic ?? []} value={value as number} />;
    case 'ref':
      return refLabel(field.ref ? refOptions[field.ref] : undefined, value);
    case 'json':
    case 'channelOptions':
      return <JsonText value={value} />;
    case 'textarea':
    case 'template':
    case 'jsonTemplate':
      return value ? <LongText value={String(value)} /> : EMPTY;
    case 'u64': {
      const text = orDash(value as string);
      return (
        <Tooltip title={text}>
          <span className="msg-mono">{shortId(text)}</span>
        </Tooltip>
      );
    }
    default:
      if (value === '' && field.emptyLabel) {
        return field.emptyLabel;
      }
      return orDash(value as string);
  }
}

/**
 * # 详情里的字段值
 *
 * 与表格的差别在长文本和 ID：JSON 展开成格式化文本、模板原样铺开、ID 保留完整值并可复制。
 */
function renderDetailValue(
  field: FieldConfig,
  record: MsgRow,
  refOptions: Record<string, RefOptions>,
): ReactNode {
  const value = record[field.key];
  if (field.kind === 'json' || field.kind === 'channelOptions') {
    return value ? <pre className="msg-json-view">{prettyJson(String(value))}</pre> : EMPTY;
  }
  if (field.kind === 'textarea' || field.kind === 'template' || field.kind === 'jsonTemplate') {
    return value ? <pre className="msg-text-view">{String(value)}</pre> : EMPTY;
  }
  if (field.kind === 'u64') {
    const text = orDash(value as string);
    if (text === EMPTY) {
      return EMPTY;
    }
    return (
      <Typography.Text className="msg-mono msg-break" copyable>
        {text}
      </Typography.Text>
    );
  }
  return renderCell(field, record, refOptions);
}

/** 把外键选项折成选择面板需要的目录结构：无左树、无分组，只有可搜索的表格行。 */
function refBundle(options: EntitySelectOption[]): DirectoryBundle {
  const entries = options
    .filter((option) => option.value !== '')
    .map((option) => ({
      id: option.value,
      name: option.label,
      label: option.label,
      detail: option.detail ?? `#${option.value}`,
      enabled: !option.disabled,
    }));
  return { entries, tree: [], groupsByEntry: new Map() };
}

export interface EntityPageProps {
  config: EntityConfig;
  /**
   * 固定查询条件（主从页里子表按外键过滤）：查询时强制带上，筛选区与表单里
   * 对应的字段隐藏，新增时自动写入提交体。普通实体页不传。
   */
  fixedFilters?: QueryParams;
  /** 单选模式：表格左侧出现单选框，点击行也能选中，用于「选主记录 → 看子记录」 */
  selectable?: {
    selectedId?: string;
    onSelect: (row: MsgRow) => void;
  };
  /** 分区标题（合并维护页里区分上下两块） */
  sectionTitle?: ReactNode;
  /** 删除成功回调：主表删除当前选中行时，外层据此清空子表选择 */
  onDeleted?: (row: MsgRow) => void;
}

export default function EntityPage({
  config,
  fixedFilters,
  selectable,
  sectionTitle,
  onDeleted,
}: EntityPageProps) {
  const { message, modal } = AntdApp.useApp();
  const screens = Grid.useBreakpoint();
  // 首帧 breakpoint 还没算出来，按桌面处理；窄屏下再取消操作列固定，避免固定列吃掉大半屏
  const isDesktop = screens.lg !== false;
  const api = useMemo(() => crudOf(config.path), [config.path]);
  const fixedKeys = useMemo(() => Object.keys(fixedFilters ?? {}), [fixedFilters]);

  const { data, loading, error, pageNum, pageSize, search, reload, setPageNum, setPageSize } =
    usePagedQuery<MsgRow>(
      (params) => api.page(params),
      // 首屏也要带默认排序，否则第一页与「查询」之后的顺序不一致
      {
        ...fixedFilters,
        ...(config.defaultOrderBy ? { _orderBy: config.defaultOrderBy } : {}),
      },
    );

  const [draft, setDraft] = useState<FilterDraft>({});
  const [keyword, setKeyword] = useState('');
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<MsgRow | null>(null);
  const [saving, setSaving] = useState(false);
  const [detail, setDetail] = useState<MsgRow | null>(null);
  const [form] = Form.useForm<FormValues>();

  /** 表单里出现的外键，一次性把选项拉齐 */
  const refKeys = useMemo(
    () =>
      config.fields
        .filter((field) => field.kind === 'ref' && field.ref && !fixedKeys.includes(field.key))
        .map((field) => field.ref!),
    [config.fields, fixedKeys],
  );
  // 当前页记录一并交给选项层：批量映射没覆盖到的外键 ID 会按主键补查，
  // 避免生产环境「消息类别」这类列退化成一串短 ID
  const refOptions = useRefOptions(refKeys, data.list, config.fields);

  /**
   * 目标主体字段的目录来源：由同表单的类别字段（如消息目标的「目标类别」）
   * 取出类别编码，再映射到用户 / 角色 / 组织目录。
   */
  const subjectField = useMemo(
    () => config.fields.find((field) => field.kind === 'subject'),
    [config.fields],
  );
  const subjectByField = useMemo(
    () => config.fields.find((field) => field.key === subjectField?.subjectBy),
    [config.fields, subjectField],
  );
  const subjectByValue = Form.useWatch(subjectField?.subjectBy ?? '__subject_by_none__', form);
  const subjectCategoryRow =
    subjectByField?.ref && subjectByValue !== undefined && subjectByValue !== null
      ? refOptions[subjectByField.ref]?.byId.get(String(subjectByValue))
      : undefined;
  const subjectCategoryCode = subjectCategoryRow ? String(subjectCategoryRow.code ?? '') : '';
  const subjectSource = subjectCategoryCode
    ? subjectField?.subjectSources?.[subjectCategoryCode]
    : undefined;
  const directory = useDirectoryOptions(subjectSource, formOpen);

  // 类别声明了「目标标识键」时，标识由事件标签决定，配置里不保存历史值
  const subjectTargetKeys =
    subjectCategoryRow && subjectCategoryRow.targetKeys
      ? String(subjectCategoryRow.targetKeys).trim()
      : '';
  useEffect(() => {
    if (formOpen && subjectField && subjectTargetKeys) {
      form.setFieldValue(subjectField.key, '');
    }
  }, [formOpen, subjectTargetKeys, subjectField, form]);

  /** 筛选区同样按「目标类别」切换目录，让筛选里的目标标识也能选 */
  const filterSubjectByValue = subjectByField ? draft[subjectByField.key] : undefined;
  const filterCategoryRow =
    subjectByField?.ref && filterSubjectByValue !== undefined && filterSubjectByValue !== null
      ? refOptions[subjectByField.ref]?.byId.get(String(filterSubjectByValue))
      : undefined;
  const filterCategoryCode = filterCategoryRow ? String(filterCategoryRow.code ?? '') : '';
  const filterSubjectSource = filterCategoryCode
    ? subjectField?.subjectSources?.[filterCategoryCode]
    : undefined;
  const filterDirectory = useDirectoryOptions(filterSubjectSource, true);

  const searchFields = useMemo(
    () => config.fields.filter((field) => field.searchable && !fixedKeys.includes(field.key)),
    [config.fields, fixedKeys],
  );
  const formFields = useMemo(
    () => config.fields.filter((field) => field.inForm !== false && !fixedKeys.includes(field.key)),
    [config.fields, fixedKeys],
  );
  /** 成组模板字段抽出来放进「模板」Card 的 Tabs，其余字段仍按两列平铺 */
  const templateFields = useMemo(
    () => formFields.filter((field) => field.templateGroup),
    [formFields],
  );
  const normalFields = useMemo(
    () => formFields.filter((field) => !field.templateGroup),
    [formFields],
  );
  const [activeTemplateTab, setActiveTemplateTab] = useState('');

  /** 把草稿 + 关键字翻成 QueryDto 的查询参数；空值不传，避免变成等值条件 */
  const applyFilters = (next: FilterDraft, nextKeyword: string) => {
    const params: QueryParams = {
      // 固定条件始终跟着查询走，重置也删不掉（主从页的子表只属于当前主记录）
      ...fixedFilters,
      _keyword: nextKeyword.trim() || undefined,
      // 排序跟着条件一起走：重置之后仍然是「最新的在最上面」
      _orderBy: config.defaultOrderBy,
    };
    for (const field of searchFields) {
      const value = next[field.key];
      if (value === undefined || value === '') {
        continue;
      }
      // antd 的 Select 给字符串；服务端 QueryDto 是 bool，转回去避免依赖 serde 的宽松解析
      params[field.key] = field.kind === 'bool' ? value === 'true' || value === true : value;
    }
    search(params);
  };

  const resetFilters = () => {
    setDraft({});
    setKeyword('');
    applyFilters({}, '');
  };

  /** 固定条件本身变了（如主从页切到另一条消息）：保留当前筛选，但按新条件重查 */
  const fixedSignature = JSON.stringify(fixedFilters ?? {});
  const lastFixed = useRef(fixedSignature);
  useEffect(() => {
    if (lastFixed.current === fixedSignature) {
      return;
    }
    lastFixed.current = fixedSignature;
    applyFilters(draft, keyword);
    // applyFilters/draft/keyword 每次渲染都是新的，这里只关心固定条件的签名
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fixedSignature]);

  /**
   * 把值灌进表单
   *
   * antd 的 `setFieldsValue` 泛型是 `RecursivePartial<Values>`，与这里的通用
   * `Record<string, unknown>` 对不上；值的形状由 `toFormValues` 保证，转一次类型即可。
   */
  const applyFormValues = (values: FormValues) => {
    form.setFieldsValue(values as never);
  };

  const openAdd = () => {
    setEditing(null);
    form.resetFields();
    applyFormValues(toFormValues(formFields, null));
    setFormOpen(true);
  };

  const openEdit = (record: MsgRow) => {
    setEditing(record);
    form.resetFields();
    applyFormValues(toFormValues(formFields, record));
    setFormOpen(true);
  };

  const handleSubmit = async () => {
    let values: FormValues;
    try {
      values = await form.validateFields();
    } catch (info) {
      // 校验失败由 Form.Item 自己标红；若错在未点开的模板页，先切过去让用户看得见
      const errorFields = (info as { errorFields?: { name?: (string | number)[] }[] }).errorFields;
      const failedKey = errorFields?.[0]?.name?.[0];
      if (typeof failedKey === 'string' && templateFields.some((field) => field.key === failedKey)) {
        setActiveTemplateTab(failedKey);
      }
      return;
    }

    const body = {
      ...fixedFilters,
      ...toSubmitBody(formFields, values, {
        isAdd: !editing,
        isTouched: (key) => form.isFieldTouched(key),
      }),
    };
    setSaving(true);
    try {
      if (editing) {
        await api.modify({ ...body, id: editing.id });
        message.success('修改成功');
      } else {
        await api.add(body);
        message.success('新增成功');
      }
      setFormOpen(false);
      reload();
    } catch (reason) {
      message.error(errorMessage(reason));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = (record: MsgRow) => {
    modal.confirm({
      title: `删除这条${config.name}？`,
      content: '删除后不可恢复；被其他表引用时服务端会拒绝删除，请先处理引用它的记录。',
      okText: '删除',
      okButtonProps: { danger: true },
      cancelText: '取消',
      onOk: async () => {
        try {
          await api.remove(record.id);
          message.success('删除成功');
          onDeleted?.(record);
          reload();
        } catch (reason) {
          message.error(errorMessage(reason));
          // 抛回给 Modal：删除失败时保持弹窗，用户可以重试
          throw reason;
        }
      },
    });
  };

  /** 表格里直接切开关：只改这一个字段 */
  const toggleField = async (record: MsgRow, field: FieldConfig, next: boolean) => {
    await api.modify({ id: record.id, [field.key]: next });
    reload();
  };

  /**
   * 行内附加操作要用到的三件事
   *
   * 元数据只描述「这个动作是什么」，刷新 / 提示 / 确认必须由页面提供：
   * 那里才有 `usePagedQuery` 的 `reload`、antd 的 message 与 modal。
   */
  const rowActionContext: RowActionContext = {
    reload,
    notify: (type, text) => {
      if (type === 'success') {
        message.success(text);
      } else {
        message.error(text);
      }
    },
    confirm: (options) => {
      modal.confirm({
        title: options.title,
        content: options.content,
        okText: options.okText ?? '确定',
        cancelText: '取消',
        onOk: options.onOk,
      });
    },
  };

  const columns = useMemo<ColumnsType<MsgRow>>(() => {
    const result: ColumnsType<MsgRow> = [];

    if (config.showId) {
      result.push({
        title: 'ID',
        dataIndex: 'id',
        width: 150,
        render: (value: string) => (
          <Tooltip title={value}>
            <span className="msg-mono">{shortId(value)}</span>
          </Tooltip>
        ),
      });
    }

    // 启用类开关（启用 / 允许模拟）统一挪到「操作」列紧前面：状态操作离行尾
    // 按钮最近，与 alarm / dtw 等前端的列表一致。
    // 持久化已移出列表，只在编辑弹窗里改：它会改变消息链路语义，不该在列表上误触。
    const trailingSwitches: ColumnsType<MsgRow> = [];

    for (const field of config.fields) {
      // 固定条件下这一列每行都一样（子表只属于当前主记录），不重复展示
      if (field.inTable === false || fixedKeys.includes(field.key)) {
        continue;
      }
      const center = field.kind === 'dic' || field.kind === 'bool';
      const column: ColumnsType<MsgRow>[number] = {
        title: field.label,
        dataIndex: field.key,
        width: field.width,
        // 长文本由 LongText 自己处理省略与 tooltip，避免和 Table 的 ellipsis 叠加；
        // 单行文本统一交给 Table 省略（悬停显示完整值），避免 50 字编码把列撑破
        ellipsis: field.kind === 'text',
        align: center ? 'center' : undefined,
        render: (_: unknown, record: MsgRow) => {
          if (field.kind === 'bool' && field.switchable) {
            return (
              <span className="msg-switch-hit">
                <BoolSwitch
                  value={Boolean(record[field.key])}
                  onToggle={(next) => toggleField(record, field, next)}
                />
              </span>
            );
          }
          return renderCell(field, record, refOptions);
        },
      };
      if (field.switchable && (field.key === 'enabled' || field.key === 'simEnabled')) {
        trailingSwitches.push(column);
      } else {
        result.push(column);
      }
    }

    for (const extra of config.extraColumns ?? []) {
      result.push(extra);
    }

    if (!config.hideUpdateTime) {
      result.push({
        title: '更新时间',
        dataIndex: 'updateMs',
        width: 180,
        render: (value: string) => <span className="msg-mono msg-nowrap">{formatMs(value)}</span>,
      });
    }

    result.push(...trailingSwitches);

    result.push({
      title: '操作',
      key: 'actions',
      // 附加操作会多出一两个链接，150 塞不下四个就换行挤压，命中率很低
      width: config.extraActions ? 220 : 150,
      fixed: isDesktop ? 'right' : undefined,
      render: (_: unknown, record: MsgRow) => (
        <Space size={0} className="msg-table-actions">
          {config.extraActions?.(record, rowActionContext)}
          <Button type="link" size="small" onClick={() => setDetail(record)}>
            详情
          </Button>
          {config.readOnly ? null : (
            <>
              <Button type="link" size="small" onClick={() => openEdit(record)}>
                编辑
              </Button>
              <Button type="link" size="small" danger onClick={() => handleDelete(record)}>
                删除
              </Button>
            </>
          )}
        </Space>
      ),
    });

    return result;
    // 依赖里刻意不放 `api`：它按 path 记忆化，path 变了 config 也会变
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [config, isDesktop, refOptions, fixedKeys]);

  const scrollX = useMemo(
    () =>
      columns.reduce((sum, column) => sum + (typeof column.width === 'number' ? column.width : 160), 0) +
      // 单选列由 antd 自己插入，算进横向滚动宽度，避免操作列被挤出可视区
      (selectable ? 48 : 0),
    [columns, selectable],
  );

  /** 筛选控件：等值匹配，控件类型跟着字段走 */
  const renderFilter = (field: FieldConfig) => {
    const value = draft[field.key];
    const common = { allowClear: true, className: 'msg-filter-control', placeholder: field.label };
    /**
     * 选择类控件改变即查询：先把 patch 合成完整的新草稿，再同时更新 state 与发请求。
     * 不能在 setDraft 的 updater 里读旧 draft（异步），必须显式用 nextDraft 查。
     */
    const commit = (patch: FilterDraft) => {
      const nextDraft = { ...draft, ...patch };
      setDraft(nextDraft);
      applyFilters(nextDraft, keyword);
    };

    if (field.kind === 'select') {
      return (
        <Select
          {...common}
          value={value === undefined || value === null ? undefined : String(value)}
          options={field.options}
          onChange={(next) => commit({ [field.key]: next })}
        />
      );
    }
    if (field.kind === 'dic') {
      const options = dicOptions(field.dic ?? []);
      // 规范：0–1 个选项不渲染选择控件，2–5 个用按钮组，≥6 个仍用下拉。
      // 标了 `filterAsSelect` 的字段跳过这套规则，固定下拉（状态类枚举铺成按钮组太占地方）。
      if (options.length <= 1 && !field.filterAsSelect) {
        return <span className="msg-filter-static">{options[0]?.label ?? '无'}</span>;
      }
      if (options.length <= 5 && !field.filterAsSelect) {
        return (
          <div className="msg-filter-group">
            <span className="msg-filter-label">{field.label}</span>
            <OptionButtons
              allowClear
              ariaLabel={field.label}
              value={value as number | undefined}
              options={options}
              onChange={(next) => commit({ [field.key]: next })}
            />
          </div>
        );
      }
      return (
        <Select
          {...common}
          value={value as number | undefined}
          options={options}
          onChange={(next) => commit({ [field.key]: next })}
        />
      );
    }
    if (field.kind === 'ref') {
      const ref = field.ref ? refOptions[field.ref] : undefined;
      return (
        <ManagedPicker
          className="msg-filter-control"
          bundle={refBundle((ref?.options ?? []) as EntitySelectOption[])}
          loading={ref?.loading}
          title={`筛选${field.label}`}
          placeholder={field.label}
          nameLabel={field.label}
          value={value === undefined || value === null ? '' : String(value)}
          onChange={(next) => {
            const patch: FilterDraft = { [field.key]: next || undefined };
            // 目标类别变了，旧的「目标标识」筛选条件不再适用
            if (subjectField?.subjectBy === field.key) {
              patch[subjectField.key] = undefined;
            }
            commit(patch);
          }}
        />
      );
    }
    if (field.kind === 'subject') {
      // 类别没选时先选类别；大屏 / 定位卡这类动态目标仍按业务标识手填
      if (!filterSubjectByValue) {
        return <Select disabled className="msg-filter-control" placeholder="请先选择目标类别" />;
      }
      if (!filterSubjectSource) {
        return (
          <Input
            allowClear
            className="msg-filter-control"
            placeholder={field.label}
            value={value as string | undefined}
            onChange={(event) => setDraft((prev) => ({ ...prev, [field.key]: event.target.value }))}
            onPressEnter={() => applyFilters(draft, keyword)}
          />
        );
      }
      // 筛选区先选目标类别，再按类别从目录里选具体目标
      return (
        <ManagedPicker
          className="msg-filter-control"
          bundle={filterDirectory.bundle}
          kind={filterSubjectSource}
          loading={filterDirectory.loading}
          title={`筛选${field.label}`}
          placeholder={field.label}
          value={value === undefined || value === null ? '' : String(value)}
          onChange={(next) => commit({ [field.key]: next || undefined })}
        />
      );
    }
    if (field.kind === 'bool') {
      return (
        <div className="msg-filter-group">
          <span className="msg-filter-label">{field.label}</span>
          <OptionButtons
            allowClear
            ariaLabel={field.label}
            value={value === undefined ? undefined : String(value)}
            options={[
              { value: 'true', label: '是' },
              { value: 'false', label: '否' },
            ]}
            onChange={(next) => commit({ [field.key]: next })}
          />
        </div>
      );
    }
    return (
      <Input
        {...common}
        inputMode={field.kind === 'u64' ? 'numeric' : 'text'}
        value={value === undefined ? '' : String(value)}
        onChange={(event) => setDraft((prev) => ({ ...prev, [field.key]: event.target.value }))}
        onPressEnter={() => applyFilters(draft, keyword)}
      />
    );
  };

  /** 表单控件 */
  const renderFormControl = (field: FieldConfig) => {
    switch (field.kind) {
      case 'textarea':
        return (
          <Input.TextArea
            rows={field.rows ?? 4}
            maxLength={field.maxLength}
            showCount={field.maxLength !== undefined}
            autoSize={{ minRows: field.rows ?? 4, maxRows: 16 }}
            spellCheck={false}
            placeholder={`请输入${field.label}`}
          />
        );
      case 'template':
      case 'jsonTemplate':
        return (
          <TemplateEditor
            rows={field.rows ?? 4}
            maxLength={field.maxLength}
            showCount={field.maxLength !== undefined}
            isJsonTemplate={field.kind === 'jsonTemplate'}
            placeholder={
              field.kind === 'template'
                ? '示例：{{var}} 或 ${var}'
                : 'JSON 模板，如 {"key": {{var:json}}}'
            }
          />
        );
      case 'json':
        return <JsonEditor rows={field.rows ?? 8} placeholder={`请输入${field.label}`} />;
      case 'channelOptions':
        return <ChannelOptionsEditor />;
      case 'select':
        return <Select allowClear options={field.options} placeholder={`请选择${field.label}`} />;
      case 'ms':
        return (
          <DatePicker
            showTime
            style={{ width: '100%' }}
            format="YYYY-MM-DD HH:mm:ss"
            placeholder="选择时间"
          />
        );
      case 'bool':
        return <Switch checkedChildren="是" unCheckedChildren="否" />;
      case 'dic': {
        const options = dicOptions(field.dic ?? []);
        if (options.length <= 1) {
          return <span>{options[0]?.label ?? '无'}</span>;
        }
        if (options.length <= 5) {
          return <OptionButtons ariaLabel={field.label} options={options} onChange={() => undefined} />;
        }
        return <Select options={options} placeholder={`请选择${field.label}`} />;
      }
      case 'ref': {
        const ref = field.ref ? refOptions[field.ref] : undefined;
        return (
          <ManagedPicker
            bundle={refBundle((ref?.options ?? []) as EntitySelectOption[])}
            loading={ref?.loading}
            title={`选择${field.label}`}
            placeholder={`请选择${field.label}`}
            nameLabel={field.label}
            emptyText="还没有可选记录"
          />
        );
      }
      case 'subject': {
        // 类别声明了目标标识键：标识由事件标签按这些键取，配置里不填
        if (subjectField?.key === field.key && subjectTargetKeys) {
          return (
            <Typography.Text type="secondary">
              目标标识由发布方事件标签中的 {subjectTargetKeys} 决定，无需填写
            </Typography.Text>
          );
        }
        // 类别未选、或类别是 screen 这类动态目标时，仍按文本填写业务标识
        if (!subjectSource) {
          const needCategory = subjectField?.subjectBy !== undefined && !subjectByValue;
          return (
            <Input
              maxLength={field.maxLength}
              autoComplete="off"
              placeholder={needCategory ? '请先选择目标类别' : `请输入${field.label}`}
            />
          );
        }
        // 目录读不到时退回手填，不阻塞消息目标维护
        if (directory.error) {
          return (
            <Input
              maxLength={field.maxLength}
              autoComplete="off"
              placeholder={`${field.label}目录加载失败，请手动填写 ID`}
            />
          );
        }
        // 目录里选：左组织/分组树 + 右表搜索分页，与用户管理页同形态
        return (
          <ManagedPicker
            bundle={directory.bundle}
            kind={subjectSource}
            loading={directory.loading}
            title={`选择${field.label}`}
            placeholder={field.emptyLabel ?? `请选择${field.label}`}
            emptyText="目录里还没有可选目标"
          />
        );
      }
      default:
        return (
          <Input
            maxLength={field.maxLength ?? (field.kind === 'u64' ? 20 : undefined)}
            showCount={field.maxLength !== undefined}
            inputMode={field.kind === 'u64' ? 'numeric' : 'text'}
            autoComplete="off"
            placeholder={`请输入${field.label}`}
          />
        );
    }
  };

  /** 表单校验规则：必填 + u64 数字校验 + JSON/模板 + 长度上限 */
  const rulesFor = (field: FieldConfig): FormRule[] => {
    const rules: FormRule[] = [];
    if (field.required) {
      rules.push({
        required: true,
        whitespace: isTextLike(field),
        message: `请填写${field.label}`,
      });
    }
    if (field.kind === 'u64') {
      rules.push({
        validator: (_rule, value) => {
          if (value === undefined || value === null || value === '') {
            return Promise.resolve();
          }
          return /^\d+$/.test(String(value))
            ? Promise.resolve()
            : Promise.reject(new Error(`${field.label}必须是非负整数（字符串形态）`));
        },
      });
    }
    if (field.maxLength !== undefined && isTextLike(field)) {
      rules.push({
        max: field.maxLength,
        type: 'string',
        message: `${field.label}不能超过 ${field.maxLength} 个字符`,
      });
    }
    if (field.kind === 'json' || field.kind === 'channelOptions') {
      rules.push({
        validator: (_rule, value) => {
          const error = validateJsonText(value === undefined || value === null ? '' : String(value));
          return error ? Promise.reject(new Error(error)) : Promise.resolve();
        },
      });
    }
    if (field.kind === 'template') {
      rules.push({
        validator: (_rule, value) => {
          const error = validateTemplate(value === undefined || value === null ? '' : String(value));
          return error ? Promise.reject(new Error(error)) : Promise.resolve();
        },
      });
    }
    if (field.kind === 'jsonTemplate') {
      rules.push({
        validator: (_rule, value) => {
          const error = validateJsonTemplate(
            value === undefined || value === null ? '' : String(value),
          );
          return error ? Promise.reject(new Error(error)) : Promise.resolve();
        },
      });
    }
    return rules;
  };

  const emptyText = error ? (
    <span>加载失败，请重试</span>
  ) : (
    <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={`暂无${config.name}`}>
      {config.readOnly ? null : (
        <Button type="primary" onClick={openAdd}>
          新增{config.name}
        </Button>
      )}
    </Empty>
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card variant="borderless">
        {/*
          条件与「查询 / 重置 / 新增」同一行：条件区可伸缩占满左侧，按钮组靠右。
          「刷新」不在这里重复——它和「查询」打的是同一个接口、同一份条件。
        */}
        <div className="msg-toolbar">
          <div className="msg-toolbar-filters">
            <Input
              allowClear
              className="msg-filter-keyword"
              prefix={<SearchOutlined />}
              placeholder="关键字（模糊搜索）"
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              onPressEnter={() => applyFilters(draft, keyword)}
            />
            {searchFields.map((field) => (
              <span key={field.key} className="msg-filter-item">
                {renderFilter(field)}
              </span>
            ))}
          </div>
          <div className="msg-toolbar-foot">
            <Button type="primary" onClick={() => applyFilters(draft, keyword)}>
              查询
            </Button>
            <Button icon={<UndoOutlined />} onClick={resetFilters}>
              重置
            </Button>
            {config.readOnly ? null : (
              <Button type="primary" icon={<PlusOutlined />} onClick={openAdd}>
                新增{config.name}
              </Button>
            )}
          </div>
        </div>
      </Card>

      {error && (
        <Alert
          type="error"
          showIcon
          message="加载失败"
          description={error.detail || error.message}
          action={
            <Button size="small" onClick={reload}>
              重试
            </Button>
          }
        />
      )}

      <Card variant="borderless" title={sectionTitle}>
        <Table<MsgRow>
          rowKey="id"
          size="middle"
          loading={loading}
          columns={columns}
          dataSource={data.list}
          // 主从页的上表：单选 + 整行可点；操作列里的按钮不触发选中
          rowSelection={
            selectable
              ? {
                  type: 'radio',
                  selectedRowKeys: selectable.selectedId ? [selectable.selectedId] : [],
                  onChange: (_keys, rows) => {
                    if (rows[0]) {
                      selectable.onSelect(rows[0]);
                    }
                  },
                }
              : undefined
          }
          onRow={
            selectable
              ? (record) => ({
                  style: { cursor: 'pointer' },
                  onClick: (event: ReactMouseEvent<HTMLElement>) => {
                    if ((event.target as HTMLElement).closest('.msg-table-actions')) {
                      return;
                    }
                    selectable.onSelect(record);
                  },
                })
              : undefined
          }
          tableLayout="fixed"
          scroll={{ x: scrollX }}
          pagination={{
            current: pageNum,
            pageSize,
            total: Number(data.total),
            showSizeChanger: true,
            showQuickJumper: true,
            showTotal: (total) => `共 ${total} 条`,
            onChange: (page, size) => {
              if (size !== pageSize) {
                setPageSize(size);
                return;
              }
              setPageNum(page);
            },
          }}
          locale={{ emptyText }}
        />
      </Card>

      <Modal
        open={formOpen}
        title={editing ? `编辑${config.name}` : `新增${config.name}`}
        width={config.modalWidth ?? 720}
        className="msg-form-modal"
        okText="保存"
        cancelText="取消"
        confirmLoading={saving}
        maskClosable={!saving}
        closable={!saving}
        onOk={handleSubmit}
        onCancel={() => setFormOpen(false)}
        afterClose={() => {
          form.resetFields();
          setEditing(null);
        }}
      >
        <Form
          form={form}
          layout="vertical"
          className="msg-form"
          onValuesChange={(changed) => {
            // 目标类别变了，旧的目标标识不再适用，清空让用户重新选
            if (subjectField?.subjectBy && Object.prototype.hasOwnProperty.call(changed, subjectField.subjectBy)) {
              form.setFieldValue(subjectField.key, '');
            }
          }}
        >
          <AntRow gutter={[16, 0]}>
            {normalFields.map((field) => (
              <Col xs={24} sm={isWideField(field) ? 24 : 12} key={field.key}>
                <Form.Item
                  name={field.key}
                  label={field.label}
                  rules={rulesFor(field)}
                  valuePropName={field.kind === 'bool' ? 'checked' : undefined}
                  // JSON/模板在输入过程中不校验，避免刚敲一个 `{` 就整片标红；失焦和保存时仍会校验
                  validateTrigger={
                    field.kind === 'json' ||
                    field.kind === 'channelOptions' ||
                    field.kind === 'jsonTemplate' ||
                    field.kind === 'template'
                      ? ['onBlur']
                      : undefined
                  }
                  extra={field.tip}
                >
                  {renderFormControl(field)}
                </Form.Item>
              </Col>
            ))}
          </AntRow>
          {templateFields.length > 0 && (
            <Card
              size="small"
              title="模板"
              style={{ marginTop: 8 }}
              extra={
                <Typography.Text type="secondary" style={{ fontSize: 12, fontWeight: 'normal' }}>
                  三个模板共用一套编辑器；清空后保存会把该字段置为 NULL
                </Typography.Text>
              }
            >
              <Tabs
                size="small"
                activeKey={activeTemplateTab || templateFields[0]?.key}
                onChange={setActiveTemplateTab}
                items={templateFields.map((field) => ({
                  key: field.key,
                  label: field.label,
                  // 非激活页也挂载：否则那里的 Form.Item 不注册，保存时会漏掉它的校验规则
                  forceRender: true,
                  children: (
                    <TemplateTabPane field={field} rules={rulesFor(field)}>
                      {renderFormControl(field)}
                    </TemplateTabPane>
                  ),
                }))}
              />
            </Card>
          )}
          {editing && (
            <Typography.Text
              type="secondary"
              style={{ fontSize: 12, display: 'block', marginTop: 8 }}
            >
              留空的可选字段不会提交，服务端会把它们当作「不修改」；要删除已保存的模板，请用模板区域的「清空（置 NULL）」按钮。
            </Typography.Text>
          )}
        </Form>
      </Modal>

      <Drawer
        open={detail !== null}
        title={detail ? `${config.name}详情` : ''}
        width={760}
        rootClassName="msg-detail-drawer"
        onClose={() => setDetail(null)}
      >
        {detail && (
          <Descriptions column={1} size="small" bordered>
            <Descriptions.Item label="ID">
              <Typography.Text className="msg-mono msg-break" copyable>
                {detail.id}
              </Typography.Text>
            </Descriptions.Item>
            {config.fields
              .filter((field) => !fixedKeys.includes(field.key))
              .map((field) => (
                <Descriptions.Item key={field.key} label={field.label}>
                  {renderDetailValue(field, detail, refOptions)}
                </Descriptions.Item>
              ))}
            <Descriptions.Item label="创建时间">
              <span className="msg-mono">{formatMs(detail.createMs as string)}</span>
            </Descriptions.Item>
            <Descriptions.Item label="更新时间">
              <span className="msg-mono">{formatMs(detail.updateMs as string)}</span>
            </Descriptions.Item>
          </Descriptions>
        )}
      </Drawer>
    </Space>
  );
}
