/**
 * # 用户目录（跨服务只读）
 *
 * 消息目标里 user / role / org 三类目标存的是 adm 的用户/角色/组织 ID。
 * 这里用统一登录态直接读 adm 的只读列表，供「目标标识」选择面板使用。
 * 读不到时由上层退化为手动填写 ID，不阻塞消息目标维护。
 */

import { AUTH_BASE, authHeaders, handleUnauthorized } from '../lib/auth-core';

/** 目录种类 */
export type DirectoryKind = 'user' | 'role' | 'org';

/** 一个可选目标 */
export interface DirectoryEntry {
  id: string;
  name: string;
  /** 主文案：组织用全名，其余用名称 */
  label: string;
  /** 次要信息：完整 ID、组织路径 */
  detail?: string;
  code?: string;
  fullName?: string;
  treeCode?: string;
  groupId?: string;
  /** 角色所属分组名称（角色目录专用） */
  groupLabel?: string;
  enabled?: boolean;
}

/** 左侧树节点 */
export interface DirectoryTreeNode {
  id: string;
  label: string;
  treeCode?: string;
  disabled?: boolean;
}

/** 领域 tab：与 adm 用户管理页同一套数据域切换 */
export interface DirectoryRealm {
  id: string;
  label: string;
}

/** 领域 ↔ 用户绑定 */
export interface RealmUserBinding {
  id: string;
  realmId: string;
  userId: string;
}

/** 领域 ↔ 组织绑定（绑定的是子树根） */
export interface RealmOrgBinding {
  id: string;
  realmId: string;
  orgId: string;
}

/** 一份目录：条目 + 左树 + 条目归属的分组 id 集合 */
export interface DirectoryBundle {
  entries: DirectoryEntry[];
  tree: DirectoryTreeNode[];
  groupsByEntry: Map<string, string[]>;
  /** 领域 tab（用户目录才有） */
  realms?: DirectoryRealm[];
  /** 条目所属领域 id 集合 */
  realmsByEntry?: Map<string, string[]>;
  /** 左树节点所属领域 id 集合 */
  realmsByTree?: Map<string, string[]>;
}

/** adm 角色分组 */
export interface RoleGroupEntry {
  id: string;
  name: string;
  treeCode: string;
}

/** adm 组织 ↔ 用户绑定 */
export interface OrgUserBinding {
  id: string;
  orgId: string;
  userId: string;
}

