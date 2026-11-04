/**
 * # 总览
 *
 * 一屏回答三个问题：**定义侧配全了没有**、**投递是否在失败**、**死信有没有积压**。
 *
 * ## 为什么不做一个后端统计接口
 *
 * 每张表都在「几十到几千行」这个量级，`/page?_size=1` 只取 `total`，比让服务端再开一套
 * 统计接口省事，也不会出现「统计口径在两处各写一遍」的问题。真正需要趋势图时再单独做。
 *
 * ## 失败与死信为什么分开看
 *
 * 失败是**投递结论**（`deliverStatus = 3` 表示这条消息的所有目标都发失败了），死信是
 * **链路层面的放弃**（重投到上限才进死信）。一条消息可以「部分目标成功」而不算失败，
 * 也可能因为 NATS 层就投不出去而直接进死信、压根没有投递记录，两者不互相包含。
 */

import { Alert, Button, Card, Col, Row, Space, Table, Tag, Typography } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { ReloadOutlined } from '@ant-design/icons';
import { Link } from 'react-router-dom';
import { crudOf } from '../api/crud';
import type { QueryParams } from '../api/client';
import type { MsgDeadLetterVo, MsgDeliveryVo } from '../api/types';
import StatusTag from '../components/StatusTag';
import { deliverStatusDic } from '../dic';
import { entityOf } from '../entities';
import { useAsync } from '../hooks/useAsync';
import { formatMs, orDash } from '../utils/format';

/** 取某张表的记录数：只请求 1 条，用 `total` */
async function countOf(path: string, params: QueryParams = {}): Promise<number> {
  const page = await crudOf(path).page({ ...params, _page: 1, _size: 1 });
  return Number(page.total);
}

/** 总览数据 */
interface Overview {
  eventSources: number;
  categories: number;
  channels: number;
  messages: number;
  deliveries: number;
  failedDeliveries: number;
  pendingDeadLetters: number;
  messageQueues: number;
  deliveryQueues: number;
  failedList: MsgDeliveryVo[];
  deadList: MsgDeadLetterVo[];
}

/** 初始值，请求返回前先渲染 0，避免卡片跳动 */
const INITIAL: Overview = {
  eventSources: 0,
  categories: 0,
  channels: 0,
  messages: 0,
  deliveries: 0,
  failedDeliveries: 0,
  pendingDeadLetters: 0,
  messageQueues: 0,
  deliveryQueues: 0,
  failedList: [],
  deadList: [],
};

/** 统计卡 */
function StatCard({
  title,
  value,
  hint,
  danger,
  link,
  loading,
}: {
  title: string;
  /** 加载失败时传 `--`，不要把「没取到」显示成 0 */
  value: number | string;
  hint: string;
  danger?: boolean;
  link: string;
  loading: boolean;
}) {
  return (
    <Card variant="borderless" className="msg-stat-card" loading={loading}>
      <div className="msg-stat-label">{title}</div>
      <div
        className="msg-stat-value"
        style={danger && typeof value === 'number' && value > 0 ? { color: '#cf1322' } : undefined}
      >
        {typeof value === 'number' ? value.toLocaleString('zh-CN') : value}
      </div>
      <div className="msg-stat-hint" style={{ marginTop: 6 }}>
        <Link to={link}>{hint}</Link>
      </div>
    </Card>
  );
}

