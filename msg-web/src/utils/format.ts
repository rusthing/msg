/**
 * # 格式化工具
 *
 * 约定与 adm-web 保持一致：空值统一 `--`，时间只展示本地时间到秒，
 * 长 ID 在表格里截断、完整值放 `title` / 详情。
 */

import dayjs from 'dayjs';

/** 空值统一占位符 */
export const EMPTY = '--';

/**
 * # 毫秒时间戳转本地时间
 *
 * 服务端的 `create_ms` / `update_ms` 是 `u64`，线上是**字符串**（`"1760000000000"`）。
 * 必须先用 `Number()` 转成数字再交给 dayjs：`dayjs("1760000000000")` 会把它当日期字符串解析，
 * 得到 1759 年这种荒唐结果。毫秒时间戳约 `1.7e12`，远小于 `2^53`，转数字不会丢精度。
 */
export function formatMs(ms?: string | number | null): string {
  if (ms === undefined || ms === null || ms === '') {
    return EMPTY;
  }
  const value = Number(ms);
  if (!Number.isFinite(value) || value <= 0) {
    return EMPTY;
  }
  const date = dayjs(value);
  return date.isValid() ? date.format('YYYY-MM-DD HH:mm:ss') : EMPTY;
}

/**
 * # 毫秒时间戳，带毫秒位
 *
 * 投递日志用来算端到端耗时，只到秒看不出差别。
 */
export function formatMsExact(ms?: string | number | null): string {
  if (ms === undefined || ms === null || ms === '') {
    return EMPTY;
  }
  const value = Number(ms);
  if (!Number.isFinite(value) || value <= 0) {
    return EMPTY;
  }
  const date = dayjs(value);
  return date.isValid() ? date.format('YYYY-MM-DD HH:mm:ss.SSS') : EMPTY;
}

/**
 * # 两个时间点之间的耗时
 *
 * 入参都是 `u64` 字符串，先转数字再相减；任一端缺失就返回 `--`。
 * 用于投递链路的「首收 → 落库」「接收 → 发送完成」这类差值列。
 */
export function durationMs(from?: string | number | null, to?: string | number | null): string {
  if (from === undefined || from === null || from === '' || to === undefined || to === null || to === '') {
    return EMPTY;
  }
  const start = Number(from);
  const end = Number(to);
  if (!Number.isFinite(start) || !Number.isFinite(end) || start <= 0 || end <= 0) {
    return EMPTY;
  }
  const delta = end - start;
  return delta >= 0 ? `${delta} ms` : `-${Math.abs(delta)} ms`;
}

/** # 空值兜底 */
export function orDash(value?: string | number | null): string {
  if (value === undefined || value === null || value === '') {
    return EMPTY;
  }
  return String(value);
}

/**
 * # 长 ID 截断
 *
 * 雪花 ID 有 19 位，表格里完整展示会挤掉别的列；详情与 `title` 里保留完整值。
 */
export function shortId(id?: string | number | null): string {
  if (id === undefined || id === null || id === '') {
    return EMPTY;
  }
  const text = String(id);
  return text.length <= 10 ? text : `${text.slice(0, 6)}…${text.slice(-4)}`;
}

/** # 美化 JSON 文本，解析失败时原样返回 */
export function prettyJson(text?: string | null): string {
  if (!text) {
    return '';
  }
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

/**
 * # 从异常中提取可展示的文案
 *
 * 表单校验失败抛出的是 antd 的 `errorFields` 对象，不应该直接甩给用户看。
 * `ApiError` 带业务编码与 detail 时一起展示，避免用户只看到一句「请求失败」。
 */
export function errorMessage(error: unknown): string {
  if (error instanceof Error) {
    const extra = error as Error & { code?: string; detail?: string };
    const head = extra.code ? `[${extra.code}] ${error.message}` : error.message;
    if (extra.detail && extra.detail !== error.message) {
      return `${head}（${truncate(extra.detail, 160)}）`;
    }
    return head;
  }
  if (typeof error === 'string') {
    return error;
  }
  return '操作失败，请稍后重试';
}

/** # 字符串截断，用于表格里展示长 JSON / 长模板 */
export function truncate(text: string, max = 60): string {
  return text.length > max ? `${text.slice(0, max)}…` : text;
}