/** adm 列表返回的最小字段集合 */
interface DirectoryRow {
  id?: unknown;
  name?: unknown;
  nickName?: unknown;
  code?: unknown;
  fullName?: unknown;
  treeCode?: unknown;
  groupId?: unknown;
  enabled?: unknown;
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

/** 统一 GET 列表：登录失效时清理会话并抛错。 */
async function requestList<T>(path: string, orderBy?: string): Promise<T[]> {
  const query = orderBy ? `?_orderBy=${encodeURIComponent(orderBy)}` : '';
  let response: Response;
  try {
    response = await fetch(`${AUTH_BASE}${path}${query}`, {
      headers: { Accept: 'application/json', ...authHeaders() },
    });
  } catch (error) {
    throw new Error(`无法连接用户目录：${error instanceof Error ? error.message : String(error)}`);
  }

  const text = await response.text();
  let payload: Ro<T[]> | undefined;
  try {
    payload = text ? (JSON.parse(text) as Ro<T[]>) : undefined;
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

  return payload.extra ?? [];
}

function asText(value: unknown): string | undefined {
  return typeof value === 'string' && value ? value : undefined;
}

/** 把 adm 行转成目录条目；重名时靠完整 ID 区分。 */
function toEntry(kind: DirectoryKind, row: DirectoryRow): DirectoryEntry | undefined {
  const id = row.id === undefined || row.id === null ? '' : String(row.id);
  if (!id) return undefined;
  const name = asText(row.name) ?? id;
  const fullName = asText(row.fullName);
  const treeCode = asText(row.treeCode);
  const groupId = asText(row.groupId);
  const code = asText(row.code);
  return {
    id,
    name,
    label: kind === 'org' ? fullName ?? name : name,
    detail: kind === 'org' ? [treeCode, `#${id}`].filter(Boolean).join(' · ') : `#${id}`,
    code,
    fullName,
    treeCode,
    groupId,
    enabled: typeof row.enabled === 'boolean' ? row.enabled : undefined,
  };
}

/** 拉取某类目录的条目（未附加左树与分组关系）。 */
export async function loadDirectory(kind: DirectoryKind): Promise<DirectoryEntry[]> {
  const rows = await requestList<DirectoryRow>(PATHS[kind], ORDERS[kind]);
  return rows.map((row) => toEntry(kind, row)).filter((item): item is DirectoryEntry => item !== undefined);
}

/** 拉取 adm 角色分组。 */
export async function loadRoleGroups(): Promise<RoleGroupEntry[]> {
  const rows = await requestList<{ id?: unknown; name?: unknown; treeCode?: unknown }>(
    '/role-group/list',
    'tree_code:asc',
  );
  return rows
    .map((row): RoleGroupEntry | undefined => {
      const id = row.id === undefined || row.id === null ? '' : String(row.id);
      const treeCode = asText(row.treeCode);
      if (!id || !treeCode) return undefined;
      return { id, name: asText(row.name) ?? id, treeCode };
    })
    .filter((item): item is RoleGroupEntry => item !== undefined);
}

/** 拉取 adm 组织 ↔ 用户绑定。 */
export async function loadOrgUsers(): Promise<OrgUserBinding[]> {
  const rows = await requestList<{ id?: unknown; orgId?: unknown; userId?: unknown }>('/org-user/list');
  return rows
    .map((row): OrgUserBinding | undefined => {
      const id = row.id === undefined || row.id === null ? '' : String(row.id);
      const orgId = row.orgId === undefined || row.orgId === null ? '' : String(row.orgId);
      const userId = row.userId === undefined || row.userId === null ? '' : String(row.userId);
      if (!orgId || !userId) return undefined;
      return { id: id || `${orgId}:${userId}`, orgId, userId };
    })
    .filter((item): item is OrgUserBinding => item !== undefined);
}

/** 拉取 adm 领域（只保留启用领域，与用户管理页 tab 一致）。 */
export async function loadRealms(): Promise<DirectoryRealm[]> {
  const rows = await requestList<{ id?: unknown; name?: unknown; enabled?: unknown }>(
    '/realm/list',
    'sort_order:asc',
  );
  return rows
    .map((row): DirectoryRealm | undefined => {
      const id = row.id === undefined || row.id === null ? '' : String(row.id);
      if (!id || row.enabled === false) return undefined;
      return { id, label: asText(row.name) ?? id };
    })
    .filter((item): item is DirectoryRealm => item !== undefined);
}

/** 拉取 adm 领域 ↔ 用户绑定。 */
export async function loadRealmUsers(): Promise<RealmUserBinding[]> {
  const rows = await requestList<{ id?: unknown; realmId?: unknown; userId?: unknown }>('/realm-user/list');
  return rows
    .map((row): RealmUserBinding | undefined => {
      const id = row.id === undefined || row.id === null ? '' : String(row.id);
      const realmId = row.realmId === undefined || row.realmId === null ? '' : String(row.realmId);
      const userId = row.userId === undefined || row.userId === null ? '' : String(row.userId);
      if (!realmId || !userId) return undefined;
      return { id: id || `${realmId}:${userId}`, realmId, userId };
    })
    .filter((item): item is RealmUserBinding => item !== undefined);
}

/** 拉取 adm 领域 ↔ 组织绑定。 */
export async function loadRealmOrgs(): Promise<RealmOrgBinding[]> {
  const rows = await requestList<{ id?: unknown; realmId?: unknown; orgId?: unknown }>('/realm-org/list');
  return rows
    .map((row): RealmOrgBinding | undefined => {
      const id = row.id === undefined || row.id === null ? '' : String(row.id);
      const realmId = row.realmId === undefined || row.realmId === null ? '' : String(row.realmId);
      const orgId = row.orgId === undefined || row.orgId === null ? '' : String(row.orgId);
      if (!realmId || !orgId) return undefined;
      return { id: id || `${realmId}:${orgId}`, realmId, orgId };
    })
    .filter((item): item is RealmOrgBinding => item !== undefined);
}

/** 辅助数据（左树、领域关系）失败不拖垮主表：返回成功标记与空数组兜底。 */
async function settle<T>(promise: Promise<T[]>): Promise<{ data: T[]; ok: boolean }> {
  try {
    return { data: await promise, ok: true };
  } catch {
    return { data: [], ok: false };
  }
}

/** 组织条目 → 左树节点。 */
function orgTreeOf(entries: DirectoryEntry[]): DirectoryTreeNode[] {
  return entries.map((entry) => ({
    id: entry.id,
    label: entry.fullName ?? entry.name,
    treeCode: entry.treeCode,
    disabled: entry.enabled === false,
  }));
}

/** 拉齐一份目录需要的条目、左树与分组关系。 */
export async function loadBundle(kind: DirectoryKind): Promise<DirectoryBundle> {
  if (kind === 'org') {
    // 领域关系是辅助信息，拿不到也要让右侧组织表可用。
    const [entries, realmRes, realmOrgRes] = await Promise.all([
      loadDirectory('org'),
      settle(loadRealms()),
      settle(loadRealmOrgs()),
    ]);
    const realmsByEntry = new Map<string, string[]>();
    const orgById = new Map(entries.map((entry) => [entry.id, entry]));
    for (const binding of realmOrgRes.data) {
      const root = orgById.get(binding.orgId);
      if (!root?.treeCode) continue;
      for (const org of entries) {
        const code = org.treeCode ?? '';
        const extra = code.length - root.treeCode.length;
        if (extra < 0 || extra % 3 !== 0 || !code.startsWith(root.treeCode)) continue;
        const list = realmsByEntry.get(org.id) ?? [];
        list.push(binding.realmId);
        realmsByEntry.set(org.id, list);
      }
    }
    return {
      entries,
      tree: orgTreeOf(entries),
      groupsByEntry: new Map(entries.map((entry) => [entry.id, [entry.id]])),
      // 两张表齐了才暴露 tab，否则切过去组织会被全部筛掉。
      realms: realmRes.ok && realmOrgRes.ok ? realmRes.data : [],
      realmsByEntry,
      realmsByTree: realmsByEntry,
    };
  }

  if (kind === 'role') {
    // 角色分组的左树是辅助信息，拿不到也要让右侧角色表可用。
    const [entries, groups] = await Promise.all([
      loadDirectory('role'),
      loadRoleGroups().catch(() => [] as RoleGroupEntry[]),
    ]);
    const groupNameById = new Map(groups.map((group) => [group.id, group.name]));
    return {
      entries: entries.map((entry) => ({
        ...entry,
        groupLabel: entry.groupId ? groupNameById.get(entry.groupId) ?? '未分组' : '未分组',
      })),
      tree: groups.map((group) => ({ id: group.id, label: group.name, treeCode: group.treeCode })),
      groupsByEntry: new Map(entries.map((entry) => [entry.id, entry.groupId ? [entry.groupId] : []])),
    };
  }

  // 组织树与领域关系是辅助信息，拿不到也要让右侧用户表可用。
  const [entries, orgs, bindings, realmRes, realmUserRes, realmOrgRes] = await Promise.all([
    loadDirectory('user'),
    settle(loadDirectory('org')),
    settle(loadOrgUsers()),
    settle(loadRealms()),
    settle(loadRealmUsers()),
    settle(loadRealmOrgs()),
  ]);

  const groupsByEntry = new Map<string, string[]>();
  for (const binding of bindings.data) {
    const list = groupsByEntry.get(binding.userId) ?? [];
    list.push(binding.orgId);
    groupsByEntry.set(binding.userId, list);
  }

  // 三张领域关系表齐了才暴露 tab，否则切过去人员会被全部筛掉。
  const realms = realmRes.ok && realmUserRes.ok && realmOrgRes.ok ? realmRes.data : [];
  const realmsByEntry = new Map<string, string[]>();
  for (const binding of realmUserRes.data) {
    const list = realmsByEntry.get(binding.userId) ?? [];
    list.push(binding.realmId);
    realmsByEntry.set(binding.userId, list);
  }
  const orgById = new Map(orgs.data.map((org) => [org.id, org]));
  const realmsByTree = new Map<string, string[]>();
  for (const binding of realmOrgRes.data) {
    const root = orgById.get(binding.orgId);
    if (!root?.treeCode) continue;
    for (const org of orgs.data) {
      const code = org.treeCode ?? '';
      const extra = code.length - root.treeCode.length;
      if (extra < 0 || extra % 3 !== 0 || !code.startsWith(root.treeCode)) continue;
      const list = realmsByTree.get(org.id) ?? [];
      list.push(binding.realmId);
      realmsByTree.set(org.id, list);
    }
  }

  return {
    entries,
    tree: orgTreeOf(orgs.data),
    groupsByEntry,
    realms,
    realmsByEntry,
    realmsByTree,
  };
}
