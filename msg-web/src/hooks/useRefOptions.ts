/**
 * # 外键下拉选项
 *
 * `ref` 字段（消息ID、渠道ID、目标类别ID……）在表格里要显示名称、在表单里要下拉选择，
 * 所以每页初始化时把这些被引用实体的**全量列表**拉一次，在内存里做 id → 记录映射。
 *
 * 数量级是「几十个渠道 / 几百条消息定义」，一次拉完比逐条查名称划算，也不需要缓存层。
 *
 * ## 生产环境为什么要兜底
 *
 * 有些部署的网关只放行主列表用的 `/page`，`/list` 会 404，于是批量映射为空、
 * 表格里的「消息类别」只剩一串短 ID。这里改成优先走 `/page`（与主列表同款接口），
 * 失败再退回 `/list`；万一两个批量接口都拿不到，还会对当前页出现、但映射里没有的
 * 外键 ID 按主键单条补查。所有 ID 一律按字符串比对：线上 `u64` 可能是字符串也可能是
 * 数字，Map 的键类型不统一就会查不中名称。
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

/** 批量拉取的单页上限；超过就继续翻页，最多 10 页 */
const PAGE_SIZE = 500;
const MAX_PAGES = 10;

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

/** # 批量拉取一个实体的全部记录 */
async function fetchRows(path: string): Promise<Row[]> {
  const api = crudOf(path);
  try {
    // 与主列表同款接口，生产网关一定放行；分页拉全，避免选项被首屏条数截断
    const first = await api.page({ _page: 1, _size: PAGE_SIZE });
    const rows = [...first.list];
    const total = Number(first.total) || 0;
    for (let page = 2; rows.length < total && page <= MAX_PAGES; page += 1) {
      const next = await api.page({ _page: page, _size: PAGE_SIZE });
      if (!next.list.length) {
        break;
      }
      rows.push(...next.list);
    }
    return rows;
  } catch {
    // /page 不可用时退回 /list（不分页，一次性给全）
    try {
      return await api.list();
    } catch {
      // 选项拉不到不该让整页报错：控件降级成空下拉，主列表照常显示
      return [];
    }
  }
}

/** 单条补查结果缓存：`path:id` 为键，失败也记下来避免反复打接口 */
const lazyCache = new Map<string, Row>();
const lazyFailed = new Set<string>();
const lazyInflight = new Map<string, Promise<Row | undefined>>();

/** # 按主键补查一条外键记录 */
async function fetchRowById(path: string, id: string): Promise<Row | undefined> {
  const token = `${path}:${id}`;
  const hit = lazyCache.get(token);
  if (hit) {
    return hit;
  }
  if (lazyFailed.has(token)) {
    return undefined;
  }
  let task = lazyInflight.get(token);
  if (!task) {
    task = crudOf(path)
      .byId(id)
      .then((row) => {
        lazyCache.set(token, row);
        return row;
      })
      .catch(() => {
        lazyFailed.add(token);
        return undefined;
      })
      .finally(() => lazyInflight.delete(token));
    lazyInflight.set(token, task);
  }
  return task;
}

/** # 调用方给出的「字段名 → 实体 key」对应关系 */
export interface RefField {
  key: string;
  ref?: string;
}

/**
 * # 批量拉取外键选项，并补齐缺失的名称
 *
 * ## 参数
 * * `keys` - 实体 key 列表，可重复、可传空
 * * `rows` - 当前页记录，用来发现批量选项没覆盖到的外键 ID
 * * `refFields` - 本页的 `ref` 字段（`key` / `ref`）
 *
 * ## 返回值
 * `key → { options, byId, loading }`；未命中的 key 会得到空集合
 */
export function useRefOptions(
  keys: string[],
  rows: Row[] = [],
  refFields: RefField[] = [],
): Record<string, RefOptions> {
  const signature = keys.join(',');
  const [state, setState] = useState<Record<string, RefOptions>>({});
  // 单条补查落在模块缓存里，用一个计数器触发重渲染
  const [bump, setBump] = useState(0);

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
        const records = await fetchRows(config.path);
        return [
          key,
          {
            options: records
              .map((row) => ({ value: String(row.id), label: labelOf(row) }))
              .sort((left, right) => left.label.localeCompare(right.label)),
            // 统一按字符串建索引，线上 ID 是数字形态时也能查中名称
            byId: new Map(records.map((row) => [String(row.id), row])),
            loading: false,
          },
        ];
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

  /** 批量结果出来后，当前页里仍查不到名称的外键 ID（去重排序后作为 effect 依赖） */
  const missingSignature = useMemo(() => {
    const unique = signature ? [...new Set(signature.split(','))] : [];
    // 批量请求在飞时不抢跑，否则会白打一堆单条查询
    if (unique.some((key) => !state[key] || state[key].loading)) {
      return '';
    }
    const tokens = new Set<string>();
    for (const field of refFields) {
      const key = field.ref;
      if (!key || !unique.includes(key)) {
        continue;
      }
      const path = entityOf(key)?.path;
      if (!path) {
        continue;
      }
      for (const row of rows) {
        const raw = row[field.key];
        const id = raw === undefined || raw === null ? '' : String(raw);
        if (!id || state[key].byId.has(id)) {
          continue;
        }
        const token = `${path}:${id}`;
        if (lazyCache.has(token) || lazyFailed.has(token)) {
          continue;
        }
        tokens.add(token);
      }
    }
    return [...tokens].sort().join('|');
  }, [signature, state, rows, refFields]);

  useEffect(() => {
    if (!missingSignature) {
      return;
    }
    Promise.all(
      missingSignature.split('|').map((token) => {
        const at = token.lastIndexOf(':');
        return fetchRowById(token.slice(0, at), token.slice(at + 1));
      }),
    ).then((results) => {
      if (results.some(Boolean)) {
        setBump((value) => value + 1);
      }
    });
  }, [missingSignature]);

  // 未返回前按空集合兜底，避免调用方到处写 `?.`
  return useMemo(() => {
    const keysInUse = signature ? signature.split(',') : [];
    const result: Record<string, RefOptions> = {};
    for (const key of keysInUse) {
      const base = state[key] ?? EMPTY;
      const path = entityOf(key)?.path;
      const byId = new Map(base.byId);
      if (path) {
        for (const [token, row] of lazyCache) {
          if (!token.startsWith(`${path}:`)) {
            continue;
          }
          const id = token.slice(path.length + 1);
          if (!byId.has(id)) {
            byId.set(id, row);
          }
        }
      }
      // 批量选项为空时，用补查到的记录兜底，编辑表单至少能显示/选中当前值
      const options = base.options.length
        ? base.options
        : [...byId.values()].map((row) => ({ value: String(row.id), label: labelOf(row) }));
      result[key] = { ...base, byId, options };
    }
    return result;
    // bump 只用于在单条补查落缓存后重算合并结果
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state, signature, bump]);
}