export default function Dashboard() {
  const state = useAsync<Overview>(async () => {
    const delivery = entityOf('delivery')!.path;
    const deadLetter = entityOf('dead-letter')!.path;

    const [
      eventSources,
      categories,
      channels,
      messages,
      deliveries,
      failedDeliveries,
      pendingDeadLetters,
      messageQueues,
      deliveryQueues,
      failedPage,
      deadPage,
    ] = await Promise.all([
      countOf(entityOf('event-source')!.path),
      countOf(entityOf('message-category')!.path),
      countOf(entityOf('channel')!.path),
      countOf(entityOf('message')!.path),
      countOf(delivery),
      countOf(delivery, { deliverStatus: 3 }),
      countOf(deadLetter, { handled: false }),
      countOf(entityOf('message-queue')!.path),
      countOf(entityOf('delivery-queue')!.path),
      // 最近的失败与未处理死信：按主键倒序，等价于「最新」
      crudOf<MsgDeliveryVo>(delivery).page({
        deliverStatus: 3,
        _page: 1,
        _size: 5,
        _orderBy: 'id:desc',
      }),
      crudOf<MsgDeadLetterVo>(deadLetter).page({
        handled: false,
        _page: 1,
        _size: 5,
        _orderBy: 'id:desc',
      }),
    ]);

    return {
      eventSources,
      categories,
      channels,
      messages,
      deliveries,
      failedDeliveries,
      pendingDeadLetters,
      messageQueues,
      deliveryQueues,
      failedList: failedPage.list,
      deadList: deadPage.list,
    };
  }, [], INITIAL);

  const { data, loading, error, reload } = state;

  /**
   * 总览里的两张表只有半屏宽（约 500px），列宽要留出余量：
   * 固定列之和一旦超过容器宽度，没写宽度的列会被压到十几像素，
   * 表头变成一条被裁掉的竖条。所以只留最能说明问题的几列，其余去详情页看。
   */
  const failedColumns: ColumnsType<MsgDeliveryVo> = [
    {
      title: '标题',
      dataIndex: 'title',
      width: 140,
      ellipsis: true,
      render: (value: string) => orDash(value),
    },
    { title: '事件编码', dataIndex: 'eventCode', width: 120, ellipsis: true },
    {
      title: '状态',
      dataIndex: 'deliverStatus',
      width: 90,
      align: 'center',
      render: (value: number) => <StatusTag dic={deliverStatusDic} value={value} />,
    },
    {
      title: '业务触发时间',
      dataIndex: 'businessTriggerMs',
      width: 176,
      render: (value: string) => <span className="msg-mono msg-nowrap">{formatMs(value)}</span>,
    },
  ];

  const deadColumns: ColumnsType<MsgDeadLetterVo> = [
    { title: '原消息主题', dataIndex: 'subject', width: 140, ellipsis: true },
    {
      title: '转入原因',
      dataIndex: 'reason',
      ellipsis: true,
      render: (value: string) => <span title={value}>{orDash(value)}</span>,
    },
    {
      title: '进入时间',
      dataIndex: 'createMs',
      width: 176,
      render: (value: string) => <span className="msg-mono msg-nowrap">{formatMs(value)}</span>,
    },
  ];

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div className="msg-page-bar">
        <Typography.Text type="secondary">
          统计口径：直接读各表行数；失败取 `deliverStatus = 3`（全部目标都失败），死信取未勾选「已处理」
        </Typography.Text>
        <Button
          className="msg-page-bar-action"
          icon={<ReloadOutlined />}
          loading={loading}
          onClick={reload}
        >
          刷新
        </Button>
      </div>

      {error && (
        <Alert
          type="error"
          showIcon
          message="加载总览失败"
          description={error.detail || error.message}
          action={
            <Button size="small" onClick={reload}>
              重试
            </Button>
          }
        />
      )}

      <div className="msg-stat-grid">
        <StatCard
          title="消息定义"
          value={error ? '--' : data.messages}
          hint="去配置"
          link="/message"
          loading={loading}
        />
        <StatCard
          title="投递渠道"
          value={error ? '--' : data.channels}
          hint="去配置"
          link="/channel"
          loading={loading}
        />
        <StatCard
          title="事件来源"
          value={error ? '--' : data.eventSources}
          hint="去配置"
          link="/event-source"
          loading={loading}
        />
        <StatCard
          title="消息类别"
          value={error ? '--' : data.categories}
          hint="去配置"
          link="/message-category"
          loading={loading}
        />
        <StatCard
          title="投递总数"
          value={error ? '--' : data.deliveries}
          hint="查看记录"
          link="/delivery"
          loading={loading}
        />
        <StatCard
          title="全部失败的投递"
          value={error ? '--' : data.failedDeliveries}
          hint="去排查"
          link="/delivery"
          danger
          loading={loading}
        />
        <StatCard
          title="未处理死信"
          value={error ? '--' : data.pendingDeadLetters}
          hint="去处理"
          link="/dead-letter"
          danger
          loading={loading}
        />
      </div>

      <Row gutter={[16, 16]}>
        <Col xs={24} xl={12}>
          <Card
            variant="borderless"
            title="最近失败的投递"
            extra={<Link to="/delivery">全部</Link>}
          >
            <Table<MsgDeliveryVo>
              rowKey="id"
              size="small"
              tableLayout="fixed"
              loading={loading}
              columns={failedColumns}
              dataSource={data.failedList}
              pagination={false}
              scroll={{ x: 526 }}
              locale={{ emptyText: error ? '加载失败，请重试' : '没有投递失败的记录' }}
            />
          </Card>
        </Col>
        <Col xs={24} xl={12}>
          <Card
            variant="borderless"
            title="待处理死信"
            extra={<Link to="/dead-letter">全部</Link>}
          >
            <Table<MsgDeadLetterVo>
              rowKey="id"
              size="small"
              tableLayout="fixed"
              loading={loading}
              columns={deadColumns}
              dataSource={data.deadList}
              pagination={false}
              scroll={{ x: 480 }}
              locale={{ emptyText: error ? '加载失败，请重试' : '没有未处理的死信' }}
            />
          </Card>
        </Col>
      </Row>

      <Card variant="borderless" title="投递链路">
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <div className="msg-chain">
            <span className="msg-chain-node">
              <strong>消息定义</strong> 事件编码 + 模板
            </span>
            <span className="msg-chain-arrow">→</span>
            <span className="msg-chain-node">
              <strong>投递</strong> 一次业务触发（幂等键）
            </span>
            <span className="msg-chain-arrow">→</span>
            <span className="msg-chain-node">
              <strong>投递目标</strong> 按目标类别展开
            </span>
            <span className="msg-chain-arrow">→</span>
            <span className="msg-chain-node">
              <strong>投递渠道任务</strong> 目标 × 渠道
            </span>
            <span className="msg-chain-arrow">→</span>
            <span className="msg-chain-node">
              <strong>渠道投递日志</strong> 每次发送尝试
            </span>
          </div>

          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            排查顺序从前到后：先确认消息定义挂对了事件来源与目标类别，再看投递是否落库
            （没落库说明消息没进队列或幂等命中了），然后看目标与渠道任务是否展开，
            最后在渠道投递日志里看具体失败原因。
          </Typography.Text>

          <Space size={24} wrap>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              消息队列 <Tag>{data.messageQueues}</Tag>
              <Link to="/message-queue">管理</Link>
            </Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              投递队列 <Tag>{data.deliveryQueues}</Tag>
              <Link to="/delivery-queue">管理</Link>
            </Typography.Text>
          </Space>
        </Space>
      </Card>
    </Space>
  );
}
