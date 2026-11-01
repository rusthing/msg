/**
 * # 通用 CRUD 页面
 *
 * 14 张表的路由、分页参数、表单形状完全一致，差别只在字段，所以这里按 {@link EntityConfig}
 * 渲染一整套「筛选 + 列表 + 表单 + 详情」，每个实体的 `pages/*` 只剩一行配置。
 *
 * ## 三个和后端语义相关的注意点
 *
 * 1. **可选字段清空无效**：ModifyDto 的字段都是 `Option<T>`，表达不了「置为 NULL」，
 *    所以编辑时留空的字段等于「不修改」。表单里对此有明确提示，不要当成 bug。
 * 2. **`u64` 一律按字符串提交**：ID、`businessId`、时间戳在 JSON 里必须是字符串，
 *    输入框用文本 + 数字校验，不用 `InputNumber`（它会给出 number）。
 * 3. **筛选是等值匹配**：只有「关键字」走服务端模糊搜索，其余条件都是 `=`。
 */

import {
  App as AntdApp,
  Alert,
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  Modal,
  Row as AntRow,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Tooltip,
  Typography,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { PlusOutlined, ReloadOutlined, SearchOutlined, UndoOutlined } from '@ant-design/icons';
import dayjs, { type Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { crudOf } from '../api/crud';
import type { QueryParams } from '../api/client';
import type { Row as MsgRow } from '../api/types';
import BoolSwitch from '../components/BoolSwitch';
import JsonEditor from '../components/JsonEditor';
import JsonText from '../components/JsonText';
import StatusTag from '../components/StatusTag';
import type { EntityConfig, FieldConfig } from '../entities';
import { dicOptions } from '../dic';
import { usePagedQuery } from '../hooks/usePagedQuery';
import { useRefOptions, type RefOptions } from '../hooks/useRefOptions';
import { errorMessage, formatMs, orDash, prettyJson, truncate } from '../utils/format';

/** 表单值：日期控件给 Dayjs，其余按原样存 */
type FormValues = Record<string, unknown>;

/** 筛选草稿：值是输入框/下拉给的原始类型 */
type FilterDraft = Record<string, string | number | boolean | undefined>;

/** 宽控件占满一行：多行文本与 JSON 挤在半栏里没法编辑 */
function isWideField(field: FieldConfig): boolean {
  return field.kind === 'textarea' || field.kind === 'json';
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

/**
 * # 表单值 → 提交体
 *
 * - `ms`：Dayjs → 毫秒字符串（`u64`），日期控件清空时不提交；
 * - `bool`：始终提交（`false` 是有意义的值）；
 * - 其余空串/`undefined` 跳过，交给服务端的 `Option` 语义。
 */
function toSubmitBody(fields: FieldConfig[], values: FormValues): Record<string, unknown> {
  const body: Record<string, unknown> = {};
  for (const field of fields) {
    if (field.inForm === false) {
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
    if (value === undefined || value === null || value === '') {
      continue;
    }
    body[field.key] = value;
  }
  return body;
}

/** # 外键 ID → 名称（查不到就退回 ID 本身，别让单元格空白） */
function refLabel(ref: RefOptions | undefined, value: unknown) {
  const id = value === undefined || value === null ? '' : String(value);
  const row = ref?.byId.get(id);
  const name = row?.name;
  if (typeof name === 'string' && name) {
    return <Tooltip title={id}>{name}</Tooltip>;
  }
  const code = row?.code;
  if (typeof code === 'string' && code) {
    return <Tooltip title={id}>{code}</Tooltip>;
  }
  return <span className="msg-mono">{orDash(id)}</span>;
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
      return <span className="msg-mono">{formatMs(value as string)}</span>;
    case 'bool':
      return value ? <Tag color="success">是</Tag> : <Tag>否</Tag>;
    case 'dic':
      return <StatusTag dic={field.dic ?? []} value={value as number} />;
    case 'ref':
      return refLabel(field.ref ? refOptions[field.ref] : undefined, value);
    case 'json':
      return <JsonText value={value} />;
    case 'textarea':
      return value ? truncate(String(value), 40) : '-';
    case 'u64':
      return <span className="msg-mono">{orDash(value as string)}</span>;
    default:
      return orDash(value as string);
  }
}

/**
 * # 详情里的字段值
 *
 * 与表格的差别只在长文本：JSON 展开成格式化文本、多行文本原样铺开，
 * 其余字段复用 {@link renderCell}，避免两套渲染逻辑慢慢走偏。
 */
function renderDetailValue(
  field: FieldConfig,
  record: MsgRow,
  refOptions: Record<string, RefOptions>,
): ReactNode {
  const value = record[field.key];
  if (field.kind === 'json') {
    return value ? <pre className="msg-json-view">{prettyJson(String(value))}</pre> : '-';
  }
  if (field.kind === 'textarea') {
    return value ? <pre className="msg-text-view">{String(value)}</pre> : '-';
  }
  return renderCell(field, record, refOptions);
}

interface EntityPageProps {
  config: EntityConfig;
}

export default function EntityPage({ config }: EntityPageProps) {
  const { message, modal } = AntdApp.useApp();
  const api = useMemo(() => crudOf(config.path), [config.path]);

  const { data, loading, error, pageNum, pageSize, search, reload, setPageNum, setPageSize } =
    usePagedQuery<MsgRow>((params) => api.page(params));

  const [draft, setDraft] = useState<FilterDraft>({});
  const [keyword, setKeyword] = useState('');
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<MsgRow | null>(null);
  const [saving, setSaving] = useState(false);
  const [detail, setDetail] = useState<MsgRow | null>(null);
  const [form] = Form.useForm<FormValues>();

  /** 表单里出现的外键，一次性把选项拉齐 */
  const refKeys = useMemo(
    () => config.fields.filter((field) => field.kind === 'ref' && field.ref).map((field) => field.ref!),
    [config.fields],
  );
  const refOptions = useRefOptions(refKeys);

  const searchFields = useMemo(
    () => config.fields.filter((field) => field.searchable),
    [config.fields],
  );
  const formFields = useMemo(
    () => config.fields.filter((field) => field.inForm !== false),
    [config.fields],
  );

  /** 把草稿 + 关键字翻成 QueryDto 的查询参数；空值不传，避免变成等值条件 */
  const applyFilters = (next: FilterDraft, nextKeyword: string) => {
    const params: QueryParams = { _keyword: nextKeyword.trim() || undefined };
    for (const field of searchFields) {
      const value = next[field.key];
      if (value === undefined || value === '') {
        continue;
      }
      params[field.key] = value;
    }
    search(params);
  };

  const resetFilters = () => {
    setDraft({});
    setKeyword('');
    applyFilters({}, '');
  };

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
    applyFormValues(toFormValues(config.fields, null));
    setFormOpen(true);
  };

  const openEdit = (record: MsgRow) => {
    setEditing(record);
    form.resetFields();
    applyFormValues(toFormValues(config.fields, record));
    setFormOpen(true);
  };

  const handleSubmit = async () => {
    let values: FormValues;
    try {
      values = await form.validateFields();
    } catch {
      // 校验失败由 Form.Item 自己标红，不用弹窗
      return;
    }

    const body = toSubmitBody(config.fields, values);
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
          reload();
        } catch (reason) {
          message.error(errorMessage(reason));
        }
      },
    });
  };

  /** 表格里直接切开关：只改这一个字段 */
  const toggleField = async (record: MsgRow, field: FieldConfig, next: boolean) => {
    await api.modify({ id: record.id, [field.key]: next });
    reload();
  };

  const columns = useMemo<ColumnsType<MsgRow>>(() => {
    const result: ColumnsType<MsgRow> = [];

    if (config.showId) {
      result.push({
        title: 'ID',
        dataIndex: 'id',
        width: 190,
        render: (value: string) => <span className="msg-mono">{value}</span>,
      });
    }

    for (const field of config.fields) {
      if (field.inTable === false) {
        continue;
      }
      result.push({
        title: field.label,
        dataIndex: field.key,
        width: field.width,
        ellipsis: field.ellipsis,
        render: (_: unknown, record: MsgRow) => {
          if (field.kind === 'bool' && field.switchable) {
            return (
              <BoolSwitch
                value={Boolean(record[field.key])}
                onToggle={(next) => toggleField(record, field, next)}
              />
            );
          }
          return renderCell(field, record, refOptions);
        },
      });
    }

    for (const extra of config.extraColumns ?? []) {
      result.push(extra);
    }

    result.push({
      title: '更新时间',
      dataIndex: 'updateMs',
      width: 170,
      render: (value: string) => <span className="msg-mono">{formatMs(value)}</span>,
    });

    result.push({
      title: '操作',
      key: 'actions',
      width: 170,
      fixed: 'right',
      render: (_: unknown, record: MsgRow) => (
        <Space size={4}>
          <Button type="link" size="small" onClick={() => setDetail(record)}>
            详情
          </Button>
          <Button type="link" size="small" onClick={() => openEdit(record)}>
            编辑
          </Button>
          <Button type="link" size="small" danger onClick={() => handleDelete(record)}>
            删除
          </Button>
        </Space>
      ),
    });

    return result;
    // 依赖里刻意不放 `api`：它按 path 记忆化，path 变了 config 也会变
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [config, refOptions]);

  const scrollX = useMemo(
    () =>
      columns.reduce((sum, column) => sum + (typeof column.width === 'number' ? column.width : 160), 0),
    [columns],
  );

  /** 筛选控件：等值匹配，控件类型跟着字段走 */
  const renderFilter = (field: FieldConfig) => {
    const value = draft[field.key];
    const common = { allowClear: true, style: { width: 180 }, placeholder: field.label };

    if (field.kind === 'dic') {
      return (
        <Select
          {...common}
          value={value as number | undefined}
          options={dicOptions(field.dic ?? [])}
          onChange={(next) => setDraft((prev) => ({ ...prev, [field.key]: next }))}
        />
      );
    }
    if (field.kind === 'ref') {
      const ref = field.ref ? refOptions[field.ref] : undefined;
      return (
        <Select
          {...common}
          showSearch
          optionFilterProp="label"
          loading={ref?.loading}
          value={value as string | undefined}
          options={ref?.options ?? []}
          onChange={(next) => setDraft((prev) => ({ ...prev, [field.key]: next }))}
        />
      );
    }
    if (field.kind === 'bool') {
      return (
        <Select
          {...common}
          value={value === undefined ? undefined : String(value)}
          options={[
            { value: 'true', label: '是' },
            { value: 'false', label: '否' },
          ]}
          onChange={(next) => setDraft((prev) => ({ ...prev, [field.key]: next }))}
        />
      );
    }
    return (
      <Input
        {...common}
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
            placeholder={field.tip}
          />
        );
      case 'json':
        return <JsonEditor rows={field.rows ?? 8} />;
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
      case 'dic':
        return <Select options={dicOptions(field.dic ?? [])} placeholder="请选择" />;
      case 'ref': {
        const ref = field.ref ? refOptions[field.ref] : undefined;
        return (
          <Select
            showSearch
            optionFilterProp="label"
            loading={ref?.loading}
            options={ref?.options ?? []}
            placeholder={`请选择${field.label}`}
          />
        );
      }
      default:
        return (
          <Input
            maxLength={field.maxLength}
            showCount={field.maxLength !== undefined}
            placeholder={field.tip}
          />
        );
    }
  };

  /** 表单校验规则：必填 + u64 数字校验 + 长度上限 */
  const rulesFor = (field: FieldConfig) => {
    const rules: { required?: boolean; message?: string; pattern?: RegExp }[] = [];
    if (field.required) {
      rules.push({ required: true, message: `请填写${field.label}` });
    }
    if (field.kind === 'u64') {
      rules.push({ pattern: /^\d+$/, message: `${field.label}必须是非负整数（字符串形态）` });
    }
    return rules;
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card variant="borderless">
        <div className="msg-toolbar">
          <Input
            allowClear
            style={{ width: 240 }}
            prefix={<SearchOutlined />}
            placeholder="关键字（模糊搜索）"
            value={keyword}
            onChange={(event) => setKeyword(event.target.value)}
            onPressEnter={() => applyFilters(draft, keyword)}
          />
          {searchFields.map((field) => (
            <span key={field.key}>{renderFilter(field)}</span>
          ))}
          <Button type="primary" onClick={() => applyFilters(draft, keyword)}>
            查询
          </Button>
          <Button icon={<UndoOutlined />} onClick={resetFilters}>
            重置
          </Button>
          <span className="msg-toolbar-spacer" />
          <Button icon={<ReloadOutlined />} loading={loading} onClick={reload}>
            刷新
          </Button>
          <Button type="primary" icon={<PlusOutlined />} onClick={openAdd}>
            新增{config.name}
          </Button>
        </div>
      </Card>

      {error && (
        <Alert
          type="error"
          showIcon
          message="加载失败"
          description={error.detail || error.message}
        />
      )}

      <Card variant="borderless">
        <Table<MsgRow>
          rowKey="id"
          size="middle"
          loading={loading}
          columns={columns}
          dataSource={data.list}
          scroll={{ x: scrollX }}
          pagination={{
            current: pageNum,
            pageSize,
            total: Number(data.total),
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条`,
            onChange: (page, size) => {
              if (size !== pageSize) {
                setPageSize(size);
                return;
              }
              setPageNum(page);
            },
          }}
          locale={{ emptyText: `暂无${config.name}` }}
        />
      </Card>

      <Modal
        open={formOpen}
        title={editing ? `编辑${config.name}` : `新增${config.name}`}
        width={config.modalWidth ?? 720}
        okText="保存"
        cancelText="取消"
        confirmLoading={saving}
        onOk={handleSubmit}
        onCancel={() => setFormOpen(false)}
        afterClose={() => form.resetFields()}
      >
        <Form form={form} layout="vertical" className="msg-form">
          <AntRow gutter={16}>
            {formFields.map((field) => (
              <Col span={isWideField(field) ? 24 : 12} key={field.key}>
                <Form.Item
                  name={field.key}
                  label={field.label}
                  rules={rulesFor(field)}
                  valuePropName={field.kind === 'bool' ? 'checked' : undefined}
                  extra={field.tip && !isWideField(field) ? field.tip : undefined}
                >
                  {renderFormControl(field)}
                </Form.Item>
              </Col>
            ))}
          </AntRow>
          {editing && (
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              留空的可选字段不会提交，服务端会把它们当作「不修改」，而不是清空。
            </Typography.Text>
          )}
        </Form>
      </Modal>

      <Drawer
        open={detail !== null}
        title={detail ? `${config.name}详情` : ''}
        width={760}
        onClose={() => setDetail(null)}
      >
        {detail && (
          <Descriptions column={1} size="small" bordered>
            <Descriptions.Item label="ID">
              <span className="msg-mono">{detail.id}</span>
            </Descriptions.Item>
            {config.fields.map((field) => (
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
