/**
 * # 用户目录（跨服务只读）
 *
 * 消息目标里 user / role / org 三类目标存的是 adm 的用户/角色/组织 ID。
 * 这里用统一登录态直接读 adm 的只读列表，供「目标标识」下拉选择；
 * 读不到时由上层退化为手动填写 ID，不阻塞消息目标维护。
 */

import { AUTH_BASE, authHeaders, handleUnauthorized } from '../lib/auth-core';

/** 目录种类 */
export type DirectoryKind = 'user' | 'role' | 'org';

/** 一个可选目标 */
export interface DirectoryOption {
  value: string;
  label: string;
  /** 次要信息：完整 ID，供下拉第二行展示 */
  detail?: string;
}

/** adm 列表返回的最小字段集合 */
interface DirectoryRow {
  id?: unknown;
  name?: unknown;
  code?: unknown;
  fullName?: unknown;
}

interface Ro<T> {
  result?: number;
  msg?: string;
  code?: string;
  extra?: T;
}

const PATHS: Record<DirectoryKind, string> = {
  user: '/user/list',
  role: '/role/list',
  org: '/org/list',
};

const ORDERS: Record<DirectoryKind, string> = {
  user: 'name:asc',
  role: 'sort_order:asc',
  org: 'tree_code:asc',
};

/** 列表项的展示文案：名称一行，ID 一行；重名时靠 ID 区分 */
function labelOf(kind: DirectoryKind, row: DirectoryRow, id: string): string {
  const name = typeof row.name === 'string' && row.name ? row.name : id;
  if (kind === 'org') {
    return typeof row.fullName === 'string' && row.fullName ? row.fullName : name;
  }
  return name;
}

/** 拉取某类目录的全量选项 */
export async function loadDirectory(kind: DirectoryKind): Promise<DirectoryOption[]> {
  const url = `${AUTH_BASE}${PATHS[kind]}?_orderBy=${encodeURIComponent(ORDERS[kind])}`;

  let response: Response;
  try {
    response = await fetch(url, {
      headers: { Accept: 'application/json', ...authHeaders() },
    });
  } catch (error) {
    throw new Error(`无法连接用户目录：${error instanceof Error ? error.message : String(error)}`);
  }

  const text = await response.text();
  let payload: Ro<DirectoryRow[]> | undefined;
  try {
    payload = text ? (JSON.parse(text) as Ro<DirectoryRow[]>) : undefined;
  } catch {
    payload = undefined;
  }

  if (response.status === 401 || payload?.code === 'ADM00005') {
    handleUnauthorized();
    throw new Error('登录状态已失效，请重新登录');
  }
  if (!response.ok || !payload || payload.result !== 1) {
    throw new Error(payload?.msg || `目录接口返回异常（HTTP ${response.status}）`);
  }

  return (payload.extra ?? [])
    .map((row): DirectoryOption | undefined => {
      const id = row.id === undefined || row.id === null ? '' : String(row.id);
      return id ? { value: id, label: labelOf(kind, row, id), detail: `#${id}` } : undefined;
    })
    .filter((item): item is DirectoryOption => item !== undefined);
}
