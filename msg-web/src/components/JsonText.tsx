/**
 * # 表格里的 JSON 单元格
 *
 * 表格列宽有限，JSON 直接铺开会把整行撑变形。这里展示单行截断文本，悬停弹出格式化后的
 * 全文（`labels` / `annotations` / 渠道配置都是排查时的关键信息，不能只给个省略号）。
 */

import { Popover, Typography } from 'antd';
import { prettyJson, truncate } from '../utils/format';

interface JsonTextProps {
  value?: unknown;
}

export default function JsonText({ value }: JsonTextProps) {
  if (value === null || value === undefined || value === '') {
    return <>-</>;
  }
  const text = String(value);
  return (
    <Popover
      trigger="hover"
      mouseEnterDelay={0.25}
      styles={{ body: { maxWidth: 520, maxHeight: 360, overflow: 'auto', padding: 0 } }}
      content={<pre className="msg-json-view">{prettyJson(text)}</pre>}
    >
      <Typography.Text className="msg-mono" style={{ fontSize: 12 }}>
        {truncate(text, 48)}
      </Typography.Text>
    </Popover>
  );
}
