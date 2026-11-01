/**
 * # 字典
 *
 * 服务端用 `#[dic]` 标注的枚举**序列化为整数判别式**（`i16`），反序列化也只认整数，
 * 所以前端只能靠这张表把数字翻译成文案与颜色。改枚举时必须同步这里，
 * 否则页面上会出现「未知(9)」这种兜底文案。
 *
 * ## 三个状态枚举的分工
 *
 * - {@link deliverStatusDic} —— 一条投递的**汇总**结论（部分成功也算成功）；
 * - {@link deliverTargetStatusDic} —— 单个投递目标（用户/角色/大屏）的结论，多了「已读」；
 * - {@link deliverChannelStatusDic} —— 单个目标的单个渠道的结论，没有「部分成功」。
 *
 * 三层是从粗到细的关系：投递 → 投递目标 → 投递渠道，排查时按这个顺序下钻。
 */

import type { DeliverChannelStatus, DeliverStatus, DeliverTargetStatus } from '../api/types';

/** # 字典项 */
export interface DicItem<V extends number = number> {
  value: V;
  label: string;
  /** antd Tag 的预设色 */
  color: string;
}

/** # 投递状态（0-投递中、1-部分成功、2-全部成功、3-全部失败） */
export const deliverStatusDic: DicItem<DeliverStatus>[] = [
  { value: 0, label: '投递中', color: 'processing' },
  { value: 1, label: '部分成功', color: 'warning' },
  { value: 2, label: '全部成功', color: 'success' },
  { value: 3, label: '全部失败', color: 'error' },
];

/** # 投递目标状态（0-投递中、1-成功、2-失败、3-已读） */
export const deliverTargetStatusDic: DicItem<DeliverTargetStatus>[] = [
  { value: 0, label: '投递中', color: 'processing' },
  { value: 1, label: '投递成功', color: 'success' },
  { value: 2, label: '投递失败', color: 'error' },
  { value: 3, label: '目标已读', color: 'cyan' },
];

/** # 渠道投递状态（0-投递中、1-成功、2-失败） */
export const deliverChannelStatusDic: DicItem<DeliverChannelStatus>[] = [
  { value: 0, label: '投递中', color: 'processing' },
  { value: 1, label: '投递成功', color: 'success' },
  { value: 2, label: '投递失败', color: 'error' },
];

/** # 取字典项 */
export function dicItem<V extends number>(
  dic: DicItem<V>[],
  value: V | null | undefined,
): DicItem<V> | undefined {
  return dic.find((item) => item.value === value);
}

/** # 取字典文案，未命中时给出可读兜底 */
export function dicLabel<V extends number>(
  dic: DicItem<V>[],
  value: V | null | undefined,
): string {
  if (value === null || value === undefined) {
    return '-';
  }
  return dicItem(dic, value)?.label ?? `未知(${value})`;
}

/** # 转成 antd Select 的选项 */
export function dicOptions<V extends number>(
  dic: DicItem<V>[],
): { value: V; label: string }[] {
  return dic.map((item) => ({ value: item.value, label: item.label }));
}
