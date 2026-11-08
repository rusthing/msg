/**
 * # 通用模板编辑器
 *
 * 把一个完整模板拆成「模板类型」+「正文」两段来编辑，保存时再合并回
 * `@engine: <name>\n<body>` 的格式。支持语法高亮与保留占位符的 JSON 格式化，
 * 不依赖任何重型编辑器库。
 *
 * 组件是受控的（`value` / `onChange`），可以直接放进 `Form.Item`；只处理模板
 * 类型与正文，不内置启用/禁用、允许模拟等状态开关（这类状态在外层列表操作）。
 *
 * 本文件在 alarm-web 与 msg-web 各有一份副本，修改时请同步两边；两份唯一允许
 * 不同的是下面的等宽字体类名。
 */

import { Button, Tag, Typography } from 'antd';
import OptionButtons from './OptionButtons';
import { useMemo, useRef, useState } from 'react';
import type { CSSProperties, ChangeEvent } from 'react';
import {
  DEFAULT_ENGINE,
  DEFAULT_TEMPLATE_TYPES,
  formatTemplateJson,
  looksLikeJson,
  mergeTemplate,
  resolveTypeOptions,
  splitTemplate,
  tokenizeTemplate,
} from '../utils/template-editor';
import type { TemplateTokenKind, TemplateTypeOption } from '../utils/template-editor';

const MONO_CLASS = 'msg-json-editor';
const LINE_HEIGHT = 20;
const PADDING = 8;

const TOKEN_STYLE: Record<TemplateTokenKind, CSSProperties> = {
  text: {},
  variable: { color: '#7c3aed', background: 'rgba(124, 58, 237, 0.10)', borderRadius: 2 },
  filter: { color: '#0f766e' },
  tag: { color: '#b45309', background: 'rgba(180, 83, 9, 0.10)', borderRadius: 2 },
  comment: { color: '#8c8c8c', fontStyle: 'italic' },
  'json-key': { color: '#1d4ed8' },
  'json-string': { color: '#047857' },
  'json-number': { color: '#b45309' },
  'json-literal': { color: '#c2410c' },
  'json-punct': { color: '#6b7280' },
};

export interface TemplateEditorProps {
  /** 传给内部 textarea，供 Form.Item 的 label 关联 */
  id?: string;
  /** 完整模板（含首行声明） */
  value?: string;
  onChange?: (value: string) => void;
  /** 类型列表可扩展；默认 robotech / minijinja */
  types?: TemplateTypeOption[];
  readOnly?: boolean;
  /** 正文区行数 */
  rows?: number;
  placeholder?: string;
  /** 完整模板的最大长度（含首行）；正文输入上限会自动扣掉首行 */
  maxLength?: number;
  showCount?: boolean;
  /** auto：无首行的 robotech 模板保持无首行；always：总是写首行 */
  headerMode?: 'auto' | 'always';
  /** 是否展示 JSON 格式化按钮（默认展示，仅正文看起来像 JSON 时出现） */
  allowJsonFormat?: boolean;
  /** 已知这是 JSON 模板：即使正文为空也展示格式化按钮 */
  isJsonTemplate?: boolean;
  /** 是否展示当前类型的说明 */
  showTypeHelp?: boolean;
}

