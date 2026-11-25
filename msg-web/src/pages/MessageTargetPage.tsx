/**
 * # 消息目标（含渠道绑定）
 *
 * 「这个目标发给谁」和「走哪几条渠道」是同一份配置，所以渠道多选直接放在
 * 新增/编辑弹窗里，且至少选一条才能保存。后端在同一个事务里写目标与绑定行，
 * 列表上的「渠道」列把该目标的所有绑定合并成一组标签展示。
 *
 * 新增时按目标类别的「默认渠道集」预填（is_default 优先），用户可调整；
 * 保存后默认渠道不再参与投递，只保留为下次新建的模板。
 */

import {
  App as AntdApp,
  Button,
  Card,
  Descriptions,
  Empty,
  Form,
  Input,
  Select,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
} from 'antd';
import { Modal, Drawer } from '../lib/federated-overlay';
import type { ColumnsType } from 'antd/es/table';
import { PlusOutlined, ReloadOutlined, SearchOutlined, UndoOutlined } from '@ant-design/icons';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { crudOf } from '../api/crud';
import type { DirectoryBundle, DirectoryKind } from '../api/directory';
import type { Row } from '../api/types';
import { ManagedPicker } from '../components/ManagedPicker';
import { useDirectoryOptions } from '../hooks/useDirectoryOptions';
import { usePagedQuery } from '../hooks/usePagedQuery';
import { errorMessage, formatMs, shortId } from '../utils/format';

const targetApi = crudOf('/msg/message-target');
const bindingApi = crudOf('/msg/message-target-channel');
const messageApi = crudOf('/msg/message');
const categoryApi = crudOf('/msg/target-category');
const channelApi = crudOf('/msg/channel');
const categoryChannelApi = crudOf('/msg/target-category-channel');

interface TargetFormValues {
  messageId?: string;
  targetCategoryId?: string;
  targetId?: string;
  channelIds?: string[];
}

const SUBJECT_SOURCES: Record<string, DirectoryKind> = {
  user: 'user',
  role: 'role',
  org: 'org',
};

/** 外键选项折成选择面板用的目录：无树、无分组，只有可搜索的条目 */
function toBundle(
  rows: Row[],
  labelOf: (row: Row) => string,
): DirectoryBundle {
  return {
    entries: rows.map((row) => {
      const label = labelOf(row);
      return { id: String(row.id), name: label, label, detail: `#${String(row.id)}` };
    }),
    tree: [],
    groupsByEntry: new Map(),
  };
}

function labelOfChannel(row: Row): string {
  return String(row.name || row.channelType || shortId(row.id));
}

