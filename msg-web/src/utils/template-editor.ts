/**
 * # 模板编辑器纯逻辑
 *
 * 模板字符串的格式由 `robotech-template` 定义：首行可写 `@engine: <name>`，
 * 之后的全部内容交给该引擎渲染；没有首行声明时默认 `robotech`。
 *
 * 本文件在 alarm-web 与 msg-web 各有一份副本（两个前端是相互独立的 pnpm
 * 工程，没有共享包），修改时请同步两边；唯一允许不同的是组件里的等宽字体
 * 类名。这里不依赖 React，方便单独测试。
 */

/** 编辑器可选的模板类型（与 `robotech-template/src/engine.rs` 的内置引擎一致） */
export interface TemplateTypeOption {
  /** 引擎名，写入首行 `@engine: <value>` */
  value: string;
  /** 下拉框显示名 */
  label: string;
  /** 这个类型怎么渲染、适合什么场景 */
  description: string;
  /** 语法摘要 */
  syntax: string;
}

export const DEFAULT_TEMPLATE_TYPES: TemplateTypeOption[] = [
  {
    value: 'robotech',
    label: 'robotech（默认）',
    description:
      '项目内置默认引擎。{{var}}（或 ${var}）直接代入变量，{{var:filter(arg)|filter}} 按管道从左到右加工；内置 json/time/since/upper/lower/trim/default/truncate/fixed/mask/url 11 个过滤器。变量缺失时保留占位符，json 过滤器输出 null。适合标题、正文和 JSON 报文。',
    syntax: '{{var}} / ${var} / {{var:upper|truncate(10)}}',
  },
  {
    value: 'minijinja',
    label: 'minijinja',
    description:
      'Jinja2 风格引擎，支持 {% if %}、{% for %}、{% set %} 等控制流与 | 过滤器；未定义变量渲染为空串。适合按级别/状态输出不同文案、遍历人员列表。首行必须保留 @engine: minijinja。',
    syntax: '{{ var }} / {% if ok %}…{% endif %} / {{ var|upper }}',
  },
];

export const DEFAULT_ENGINE = 'robotech';

const HEADER_RE = /^@engine:\s*([^\s]+)\s*$/i;
const SENTINEL_OPEN = '\uE000';
const SENTINEL_CLOSE = '\uE001';

/** 模板拆分结果 */
export interface TemplateParts {
  /** 规范化后的引擎名（小写）；无首行时为 robotech */
  engine: string;
  /** 去掉首行后的正文；无首行时就是原模板 */
  body: string;
  /** 原模板是否带 `@engine:` 首行 */
  hadHeader: boolean;
  /** 首行后的换行符，合并时原样还原（CRLF 模板不会被改成 LF） */
  separator: string;
}

/**
 * # 拆分模板为「引擎 + 正文」
 *
 * 与 `robotech-template/src/header.rs` 的 `parse_engine_header` 保持一致：
 * 只看第一行，允许前后空白，引擎名大小写不敏感；没有声明时默认 robotech，
 * 正文为原模板。
 */
export function splitTemplate(value?: string | null): TemplateParts {
  const text = value ?? '';
  const lineEnd = text.indexOf('\n');
  if (lineEnd === -1) {
    const matched = HEADER_RE.exec(text.trim());
    if (matched) {
      return { engine: matched[1].toLowerCase(), body: '', hadHeader: true, separator: '\n' };
    }
    return { engine: DEFAULT_ENGINE, body: text, hadHeader: false, separator: '\n' };
  }

  const firstLine = text.slice(0, lineEnd).trim();
  const matched = HEADER_RE.exec(firstLine);
  const separator = text[lineEnd - 1] === '\r' ? '\r\n' : '\n';
  if (!matched) {
    return { engine: DEFAULT_ENGINE, body: text, hadHeader: false, separator };
  }
  return {
    engine: matched[1].toLowerCase(),
    body: text.slice(lineEnd + 1),
    hadHeader: true,
    separator,
  };
}

export interface MergeTemplateOptions {
  /** 原模板是否带首行 */
  hadHeader: boolean;
  /** 原模板首行后的换行符 */
  separator?: string;
  /**
   * auto（默认）：默认引擎且原本没有首行时保持无首行，保证存量模板字节不变；
   * always：总是写出首行。
   */
  headerMode?: 'auto' | 'always';
}

/**
 * # 合并「引擎 + 正文」为完整模板
 *
 * 无首行的 robotech 模板保持无首行：后端默认就是 robotech，加了反而会占用
 * 标题模板的 150 字符上限；切换成非默认引擎时必须写出首行。
 */
export function mergeTemplate(
  engine: string,
  body: string,
  options: MergeTemplateOptions,
): string {
  const normalized = engine.trim().toLowerCase() || DEFAULT_ENGINE;
  const emitHeader =
    options.headerMode === 'always' || options.hadHeader || normalized !== DEFAULT_ENGINE;
  if (!emitHeader) {
    return body;
  }
  return `@engine: ${normalized}${options.separator ?? '\n'}${body}`;
}

