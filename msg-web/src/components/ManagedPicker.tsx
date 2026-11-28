/**
 * # 用户管理式选择面板
 *
 * 与 adm 用户管理页同形态：左侧组织/角色分组树，右侧带搜索、表头与分页的表格，
 * 整行点选。面板通过 portal 锚定在字段下方，不新开独立窗口。
 */

import { useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  CheckOutlined,
  CloseOutlined,
  DownOutlined,
  RightOutlined,
  SearchOutlined,
} from '@ant-design/icons';
import type { DirectoryBundle, DirectoryEntry, DirectoryKind } from '../api/directory';

interface ManagedPickerProps {
  value?: string | null;
  onChange?: (value: string) => void;
  bundle: DirectoryBundle;
  /** 目录种类；业务外键（消息/渠道/投递）没有固定种类时不传。 */
  kind?: DirectoryKind;
  /** 右表第一列表头，默认按 kind 推导。 */
  nameLabel?: string;
  loading?: boolean;
  disabled?: boolean;
  allowClear?: boolean;
  placeholder?: string;
  title?: string;
  emptyText?: string;
  className?: string;
}

const PANEL_WIDTH = 640;
const PANEL_HEIGHT = 430;
const PAGE_SIZE = 8;

/**
 * 选择面板的 portal 宿主：body 上的一个隐形容器（`display: contents`）。
 *
 * 面板是 `position: fixed` + 视口坐标（`getBoundingClientRect`）。uiam 给
 * `.federation-app` 加了 `transform`，窗口内任何 `fixed` 后代的包含块都会变成
 * 那个窗口，于是「按视口算坐标、按窗口画位置」，面板就整体偏出字段一截。
 * 挂到 body 上，`fixed` 才真正相对视口，坐标与绘制同一个基准。
 * 暗色靠 `data-theme` 复制过去（msg 的暗色令牌就是 `[data-theme='dark']`）。
 */
let portalRoot: HTMLDivElement | null = null;

function portalHost(): HTMLElement {
  if (!portalRoot) {
    portalRoot = document.createElement('div');
    portalRoot.className = 'msg-overlay-portal';
  }
  const theme = document.querySelector<HTMLElement>('.msg-federated')?.getAttribute('data-theme');
  if (theme) portalRoot.setAttribute('data-theme', theme);
  else portalRoot.removeAttribute('data-theme');
  if (portalRoot.parentElement !== document.body) document.body.appendChild(portalRoot);
  return portalRoot;
}

/** treeCode 每层 3 位，根为 0。 */
function depthOf(treeCode?: string): number {
  return treeCode ? Math.max(0, Math.floor(treeCode.length / 3) - 1) : 0;
}

/** 选中节点 + 全部子孙的 id；treeCode 非法时退回只含自身。 */
function subtreeIds(nodes: DirectoryBundle['tree'], rootId: string): string[] {
  const root = nodes.find((node) => node.id === rootId);
  if (!root?.treeCode) return [rootId];
  const prefix = root.treeCode;
  return nodes
    .filter((node) => {
      const code = node.treeCode ?? '';
      const extra = code.length - prefix.length;
      return extra >= 0 && extra % 3 === 0 && code.startsWith(prefix);
    })
    .map((node) => node.id);
}