/** # 消息目标页 */
export default function MessageTargetPage() {
  const { message, modal } = AntdApp.useApp();
  const [form] = Form.useForm<TargetFormValues>();

  const [messages, setMessages] = useState<Row[]>([]);
  const [categories, setCategories] = useState<Row[]>([]);
  const [channels, setChannels] = useState<Row[]>([]);
  const [categoryChannels, setCategoryChannels] = useState<Row[]>([]);
  const [bindings, setBindings] = useState<Row[]>([]);
  const [optionsError, setOptionsError] = useState('');

  const [draft, setDraft] = useState<{ messageId?: string; targetCategoryId?: string; targetId?: string }>({});
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<Row | null>(null);
  const [saving, setSaving] = useState(false);
  const [detail, setDetail] = useState<Row | null>(null);

  const fetchPage = useCallback((params: Record<string, string | number | boolean | null | undefined>) =>
    targetApi.page(params), []);
  const { data, loading, error, pageNum, pageSize, search, reload, setPageNum, setPageSize } =
    usePagedQuery<Row>(fetchPage);

  /** 选项与绑定一次性拉全；目标/渠道都是配置表，量不大 */
  const loadOptions = useCallback(async () => {
    const results = await Promise.allSettled([
      messageApi.list(),
      categoryApi.list(),
      channelApi.list(),
      categoryChannelApi.list(),
      bindingApi.list(),
    ]);
    const values = results.map((result) => (result.status === 'fulfilled' ? result.value : []));
    setMessages(values[0]);
    setCategories(values[1]);
    setChannels(values[2]);
    setCategoryChannels(values[3]);
    setBindings(values[4]);
    const failed = results.some((result) => result.status === 'rejected');
    setOptionsError(failed ? '部分下拉选项加载失败，列表仍可正常使用' : '');
  }, []);
  useEffect(() => {
    void loadOptions();
  }, [loadOptions]);

  const channelsByTarget = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const row of bindings) {
      const key = String(row.messageTargetId);
      const list = map.get(key) ?? [];
      list.push(String(row.channelId));
      map.set(key, list);
    }
    return map;
  }, [bindings]);

  const channelLabels = useMemo(() => {
    const map = new Map<string, string>();
    for (const row of channels) {
      map.set(String(row.id), labelOfChannel(row));
    }
    return map;
  }, [channels]);

  const messageLabels = useMemo(() => {
    const map = new Map<string, string>();
    for (const row of messages) {
      map.set(String(row.id), String(row.name || row.eventCode || shortId(row.id)));
    }
    return map;
  }, [messages]);

  const categoryRows = useMemo(() => {
    const map = new Map<string, Row>();
    for (const row of categories) {
      map.set(String(row.id), row);
    }
    return map;
  }, [categories]);

  /** 类别默认渠道：is_default 优先，一条都没标时取该类别全部绑定 */
  const defaultChannelsOf = useCallback(
    (categoryId: string) => {
      const rows = categoryChannels.filter((row) => String(row.targetCategoryId) === categoryId);
      const defaults = rows
        .filter((row) => Boolean(row.isDefault))
        .map((row) => String(row.channelId));
      const chosen = defaults.length ? defaults : rows.map((row) => String(row.channelId));
      return [...new Set(chosen)];
    },
    [categoryChannels],
  );

  const watchedCategoryId = Form.useWatch('targetCategoryId', form);
  const categoryRow = categoryRows.get(String(watchedCategoryId ?? ''));
  const categoryCode = String(categoryRow?.code ?? '');
  const subjectSource = SUBJECT_SOURCES[categoryCode];
  const subjectTargetKeys = String(categoryRow?.targetKeys ?? '').trim();
  const directory = useDirectoryOptions(subjectSource, formOpen);
  const lastCategory = useRef<string | undefined>(undefined);

  /** 新增时选类别就按默认渠道预填；只在类别真正变化时覆盖，用户改过的渠道不会被冲掉 */
  useEffect(() => {
    if (!formOpen || editing) {
      return;
    }
    const next = watchedCategoryId ? String(watchedCategoryId) : undefined;
    if (!next || lastCategory.current === next) {
      return;
    }
    lastCategory.current = next;
    form.setFieldValue('channelIds', defaultChannelsOf(next));
  }, [formOpen, editing, watchedCategoryId, defaultChannelsOf, form]);

  /** 类别声明了目标标识键时，标识由事件标签决定，不在这里保存 */
  useEffect(() => {
    if (formOpen && subjectTargetKeys) {
      form.setFieldValue('targetId', '');
    }
  }, [formOpen, subjectTargetKeys, form]);

  const channelBundle = useMemo(() => toBundle(channels, labelOfChannel), [channels]);
  const messageBundle = useMemo(
    () => toBundle(messages, (row) => String(row.name || row.eventCode || shortId(row.id))),
    [messages],
  );
  const messageOptions = messageBundle.entries.map((entry) => ({ value: entry.id, label: entry.label }));
  const categoryOptions = categories.map((row) => ({
    value: String(row.id),
    label: String(row.name || row.code || shortId(row.id)),
  }));
  const channelOptions = channelBundle.entries.map((entry) => ({ value: entry.id, label: entry.label }));

  const applyFilters = (next: typeof draft) => {
    search({
      messageId: next.messageId || undefined,
      targetCategoryId: next.targetCategoryId || undefined,
      targetId: next.targetId?.trim() || undefined,
    });
  };

  const resetFilters = () => {
    setDraft({});
    search({});
  };

  const openAdd = () => {
    setEditing(null);
    form.resetFields();
    lastCategory.current = undefined;
    form.setFieldsValue({ targetId: '', channelIds: [] });
    setFormOpen(true);
  };

  const openEdit = (row: Row) => {
    setEditing(row);
    form.resetFields();
    form.setFieldsValue({
      messageId: String(row.messageId),
      targetCategoryId: String(row.targetCategoryId),
      targetId: String(row.targetId ?? ''),
      channelIds: channelsByTarget.get(String(row.id)) ?? [],
    });
    setFormOpen(true);
  };

  const submit = async () => {
    const values = await form.validateFields();
    const channelIds = (values.channelIds ?? []).map(String);
    if (!channelIds.length) {
      message.error('请至少绑定一个渠道');
      return;
    }
    const body: Record<string, unknown> = {
      messageId: String(values.messageId),
      targetCategoryId: String(values.targetCategoryId),
      targetId: values.targetId ?? '',
      channelIds,
    };
    if (editing) {
      body.id = editing.id;
    }
    setSaving(true);
    try {
      if (editing) {
        await targetApi.modify(body);
        message.success('修改成功');
      } else {
        await targetApi.add(body);
        message.success('新增成功');
      }
      setFormOpen(false);
      await loadOptions();
      reload();
    } catch (reason) {
      message.error(errorMessage(reason));
    } finally {
      setSaving(false);
    }
  };

  const remove = (row: Row) => {
    modal.confirm({
      title: '删除消息目标？',
      content: '该目标的渠道绑定会一起删除，消息本身不受影响。',
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        try {
          await targetApi.remove(row.id);
          message.success('删除成功');
          await loadOptions();
          reload();
        } catch (reason) {
          message.error(errorMessage(reason));
        }
      },
    });
  };

  const columns: ColumnsType<Row> = [
    {
      title: '消息',
      dataIndex: 'messageId',
      width: 190,
      render: (value: string) => (
        <Tooltip title={String(value)}>
          <span>{messageLabels.get(String(value)) ?? shortId(value)}</span>
        </Tooltip>
      ),
    },
    {
      title: '目标类别',
      dataIndex: 'targetCategoryId',
      width: 150,
      render: (value: string) => categoryRows.get(String(value))?.name ?? shortId(value),
    },
    {
      title: '目标标识',
      dataIndex: 'targetId',
      width: 180,
      render: (value: string) =>
        value ? <Typography.Text className="msg-mono">{value}</Typography.Text> : <Tag>全部目标</Tag>,
    },
    {
      title: '渠道',
      key: 'channels',
      width: 240,
      render: (_value, row) => {
        const ids = channelsByTarget.get(String(row.id)) ?? [];
        if (!ids.length) {
          return <Typography.Text type="secondary">未绑定（无法投递）</Typography.Text>;
        }
        return (
          <Space size={[4, 4]} wrap>
            {ids.map((id) => (
              <Tag key={id} color="blue">
                {channelLabels.get(id) ?? shortId(id)}
              </Tag>
            ))}
          </Space>
        );
      },
    },
    { title: '更新时间', dataIndex: 'updateMs', width: 180, render: (value: string) => <span className="msg-mono msg-nowrap">{formatMs(value)}</span> },
    {
      title: '操作',
      key: 'actions',
      width: 170,
      fixed: 'right',
      render: (_value, row) => (
        <Space size={4} className="msg-table-actions">
          <Button type="link" size="small" onClick={() => setDetail(row)}>
            详情
          </Button>
          <Button type="link" size="small" onClick={() => openEdit(row)}>
            编辑
          </Button>
          <Button type="link" size="small" danger onClick={() => remove(row)}>
            删除
          </Button>
        </Space>
      ),
    },
  ];

  const subjectField = (
    <Form.Item
      name="targetId"
      label="目标标识"
      extra={subjectTargetKeys ? `目标标识由事件标签中的 ${subjectTargetKeys} 决定，无需填写` : undefined}
    >
      {subjectTargetKeys ? (
        <Input disabled placeholder="由事件标签决定" />
      ) : !subjectSource ? (
        <Input allowClear maxLength={100} placeholder="留空表示该类别下的全部目标" />
      ) : directory.error ? (
        <Input allowClear maxLength={100} placeholder="目录加载失败，请手动填写 ID" />
      ) : (
        <ManagedPicker
          bundle={directory.bundle}
          kind={subjectSource}
          loading={directory.loading}
          title="选择目标"
          placeholder="请选择目标"
          nameLabel="目标"
          emptyText="目录里还没有可选目标"
        />
      )}
    </Form.Item>
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card variant="borderless">
        <Space wrap align="center">
          <Select
            allowClear
            showSearch
            className="msg-filter-control"
            style={{ width: 220 }}
            placeholder="消息"
            options={messageOptions}
            value={draft.messageId}
            onChange={(value) => setDraft((prev) => ({ ...prev, messageId: value }))}
          />
          <Select
            allowClear
            showSearch
            className="msg-filter-control"
            style={{ width: 180 }}
            placeholder="目标类别"
            options={categoryOptions}
            value={draft.targetCategoryId}
            onChange={(value) => setDraft((prev) => ({ ...prev, targetCategoryId: value }))}
          />
          <Input
            allowClear
            className="msg-filter-control"
            style={{ width: 180 }}
            placeholder="目标标识"
            value={draft.targetId}
            onChange={(event) => setDraft((prev) => ({ ...prev, targetId: event.target.value }))}
            onPressEnter={() => applyFilters(draft)}
          />
          <Button type="primary" icon={<SearchOutlined />} onClick={() => applyFilters(draft)}>
            查询
          </Button>
          <Button icon={<UndoOutlined />} onClick={resetFilters}>
            重置
          </Button>
          <Button icon={<ReloadOutlined />} onClick={reload}>
            刷新
          </Button>
        </Space>
        {optionsError && (
          <Typography.Text type="warning" style={{ display: 'block', marginTop: 8 }}>
            {optionsError}
          </Typography.Text>
        )}
      </Card>

      <Card
        variant="borderless"
        title={
          <Space>
            <span>消息目标</span>
            <Typography.Text type="secondary" style={{ fontSize: 12, fontWeight: 'normal' }}>
              渠道是目标的必填项，新增时按类别默认渠道预填
            </Typography.Text>
          </Space>
        }
        extra={
          <Button type="primary" icon={<PlusOutlined />} onClick={openAdd}>
            新增消息目标
          </Button>
        }
      >
        {error && <Empty description={error.message} />}
        <Table<Row>
          rowKey="id"
          size="middle"
          loading={loading}
          columns={columns}
          dataSource={data.list}
          tableLayout="fixed"
          scroll={{ x: 1030 }}
          pagination={{
            current: pageNum,
            pageSize,
            total: Number(data.total) || 0,
            showSizeChanger: true,
            showTotal: (total) => `共 ${total} 条`,
            onChange: (nextPage, nextSize) => {
              if (nextSize !== pageSize) {
                setPageSize(nextSize);
              } else {
                setPageNum(nextPage);
              }
            },
          }}
        />
      </Card>

      <Modal
        open={formOpen}
        title={editing ? '编辑消息目标' : '新增消息目标'}
        onCancel={() => setFormOpen(false)}
        onOk={() => void submit()}
        okText="保存"
        cancelText="取消"
        confirmLoading={saving}
        width={680}
        destroyOnHidden
      >
        <Form form={form} layout="vertical" initialValues={{ targetId: '', channelIds: [] }}>
          <Form.Item name="messageId" label="消息" rules={[{ required: true, message: '请选择消息' }]}>
            <ManagedPicker
              bundle={messageBundle}
              title="选择消息"
              placeholder="请选择消息"
              nameLabel="消息"
              emptyText="还没有消息定义"
            />
          </Form.Item>
          <Form.Item
            name="targetCategoryId"
            label="目标类别"
            rules={[{ required: true, message: '请选择目标类别' }]}
          >
            <Select
              showSearch
              placeholder="请选择目标类别"
              options={categoryOptions}
              onChange={() => form.setFieldValue('targetId', '')}
            />
          </Form.Item>
          {subjectField}
          <Form.Item
            name="channelIds"
            label="绑定渠道"
            rules={[{ required: true, type: 'array', min: 1, message: '请至少绑定一个渠道' }]}
            extra="可多选；保存后该目标只从这些渠道投递"
          >
            <Select
              mode="multiple"
              showSearch
              allowClear
              placeholder="请选择渠道"
              options={channelOptions}
              optionFilterProp="label"
            />
          </Form.Item>
        </Form>
      </Modal>

      <Drawer
        open={Boolean(detail)}
        title="消息目标详情"
        width={520}
        onClose={() => setDetail(null)}
      >
        {detail && (
          <Descriptions column={1} bordered size="small">
            <Descriptions.Item label="ID">
              <span className="msg-mono">{String(detail.id)}</span>
            </Descriptions.Item>
            <Descriptions.Item label="消息">
              {messageLabels.get(String(detail.messageId)) ?? shortId(String(detail.messageId))}
            </Descriptions.Item>
            <Descriptions.Item label="目标类别">
              {String(categoryRows.get(String(detail.targetCategoryId))?.name ?? '') ||
                shortId(String(detail.targetCategoryId))}
            </Descriptions.Item>
            <Descriptions.Item label="目标标识">
              {String(detail.targetId ?? '') || '全部目标'}
            </Descriptions.Item>
            <Descriptions.Item label="渠道">
              <Space size={[4, 4]} wrap>
                {(channelsByTarget.get(String(detail.id)) ?? []).map((id) => (
                  <Tag key={id} color="blue">
                    {channelLabels.get(id) ?? shortId(id)}
                  </Tag>
                ))}
              </Space>
            </Descriptions.Item>
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
