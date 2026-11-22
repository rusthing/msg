/**
 * # 目录选项 Hook
 *
 * 目标标识按目标类别切换目录（用户 / 角色 / 组织）。同一类别在本次会话里只拉一次，
 * 切换类别或重新打开表单时复用缓存，避免每次弹窗都打一遍 adm。
 *
 * 一份目录包含三样东西：可选项、左树（组织树 / 角色分组树）、条目归属的分组集合，
 * 正好对应「用户管理式选择面板」需要的右表与左树。
 */

import { useEffect, useState } from 'react';
import { loadBundle, type DirectoryBundle, type DirectoryKind } from '../api/directory';

/** 进程内缓存：目录变动不频繁，切换类别时不必重复请求 */
const cache = new Map<DirectoryKind, DirectoryBundle>();

const EMPTY: DirectoryBundle = { entries: [], tree: [], groupsByEntry: new Map() };

export interface DirectoryState {
  bundle: DirectoryBundle;
  loading: boolean;
  error: Error | null;
}

/** 按需拉取某类目录；`kind` 为空或表单未打开时不请求 */
export function useDirectoryOptions(kind: DirectoryKind | undefined, enabled: boolean): DirectoryState {
  const [state, setState] = useState<DirectoryState>(() => {
    const cached = kind ? cache.get(kind) : undefined;
    return { bundle: cached ?? EMPTY, loading: false, error: null };
  });

  useEffect(() => {
    if (!kind || !enabled) {
      setState({ bundle: kind ? cache.get(kind) ?? EMPTY : EMPTY, loading: false, error: null });
      return;
    }
    const cached = cache.get(kind);
    if (cached) {
      setState({ bundle: cached, loading: false, error: null });
      return;
    }

    let alive = true;
    setState({ bundle: EMPTY, loading: true, error: null });
    loadBundle(kind)
      .then((bundle) => {
        cache.set(kind, bundle);
        if (alive) {
          setState({ bundle, loading: false, error: null });
        }
      })
      .catch((reason: unknown) => {
        if (alive) {
          setState({
            bundle: EMPTY,
            loading: false,
            error: reason instanceof Error ? reason : new Error(String(reason)),
          });
        }
      });

    return () => {
      alive = false;
    };
  }, [kind, enabled]);

  return state;
}
