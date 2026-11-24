/**
 * # 消息定义与目标：合并维护
 *
 * 「消息」和「消息目标」是同一份配置的两层：一条消息发给哪些目标，必须和模板、
 * 事件编码放在一个视野里才看得懂。所以这里把两张表放到同一页：
 *
 * - 上半部分是消息列表（单选），沿用 `message` 实体的筛选、分页与增删改；
 * - 下半部分是**当前选中消息**的目标列表，新增时自动带入 `messageId`，
 *   切换上半部分的选择即重新加载，不用先去筛选区选一遍消息。
 *
 * 两张表本身仍是独立的 CRUD 接口，只是共用一个页面；`message-target` 的独立
 * 路由保留，方便从投递链路等页面深链过来。
 *
 * 子表用 `key={selected.id}` 强制重建：切换消息时清空子表的筛选、页码和弹窗状态，
 * 避免上一条消息的筛选条件被带到下一条。
 */

import { useMemo, useState } from 'react';
import { Card, Empty, Space, Typography } from 'antd';
import type { Row } from '../api/types';
import { entityOf } from '../entities';
import { shortId } from '../utils/format';
import EntityPage from './EntityPage';

const messageConfig = entityOf('message')!;
const targetConfig = entityOf('message-target')!;

export default function MessageWorkbench() {
  const [selected, setSelected] = useState<Row | null>(null);

  /** 子表的固定条件：只在选中消息时创建，切换消息由 key 负责重建子表 */
  const fixedFilters = useMemo(
    () => (selected ? { messageId: selected.id } : undefined),
    [selected],
  );

  const selectedLabel = selected
    ? String(selected.name || selected.eventCode || shortId(selected.id))
    : '';

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <EntityPage
        config={messageConfig}
        sectionTitle={
          <Space size={8} wrap>
            <span>消息定义</span>
            <Typography.Text type="secondary" style={{ fontSize: 12, fontWeight: 'normal' }}>
              选中一条消息，下方显示并维护它发给哪些目标
            </Typography.Text>
          </Space>
        }
        selectable={{
          selectedId: selected?.id,
          onSelect: (row) => setSelected((prev) => (prev?.id === row.id ? prev : row)),
        }}
        onDeleted={(row) => {
          if (selected?.id === row.id) {
            setSelected(null);
          }
        }}
      />

      {selected && fixedFilters ? (
        <EntityPage
          key={selected.id}
          config={targetConfig}
          fixedFilters={fixedFilters}
          sectionTitle={
            <Space size={8} wrap>
              <span>消息目标</span>
              <Typography.Text type="secondary" style={{ fontSize: 12, fontWeight: 'normal' }}>
                {selectedLabel}
              </Typography.Text>
            </Space>
          }
        />
      ) : (
        <Card variant="borderless">
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description="请先在上方选择一条消息，再维护它的目标"
          />
        </Card>
      )}
    </Space>
  );
}