export default function TemplateEditor({
  id,
  value,
  onChange,
  types,
  readOnly = false,
  rows = 4,
  placeholder,
  maxLength,
  showCount = false,
  headerMode = 'auto',
  allowJsonFormat = true,
  isJsonTemplate = false,
  showTypeHelp = true,
}: TemplateEditorProps) {
  const parts = splitTemplate(value);
  const options = useMemo(() => resolveTypeOptions(parts.engine, types), [parts.engine, types]);
  const current = options.find((item) => item.value === parts.engine);
  const tokens = useMemo(
    () => tokenizeTemplate(parts.body, parts.engine),
    [parts.body, parts.engine],
  );
  const [focused, setFocused] = useState(false);
  const [formatError, setFormatError] = useState<string | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const preRef = useRef<HTMLPreElement>(null);

  const emitHeader =
    headerMode === 'always' || parts.hadHeader || parts.engine !== DEFAULT_ENGINE;
  const headerLength = emitHeader ? `@engine: ${parts.engine}${parts.separator}`.length : 0;
  const totalLength = headerLength + parts.body.length;
  const bodyMaxLength = maxLength === undefined ? undefined : Math.max(0, maxLength - headerLength);
  const mergeOptions = { hadHeader: parts.hadHeader, separator: parts.separator, headerMode };
  const jsonLike = looksLikeJson(parts.body);
  const showJsonFormat = allowJsonFormat && (isJsonTemplate || jsonLike);
  const knownType = (types ?? DEFAULT_TEMPLATE_TYPES).some((item) => item.value === parts.engine);

  const emit = (engine: string, body: string) => {
    setFormatError(null);
    onChange?.(mergeTemplate(engine, body, mergeOptions));
  };

  const handleFormat = () => {
    const result = formatTemplateJson(parts.body, parts.engine);
    if (!result.ok) {
      setFormatError(result.error);
      return;
    }
    emit(parts.engine, result.text);
  };

  /** 高亮层与输入层共用滚动位置 */
  const syncScroll = () => {
    const textarea = textareaRef.current;
    const pre = preRef.current;
    if (!textarea || !pre) {
      return;
    }
    pre.scrollTop = textarea.scrollTop;
    pre.scrollLeft = textarea.scrollLeft;
  };

  const textareaHeight = rows * LINE_HEIGHT + PADDING * 2;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6, width: '100%' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
        <Typography.Text strong style={{ fontSize: 12 }}>
          模板类型
        </Typography.Text>
        <OptionButtons
          ariaLabel="模板类型"
          value={parts.engine}
          disabled={readOnly}
          onChange={(next) => {
            if (next !== undefined) emit(next, parts.body);
          }}
          options={options.map((item) => ({ value: item.value, label: item.label }))}
        />
        <Tag
          className={MONO_CLASS}
          color={parts.engine === 'minijinja' ? 'blue' : 'default'}
          style={{ marginInlineEnd: 0 }}
        >
          @engine: {parts.engine}
        </Tag>
        {!knownType && <Tag color="warning">未登记引擎</Tag>}
        {readOnly && <Tag>只读</Tag>}
      </div>

      {showTypeHelp && current && (
        <div style={{ fontSize: 12, lineHeight: 1.6, color: 'rgba(0, 0, 0, 0.65)' }}>
          <div>{current.description}</div>
          {current.syntax && (
            <div style={{ marginTop: 2 }}>
              <Typography.Text code style={{ fontSize: 12 }}>
                {current.syntax}
              </Typography.Text>
            </div>
          )}
        </div>
      )}

      <div
        style={{
          position: 'relative',
          border: `1px solid ${focused ? '#1677ff' : '#d9d9d9'}`,
          borderRadius: 6,
          background: readOnly ? '#fafafa' : '#fff',
          boxShadow: focused ? '0 0 0 2px rgba(22, 119, 255, 0.10)' : 'none',
          transition: 'border-color 0.2s, box-shadow 0.2s',
        }}
      >
        <pre
          ref={preRef}
          aria-hidden
          style={{
            position: 'absolute',
            inset: 0,
            margin: 0,
            padding: PADDING,
            lineHeight: `${LINE_HEIGHT}px`,
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            overflow: 'hidden',
            pointerEvents: 'none',
            color: '#1f1f1f',
            tabSize: 2,
          }}
        >
          {tokens.map((token, index) => (
            <span key={index} style={TOKEN_STYLE[token.kind]}>
              {token.text}
            </span>
          ))}
        </pre>
        <textarea
          id={id}
          ref={textareaRef}
          value={parts.body}
          readOnly={readOnly}
          spellCheck={false}
          placeholder={placeholder}
          aria-label="模板正文"
          onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
            emit(parts.engine, event.target.value)
          }
          onScroll={syncScroll}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          maxLength={bodyMaxLength}
          className={MONO_CLASS}
          style={{
            position: 'relative',
            zIndex: 1,
            display: 'block',
            width: '100%',
            height: textareaHeight,
            margin: 0,
            padding: PADDING,
            lineHeight: `${LINE_HEIGHT}px`,
            border: 'none',
            outline: 'none',
            resize: 'vertical',
            background: 'transparent',
            color: 'transparent',
            WebkitTextFillColor: 'transparent',
            caretColor: '#1f1f1f',
            whiteSpace: 'pre-wrap',
            wordBreak: 'break-word',
            overflow: 'auto',
            tabSize: 2,
          }}
        />
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
        {showJsonFormat && (
          <Button size="small" disabled={readOnly} onClick={handleFormat}>
            格式化 JSON
          </Button>
        )}
        {formatError ? (
          <Typography.Text type="danger" style={{ fontSize: 12, lineHeight: 1.5 }}>
            {formatError}
          </Typography.Text>
        ) : (
          (isJsonTemplate || jsonLike) &&
          !readOnly && (
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              占位符会原样保留
            </Typography.Text>
          )
        )}
        <span style={{ marginLeft: 'auto' }} />
        {showCount && maxLength !== undefined && (
          <Typography.Text
            type={totalLength > maxLength ? 'danger' : 'secondary'}
            style={{ fontSize: 12 }}
          >
            {totalLength} / {maxLength}
          </Typography.Text>
        )}
      </div>
    </div>
  );
}
