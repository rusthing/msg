/**
 * # 外键下拉选项
 *
 * `ref` 字段（消息ID、渠道ID、目标类别ID……）在表格里要显示名称、在表单里要下拉选择，
 * 所以每页初始化时把这些被引用实体的**全量列表**拉一次，在内存里做 id → 记录映射。
 *
 * 数量级是「几十个渠道 / 几百条消息定义」，一次拉完比逐条查名称划算，也不需要缓存层。
 */

import { useEffect, useMemo, useState } from 'react';
import { crudOf } from '../api/crud';
import type { Row } from '../api/types';
import { entityOf } from '../entities';

/** # 一个实体的下拉选项集合 */
export interface RefOptions {
  /** antd Select 选项 */
  options: { value: string; label: string }[];
  /** id → 记录，用于表格里把 ID 翻成名称 */
  byId: Map<string, Row>;
  /** 是否还在拉取 */
  loading: boolean;
}

/** 空集合，请求未返回或实体不存在时用 */
const EMPTY: RefOptions = { options: [], byId: new Map(), loading: false };

/** # 从记录里取可读名称 */
function labelOf(row: Row): string {
  const name = row.name;
  const code = row.code;
  if (typeof name === 'string' && name) {
    return typeof code === 'string' && code ? `${name}（${code}）` : name;
  }
  if (typeof code === 'string' && code) {
    return code;
  }
  return String(row.id);
}

/**
 * # 批量拉取外键选项
 *
 * ## 参数
 * * `keys` - 实体 key 列表，可重复、可传空
 *
 * ## 返回值
 * `key → { options, byId, loading }`；未命中的 key 会得到空集合
 */
export function useRefOptions(keys: string[]): Record<string, RefOptions> {
  const signature = keys.join(',');
  const [state, setState] = useState<Record<string, RefOptions>>({});

  useEffect(() => {
    const unique = signature ? [...new Set(signature.split(','))] : [];
    if (unique.length === 0) {
      setState({});
      return;
    }

    let alive = true;
    setState(
      Object.fromEntries(unique.map((key) => [key, { ...EMPTY, loading: true } as RefOptions])),
    );

    Promise.all(
      unique.map(async (key): Promise<[string, RefOptions]> => {
        const config = entityOf(key);
        if (!config) {
          return [key, EMPTY];
        }
        try {
          const rows = await crudOf(config.path).list();
          return [
            key,
            {
              options: rows
                .map((row) => ({ value: row.id, label: labelOf(row) }))
                .sort((left, right) => left.label.localeCompare(right.label)),
              byId: new Map(rows.map((row) => [row.id, row])),
              loading: false,
            },
          ];
        } catch {
          // 选项拉不到不该让整页报错：控件降级成空下拉，主列表照常显示
          return [key, EMPTY];
        }
      }),
    ).then((entries) => {
      if (alive) {
        setState(Object.fromEntries(entries));
      }
    });

    return () => {
      alive = false;
    };
  }, [signature]);

  // 未返回前按空集合兜底，避免调用方到处写 `?.`
  return useMemo(() => {
    const keysInUse = signature ? signature.split(',') : [];
    const result: Record<string, RefOptions> = {};
    for (const key of keysInUse) {
      result[key] = state[key] ?? EMPTY;
    }
    return result;
  }, [state, signature]);
}
