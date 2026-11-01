/**
 * # 通用 CRUD 客户端
 *
 * 14 张表的路由形状完全一致（`#[router(crud)]` 生成），所以不为每张表各写一份 api 文件，
 * 而是按「资源路径」造一个客户端：
 *
 * | 动作 | 方法与路径 |
 * | --- | --- |
 * | 分页 | `GET /{前缀}/{资源}/page` |
 * | 条件列表 | `GET /{前缀}/{资源}/list` |
 * | 主键查询 | `GET /{前缀}/{资源}/{id}` |
 * | 新增 | `POST /{前缀}/{资源}` |
 * | 修改 | `PUT /{前缀}/{资源}` |
 * | 删除 | `DELETE /{前缀}/{资源}/{id}` |
 *
 * ## 两个必须知道的细节
 *
 * - **分页参数叫 `_page` / `_size`**，不是 `pageNum` / `pageSize`；排序是 `_orderBy`
 *   （如 `createMs desc`），关键字是 `_keyword`（服务端做模糊搜索）。
 * - **查询条件全是等值匹配**：`#[crud_dto]` 生成的 QueryDto 把每个业务列都映射成
 *   `Column::Xxx.eq(v)`，只有 `_keyword` 是模糊的。想要「包含」只能在关键字框里搜。
 */

import { apiDelete, apiGet, apiGetList, apiGetPage, apiPost, apiPut } from './client';
import type { QueryParams } from './client';
import type { Id, PageRx, Row } from './types';

/** # 一个资源上可用的动作 */
export interface CrudClient<T extends { id: Id } = Row> {
  /** 分页查询，`params` 里需自带 `_page` / `_size`（由 `usePagedQuery` 补齐） */
  page: (params?: QueryParams) => Promise<PageRx<T>>;
  /** 条件列表，不带分页；适合下拉选项与统计 */
  list: (params?: QueryParams) => Promise<T[]>;
  /** 主键查询 */
  byId: (id: string) => Promise<T>;
  /** 新增，返回写入后的 VO */
  add: (body: Record<string, unknown>) => Promise<T>;
  /** 修改，`body` 必须含 `id` */
  modify: (body: Record<string, unknown>) => Promise<T>;
  /** 按主键删除 */
  remove: (id: string) => Promise<T>;
}

/**
 * # 按资源路径造 CRUD 客户端
 *
 * ## 参数
 * * `path` - 资源根路径，含服务端路由前缀，如 `/msg/message`
 */
export function crudOf<T extends { id: Id } = Row>(path: string): CrudClient<T> {
  return {
    page: (params) => apiGetPage<T>(`${path}/page`, params),
    list: (params) => apiGetList<T>(`${path}/list`, params),
    byId: (id) => apiGet<T>(`${path}/${id}`),
    add: (body) => apiPost<T>(path, body),
    modify: (body) => apiPut<T>(path, body),
    remove: (id) => apiDelete<T>(`${path}/${id}`),
  };
}