/** 把当前模板声明的未知引擎补进类型下拉，避免保存时丢掉它 */
export function resolveTypeOptions(engine: string, types?: TemplateTypeOption[]): TemplateTypeOption[] {
  const base = types && types.length ? types : DEFAULT_TEMPLATE_TYPES;
  if (base.some((item) => item.value === engine)) {
    return base;
  }
  return [
    ...base,
    {
      value: engine,
      label: `${engine}（未登记）`,
      description: '现有模板首行声明的引擎不在前端类型列表中；保存时会原样保留这个引擎名。',
      syntax: `@engine: ${engine}`,
    },
  ];
}

// ─── 语法高亮 ──────────────────────────────────────────────────────────────────

export type TemplateTokenKind =
  | 'text'
  | 'variable'
  | 'filter'
  | 'tag'
  | 'comment'
  | 'json-key'
  | 'json-string'
  | 'json-number'
  | 'json-literal'
  | 'json-punct';

export interface TemplateToken {
  kind: TemplateTokenKind;
  text: string;
}

interface PlaceholderBounds {
  start: number;
  end: number;
  open: string;
  close: string;
  inner: string;
  kind: 'variable' | 'tag' | 'comment';
}

/** 识别从 index 开始的占位符 / 控制标签；未闭合时按普通文本处理 */
function placeholderAt(text: string, index: number, engine: string): PlaceholderBounds | null {
  if (text.startsWith('{{', index)) {
    const close = text.indexOf('}}', index + 2);
    if (close === -1) {
      return null;
    }
    return { start: index, end: close + 2, open: '{{', close: '}}', inner: text.slice(index + 2, close), kind: 'variable' };
  }

  if (engine === 'minijinja') {
    if (text.startsWith('{%', index)) {
      const close = text.indexOf('%}', index + 2);
      if (close === -1) {
        return null;
      }
      return { start: index, end: close + 2, open: '{%', close: '%}', inner: text.slice(index + 2, close), kind: 'tag' };
    }
    if (text.startsWith('{#', index)) {
      const close = text.indexOf('#}', index + 2);
      if (close === -1) {
        return null;
      }
      return { start: index, end: close + 2, open: '{#', close: '#}', inner: text.slice(index + 2, close), kind: 'comment' };
    }
    return null;
  }

  if (text.startsWith('${', index)) {
    const close = text.indexOf('}', index + 2);
    if (close === -1) {
      return null;
    }
    return { start: index, end: close + 1, open: '${', close: '}', inner: text.slice(index + 2, close), kind: 'variable' };
  }
  return null;
}

type PushToken = (kind: TemplateTokenKind, text: string) => void;

/** 占位符再拆成「变量 / 过滤器」，让过滤器链有明显区分 */
function pushPlaceholder(bounds: PlaceholderBounds, engine: string, push: PushToken): void {
  if (bounds.kind === 'tag' || bounds.kind === 'comment') {
    push(bounds.kind, bounds.open + bounds.inner + bounds.close);
    return;
  }

  // robotech 用冒号接过滤器链，minijinja 用管道
  const separator = engine === 'minijinja' ? '|' : ':';
  const position = bounds.inner.indexOf(separator);
  if (position === -1) {
    push('variable', bounds.open + bounds.inner + bounds.close);
    return;
  }
  push('variable', bounds.open + bounds.inner.slice(0, position + 1));
  push('filter', bounds.inner.slice(position + 1));
  push('variable', bounds.close);
}

