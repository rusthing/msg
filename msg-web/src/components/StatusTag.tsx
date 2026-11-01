/**
 * # 状态标签
 *
 * 三个投递状态字典共用一个组件：传哪个字典由调用方决定，避免为每个枚举各写一个 Tag。
 * 未知取值也要显式说出来，别让单元格变成空白。
 */

import { Tag } from 'antd';
import type { DicItem } from '../dic';
import { dicItem } from '../dic';

interface StatusTagProps {
  /** 字典表 */
  dic: DicItem[];
  /** 服务端返回的整数判别式 */
  value?: number | null;
}

export default function StatusTag({ dic, value }: StatusTagProps) {
  if (value === null || value === undefined) {
    return <Tag>-</Tag>;
  }
  const item = dicItem(dic, value);
  if (!item) {
    return <Tag color="default">未知({value})</Tag>;
  }
  return <Tag color={item.color}>{item.label}</Tag>;
}