export function ManagedPicker({
  value,
  onChange,
  bundle,
  kind,
  nameLabel,
  loading = false,
  disabled = false,
  allowClear = true,
  placeholder = '请选择',
  title = '请选择',
  emptyText = '没有匹配的选项',
  className,
}: ManagedPickerProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [treeQuery, setTreeQuery] = useState('');
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());
  const [treeSelected, setTreeSelected] = useState<string | null>(null);
  const [activeRealm, setActiveRealm] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [rect, setRect] = useState<{ left: number; top: number; width: number; height: number } | null>(null);

  const current = value === undefined || value === null ? '' : String(value);
  const selected = useMemo(() => bundle.entries.find((entry) => entry.id === current), [bundle.entries, current]);

  /** 领域 tab 收窄后的左树；未选领域时保持全量。 */
  const realmTree = useMemo(() => {
    if (!activeRealm || !bundle.realmsByTree) return bundle.tree;
    return bundle.tree.filter((node) => bundle.realmsByTree?.get(node.id)?.includes(activeRealm));
  }, [activeRealm, bundle.realmsByTree, bundle.tree]);

  const allowedGroups = useMemo(() => {
    if (!treeSelected) return null;
    return new Set(subtreeIds(realmTree, treeSelected));
  }, [realmTree, treeSelected]);

  const filtered = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    return bundle.entries.filter((entry) => {
      if (activeRealm && !bundle.realmsByEntry?.get(entry.id)?.includes(activeRealm)) return false;
      if (allowedGroups) {
        const groups = bundle.groupsByEntry.get(entry.id) ?? [];
        if (!groups.some((id) => allowedGroups.has(id))) return false;
      }
      if (!keyword) return true;
      return [entry.label, entry.name, entry.detail, entry.treeCode, entry.code, entry.groupLabel, entry.id]
        .filter(Boolean)
        .some((text) => String(text).toLowerCase().includes(keyword));
    });
  }, [activeRealm, allowedGroups, bundle, query]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const currentPage = Math.min(page, pageCount);
  const pageRows = filtered.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  /**
   * 面板摆放：下方不够就翻到上方，范围按**应用根节点**收敛。
   * 联邦窗口里视口是整块桌面，按视口算会把面板推到窗口外，看起来像下拉框错位。
   */
  const place = () => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const box = trigger.getBoundingClientRect();
    const root = document.querySelector<HTMLElement>('.msg-federated:not(.msg-overlay-portal)');
    const area = root?.getBoundingClientRect();
    const minX = (area ? area.left : 0) + 8;
    const maxX = (area ? area.right : window.innerWidth) - 8;
    const minY = (area ? area.top : 0) + 8;
    const maxY = (area ? area.bottom : window.innerHeight) - 8;
    const width = Math.max(240, Math.min(Math.max(box.width, PANEL_WIDTH), maxX - minX));
    const below = maxY - box.bottom - 6;
    const above = box.top - minY - 6;
    const flip = below < 300 && above > below;
    const height = Math.min(PANEL_HEIGHT, Math.max(240, flip ? above : below));
    setRect({
      left: Math.max(minX, Math.min(box.left, maxX - width)),
      top: flip ? Math.max(minY, box.top - height - 6) : Math.min(box.bottom + 6, maxY - height),
      width,
      height,
    });
  };

  const openPanel = () => {
    if (disabled) return;
    place();
    setQuery('');
    setTreeQuery('');
    setPage(1);
    setOpen(true);
  };
  const closePanel = (focusTrigger = false) => {
    setOpen(false);
    if (focusTrigger) triggerRef.current?.focus();
  };
  const choose = (entry: DirectoryEntry) => {
    // 停用项可见但不可选，避免把消息目标指向已停用的账号/组织。
    if (entry.enabled === false) return;
    onChange?.(entry.id);
    closePanel(true);
  };

  useEffect(() => {
    if (!open) return;
    searchRef.current?.focus();
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (panelRef.current?.contains(target) || triggerRef.current?.contains(target)) return;
      closePanel();
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.stopPropagation();
        closePanel(true);
      }
    };
    window.addEventListener('mousedown', onPointerDown);
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('resize', place);
    window.addEventListener('scroll', place, true);
    return () => {
      window.removeEventListener('mousedown', onPointerDown);
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('resize', place);
      window.removeEventListener('scroll', place, true);
    };
    // place 只依赖 ref 与视口尺寸
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  useEffect(() => {
    setPage(1);
  }, [query, treeSelected]);

  /** 有子节点的编码集合：用于显示折叠箭头。 */
  const parentCodes = useMemo(() => {
    const codes = new Set<string>();
    for (const node of realmTree) {
      const code = node.treeCode;
      if (!code) continue;
      for (let length = code.length - 3; length >= 3; length -= 3) codes.add(code.slice(0, length));
    }
    return codes;
  }, [realmTree]);

  const treeVisible = useMemo(() => {
    const keyword = treeQuery.trim().toLowerCase();
    if (!keyword) return realmTree;
    const keep = new Set<string>();
    for (const node of realmTree) {
      if (!node.label.toLowerCase().includes(keyword)) continue;
      keep.add(node.id);
      const code = node.treeCode;
      if (!code) continue;
      for (let length = code.length - 3; length >= 3; length -= 3) {
        const prefix = code.slice(0, length);
        const parent = realmTree.find((candidate) => candidate.treeCode === prefix);
        if (parent) keep.add(parent.id);
      }
    }
    return realmTree.filter((node) => keep.has(node.id));
  }, [realmTree, treeQuery]);

  const isHidden = (treeCode?: string) => {
    if (!treeCode || treeQuery.trim()) return false;
    for (let length = treeCode.length - 3; length >= 3; length -= 3) {
      if (collapsed.has(treeCode.slice(0, length))) return true;
    }
    return false;
  };

  const toggleNode = (code: string) =>
    setCollapsed((previous) => {
      const next = new Set(previous);
      if (next.has(code)) next.delete(code);
      else next.add(code);
      return next;
    });

  const displayText = selected ? selected.label : current ? `#${current}（未在列表中）` : placeholder;

  const nameCell = (entry: DirectoryEntry) => (
    <div className="msgp-cell-main">
      <span className="msgp-cell-title">{entry.label}</span>
      {kind === 'role' && entry.groupLabel ? (
        <span className="msgp-cell-sub">{entry.groupLabel}</span>
      ) : kind === 'org' && entry.treeCode ? (
        <span className="msgp-cell-sub">{entry.treeCode}</span>
      ) : kind === undefined && entry.detail ? (
        <span className="msgp-cell-sub">{entry.detail}</span>
      ) : null}
    </div>
  );

  const idCell = (entry: DirectoryEntry) => (
    <span className="msgp-mono">{kind === 'org' ? entry.treeCode ?? entry.detail ?? '--' : entry.detail ?? '--'}</span>
  );

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        role="combobox"
        aria-expanded={open}
        aria-haspopup="dialog"
        disabled={disabled || loading}
        onClick={() => (open ? closePanel() : openPanel())}
        className={`msgp-trigger${className ? ` ${className}` : ''}`}
      >
        <span className={`msgp-trigger-text${selected ? '' : ' is-placeholder'}`}>
          {loading ? '加载中…' : displayText}
        </span>
        {current && allowClear && !disabled && !loading ? (
          <span
            role="button"
            tabIndex={-1}
            aria-label="清除"
            className="msgp-clear"
            onClick={(event) => {
              event.stopPropagation();
              onChange?.('');
            }}
          >
            <CloseOutlined />
          </span>
        ) : null}
        <DownOutlined className={`msgp-arrow${open ? ' is-open' : ''}`} />
      </button>

      {open && rect
        ? createPortal(
            <div
              ref={panelRef}
              role="dialog"
              aria-label={title}
              className="msgp-panel"
              style={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height }}
            >
              <div className="msgp-head">
                <span className="msgp-head-title">{title}</span>
                <span className="msgp-head-count">
                  {loading ? '加载中…' : `${filtered.length} / ${bundle.entries.length}`}
                </span>
              </div>

              {bundle.realms && bundle.realms.length > 1 ? (
                <div className="msgp-tabs">
                  <button
                    type="button"
                    className={`msgp-tab${activeRealm === null ? ' is-active' : ''}`}
                    onClick={() => {
                      setActiveRealm(null);
                      setTreeSelected(null);
                    }}
                  >
                    全部领域
                  </button>
                  {bundle.realms.map((realm) => (
                    <button
                      key={realm.id}
                      type="button"
                      className={`msgp-tab${activeRealm === realm.id ? ' is-active' : ''}`}
                      onClick={() => {
                        setActiveRealm(realm.id);
                        setTreeSelected(null);
                      }}
                    >
                      {realm.label}
                    </button>
                  ))}
                </div>
              ) : null}

              <div className="msgp-body">
                {bundle.tree.length > 0 ? (
                  <aside className="msgp-tree">
                    <div className="msgp-tree-search">
                      <SearchOutlined />
                      <input
                        value={treeQuery}
                        onChange={(event) => setTreeQuery(event.target.value)}
                        placeholder={kind === 'role' ? '搜索角色分组' : '搜索组织'}
                      />
                    </div>
                    <div className="msgp-tree-list">
                      <button
                        type="button"
                        className={`msgp-tree-all${treeSelected ? '' : ' is-active'}`}
                        onClick={() => setTreeSelected(null)}
                      >
                        全部
                        <span>不限</span>
                      </button>
                      {treeVisible.map((node) => {
                        if (isHidden(node.treeCode)) return null;
                        const code = node.treeCode;
                        const isParent = code ? parentCodes.has(code) : false;
                        const expanded = !(code && collapsed.has(code));
                        const active = treeSelected === node.id;
                        return (
                          <div
                            key={node.id}
                            className="msgp-tree-row"
                            style={{ paddingLeft: 2 + depthOf(node.treeCode) * 12 }}
                          >
                            {isParent ? (
                              <button
                                type="button"
                                className="msgp-tree-toggle"
                                aria-label={expanded ? '收起子分组' : '展开子分组'}
                                aria-expanded={expanded}
                                onClick={() => toggleNode(code!)}
                              >
                                {expanded ? <DownOutlined /> : <RightOutlined />}
                              </button>
                            ) : (
                              <span className="msgp-tree-toggle-space" />
                            )}
                            <button
                              type="button"
                              title={node.label}
                              className={`msgp-tree-item${active ? ' is-active' : ''}`}
                              onClick={() => setTreeSelected(node.id)}
                            >
                              {node.label}
                            </button>
                          </div>
                        );
                      })}
                    </div>
                  </aside>
                ) : null}

                <div className="msgp-main">
                  <div className="msgp-search">
                    <SearchOutlined />
                    <input
                      ref={searchRef}
                      value={query}
                      onChange={(event) => setQuery(event.target.value)}
                      placeholder="搜索姓名、工号或登录名"
                    />
                  </div>

                  <div className="msgp-table-wrap">
                    <table className="msgp-table">
                      <thead>
                        <tr>
                          <th>
                            {nameLabel ?? (kind === 'org' ? '组织' : kind === 'role' ? '角色' : kind === 'user' ? '用户' : '名称')}
                          </th>
                          <th className="msgp-col-id">{kind === 'org' ? '编码' : '标识'}</th>
                          <th className="msgp-col-status">状态</th>
                          <th className="msgp-col-check" />
                        </tr>
                      </thead>
                      <tbody>
                        {pageRows.map((entry) => {
                          const active = entry.id === current;
                          return (
                            <tr
                              key={entry.id}
                              className={active ? 'is-active' : undefined}
                              onClick={() => choose(entry)}
                            >
                              <td>{nameCell(entry)}</td>
                              <td className="msgp-col-id">{idCell(entry)}</td>
                              <td className="msgp-col-status">
                                <span className={`msgp-status${entry.enabled === false ? ' is-off' : ''}`}>
                                  {entry.enabled === false ? '停用' : '启用'}
                                </span>
                              </td>
                              <td className="msgp-col-check">
                                {active ? <CheckOutlined className="msgp-check" /> : null}
                              </td>
                            </tr>
                          );
                        })}
                      </tbody>
                    </table>
                    {pageRows.length === 0 ? (
                      <p className="msgp-empty">{loading ? '加载中…' : emptyText}</p>
                    ) : null}
                  </div>

                  <div className="msgp-foot">
                    <span>
                      共 {filtered.length} 条 · 第 {currentPage} / {pageCount} 页
                    </span>
                    <div>
                      <button
                        type="button"
                        disabled={currentPage <= 1}
                        onClick={() => setPage((previous) => Math.max(1, previous - 1))}
                      >
                        上一页
                      </button>
                      <button
                        type="button"
                        disabled={currentPage >= pageCount}
                        onClick={() => setPage((previous) => Math.min(pageCount, previous + 1))}
                      >
                        下一页
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>,
            portalHost(),
          )
        : null}
    </>
  );
}