/** 去掉正文开头可能出现的 minijinja 控制标签，用于判断是不是 JSON 正文 */
const LEADING_TAG_RE = /^\s*(?:\{%[\s\S]*?%\}|\{#[\s\S]*?#\})/;

function stripLeadingBlockTags(text: string): string {
  let rest = text;
  let match = LEADING_TAG_RE.exec(rest);
  while (match) {
    rest = rest.slice(match[0].length);
    match = LEADING_TAG_RE.exec(rest);
  }
  return rest;
}

/** 正文是否以 JSON 结构开头（`{` / `[`，允许前面有 minijinja 控制标签） */
export function looksLikeJson(body: string): boolean {
  const head = stripLeadingBlockTags(body).trimStart();
  return head.startsWith('{') || head.startsWith('[');
}

/**
 * # 把模板正文切成带类型的 token
 *
 * 普通文本模板只挑出占位符；JSON 正文额外按 JSON 词法着色。token 拼回去必须
 * 与原文逐字符一致——高亮只是加样式，不改写用户输入。
 */
export function tokenizeTemplate(body: string, engine: string): TemplateToken[] {
  const jsonMode = looksLikeJson(body);
  const tokens: TemplateToken[] = [];
  const push: PushToken = (kind, text) => {
    if (text) {
      tokens.push({ kind, text });
    }
  };

  let index = 0;
  let inString = false;
  let stringStart = -1;

  while (index < body.length) {
    const placeholder = placeholderAt(body, index, engine);
    if (placeholder) {
      pushPlaceholder(placeholder, engine, push);
      index = placeholder.end;
      continue;
    }

    if (!jsonMode) {
      let next = index;
      while (next < body.length && !placeholderAt(body, next, engine)) {
        next += 1;
      }
      push('text', body.slice(index, next));
      index = next;
      continue;
    }

    const char = body[index];

    if (inString) {
      if (char === '\\') {
        push('json-string', body.slice(index, index + 2));
        index += 2;
        continue;
      }
      if (char === '"') {
        push('json-string', '"');
        inString = false;
        // 字符串结束后紧跟冒号 → 这段字符串是键
        let look = index + 1;
        while (look < body.length && /\s/.test(body[look])) {
          look += 1;
        }
        if (body[look] === ':' && stringStart >= 0) {
          for (let i = stringStart; i < tokens.length; i += 1) {
            if (tokens[i].kind === 'json-string') {
              tokens[i].kind = 'json-key';
            }
          }
        }
        stringStart = -1;
        index += 1;
        continue;
      }

      let next = index;
      while (
        next < body.length &&
        body[next] !== '"' &&
        body[next] !== '\\' &&
        !placeholderAt(body, next, engine)
      ) {
        next += 1;
      }
      push('json-string', body.slice(index, next));
      index = next;
      continue;
    }

    if (char === '"') {
      inString = true;
      stringStart = tokens.length;
      push('json-string', '"');
      index += 1;
      continue;
    }
    if ('{}[]:,'.includes(char)) {
      push('json-punct', char);
      index += 1;
      continue;
    }

    const rest = body.slice(index);
    const number = /^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/.exec(rest);
    if (number) {
      push('json-number', number[0]);
      index += number[0].length;
      continue;
    }
    const literal = /^(?:true|false|null)\b/.exec(rest);
    if (literal) {
      push('json-literal', literal[0]);
      index += literal[0].length;
      continue;
    }

    push('text', char);
    index += 1;
  }

  return tokens;
}

// ─── JSON 格式化 ────────────────────────────────────────────────────────────

interface RestoreItem {
  sentinel: string;
  value: string;
  /** 占位符原本在值位置：掩码时加过引号，还原时要去掉 */
  quoted: boolean;
}

function maskPlaceholders(text: string, engine: string): { text: string; restore: RestoreItem[] } {
  let masked = '';
  const restore: RestoreItem[] = [];
  let inString = false;
  let index = 0;

  while (index < text.length) {
    const placeholder = placeholderAt(text, index, engine);
    if (placeholder) {
      const sentinel = `${SENTINEL_OPEN}${restore.length}${SENTINEL_CLOSE}`;
      const quoted = !inString;
      restore.push({ sentinel, value: text.slice(placeholder.start, placeholder.end), quoted });
      masked += quoted ? `"${sentinel}"` : sentinel;
      index = placeholder.end;
      continue;
    }

    const char = text[index];
    if (inString) {
      if (char === '\\') {
        masked += text.slice(index, index + 2);
        index += 2;
        continue;
      }
      if (char === '"') {
        inString = false;
      }
      masked += char;
      index += 1;
      continue;
    }
    if (char === '"') {
      inString = true;
    }
    masked += char;
    index += 1;
  }

  return { text: masked, restore };
}

function unmaskPlaceholders(text: string, restore: RestoreItem[]): string {
  let result = text;
  for (const item of restore) {
    if (item.quoted) {
      result = result.split(`"${item.sentinel}"`).join(item.value);
    }
    result = result.split(item.sentinel).join(item.value);
  }
  return result;
}

export type TemplateFormatResult =
  | { ok: true; text: string }
  | { ok: false; error: string };

/**
 * # 格式化 JSON 正文，同时保留模板占位符
 *
 * 先把占位符掩码成不会参与 JSON 解析的哨兵（值位置的占位符连引号一起掩码），
 * 解析、缩进后再还原。任何一步失败都只返回错误，不改动正文。
 */
export function formatTemplateJson(body: string, engine: string): TemplateFormatResult {
  if (!body.trim()) {
    return { ok: false, error: '正文为空，没有可格式化的内容' };
  }

  const masked = maskPlaceholders(body, engine);
  let parsed: unknown;
  try {
    parsed = JSON.parse(masked.text);
  } catch (reason) {
    const detail = reason instanceof Error ? reason.message : String(reason);
    return { ok: false, error: `JSON 格式不正确：${detail}` };
  }

  const pretty = JSON.stringify(parsed, null, 2);
  const restored = unmaskPlaceholders(pretty, masked.restore);
  if (restored.includes(SENTINEL_OPEN)) {
    return { ok: false, error: '占位符还原失败，请检查 {{...}} / {%...%} 是否闭合' };
  }
  return { ok: true, text: restored };
}
