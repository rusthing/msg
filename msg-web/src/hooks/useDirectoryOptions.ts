/**
 * # 目录选项 Hook
 *
 * 目标标识按目标类别切换目录（用户 / 角色 / 组织）。同一类别在本次会话里只拉一次，
 * 切换类别或重新打开表单时复用缓存，避免每次弹窗都打一遍 adm。
 */

import { useEffect, useState } from 'react';
import { loadDirectory, type DirectoryKind, type DirectoryOption } from '../api/directory';

/** 进程内缓存：目录变动不频繁，切换类别时不必重复请求 */
const cache = new Map<DirectoryKind, DirectoryOption[]>();

export interface DirectoryState {
  options: DirectoryOption[];
  loading: boolean;
  error: Error | null;
}

/** 按需拉取某类目录；`kind` 为空或表单未打开时不请求 */
export function useDirectoryOptions(kind: DirectoryKind | undefined, enabled: boolean): DirectoryState {
  const [state, setState] = useState<DirectoryState>(() => {
    const cached = kind ? cache.get(kind) : undefined;
    return { options: cached ?? [], loading: false, error: null };
  });

  useEffect(() => {
    if (!kind || !enabled) {
      setState({ options: kind ? cache.get(kind) ?? [] : [], loading: false, error: null });
      return;
    }
    const cached = cache.get(kind);
    if (cached) {
      setState({ options: cached, loading: false, error: null });
      return;
    }

    let alive = true;
    setState({ options: [], loading: true, error: null });
    loadDirectory(kind)
      .then((options) => {
        cache.set(kind, options);
        if (alive) {
          setState({ options, loading: false, error: null });
        }
      })
      .catch((reason: unknown) => {
        if (alive) {
          setState({
            options: [],
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
