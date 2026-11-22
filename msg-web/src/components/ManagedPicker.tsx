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
  kind: DirectoryKind;
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
  const [page, setPage] = useState(1);
  const [rect, setRect] = useState<{ left: number; top: number; width: number; height: number } | null>(null);

  const current = value === undefined || value === null ? '' : String(value);
  const selected = useMemo(() => bundle.entries.find((entry) => entry.id === current), [bundle.entries, current]);

  const allowedGroups = useMemo(() => {
    if (!treeSelected) return null;
    return new Set(subtreeIds(bundle.tree, treeSelected));
  }, [bundle.tree, treeSelected]);

  const filtered = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    return bundle.entries.filter((entry) => {
      if (allowedGroups) {
        const groups = bundle.groupsByEntry.get(entry.id) ?? [];
        if (!groups.some((id) => allowedGroups.has(id))) return false;
      }
      if (!keyword) return true;
      return [entry.label, entry.name, entry.detail, entry.treeCode, entry.code, entry.groupLabel, entry.id]
        .filter(Boolean)
        .some((text) => String(text).toLowerCase().includes(keyword));
    });
  }, [allowedGroups, bundle, query]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const currentPage = Math.min(page, pageCount);
  const pageRows = filtered.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  const place = () => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const box = trigger.getBoundingClientRect();
    const width = Math.min(Math.max(box.width, PANEL_WIDTH), window.innerWidth - 16);
    const below = window.innerHeight - box.bottom - 12;
    const above = box.top - 12;
    const flip = below < 300 && above > below;
    const height = Math.min(PANEL_HEIGHT, Math.max(260, flip ? above : below));
    setRect({
      left: Math.min(Math.max(8, box.left), window.innerWidth - width - 8),
      top: flip ? Math.max(8, box.top - height - 6) : box.bottom + 6,
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
    for (const node of bundle.tree) {
      const code = node.treeCode;
      if (!code) continue;
      for (let length = code.length - 3; length >= 3; length -= 3) codes.add(code.slice(0, length));
    }
    return codes;
  }, [bundle.tree]);

  const treeVisible = useMemo(() => {
    const keyword = treeQuery.trim().toLowerCase();
    if (!keyword) return bundle.tree;
    const keep = new Set<string>();
    for (const node of bundle.tree) {
      if (!node.label.toLowerCase().includes(keyword)) continue;
      keep.add(node.id);
      const code = node.treeCode;
      if (!code) continue;
      for (let length = code.length - 3; length >= 3; length -= 3) {
        const prefix = code.slice(0, length);
        const parent = bundle.tree.find((candidate) => candidate.treeCode === prefix);
        if (parent) keep.add(parent.id);
      }
    }
    return bundle.tree.filter((node) => keep.has(node.id));
  }, [bundle.tree, treeQuery]);

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
                      placeholder="搜索名称、工号或 ID"
                    />
                  </div>

                  <div className="msgp-table-wrap">
                    <table className="msgp-table">
                      <thead>
                        <tr>
                          <th>{kind === 'org' ? '组织' : kind === 'role' ? '角色' : '用户'}</th>
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
            document.body,
          )
        : null}
    </>
  );
}
