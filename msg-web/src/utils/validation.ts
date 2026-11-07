/**
 * # 表单校验：JSON 与模板
 *
 * 前端只做「提交前能挡住明显错误」这一层，不逐条复刻
 * `robotech-template/src/validate.rs` 的全部诊断。服务端仍是最终裁判。
 *
 * 覆盖三类问题：
 * 1. 普通 JSON 字段（渠道配置、labels、annotations、投递详情）是否可解析；
 * 2. 模板占位符是否闭合、是否残留旧单花括号语法、变量名是否合法；
 * 3. 模板过滤器链（`{{key:trim|truncate(10)}}`）的括号与过滤器名是否合法。
 *
 * 模板有两种形态：
 * - 纯文本模板（标题、内容）：只校验占位符与过滤器链；
 * - JSON 模板（载荷、附加内容）：在占位符按 JSON 值代入后再校验整体必须是合法 JSON。
 */

import { splitTemplate } from './template-editor.ts';

/** 服务端已注册的过滤器，与 `robotech-template/src/filter.rs` 保持一致 */
export const TEMPLATE_FILTERS = [
  'json',
  'time',
  'since',
  'upper',
  'lower',
  'trim',
  'default',
  'truncate',
  'fixed',
  'mask',
  'url',
] as const;

const FILTER_SET = new Set<string>(TEMPLATE_FILTERS);

/** 不接受参数的过滤器（与服务端 `no_arg` 一致；`time` 可选格式，不在列表里） */
const NO_ARG_FILTERS = new Set(['json', 'since', 'upper', 'lower', 'trim', 'url']);

/** 必须带参数的过滤器 */
const REQUIRE_ARG_FILTERS = new Set(['default', 'truncate', 'fixed', 'mask']);

/** `{{key}}` / `{{key:过滤器链}}` 与等价别名 `${key}` */
const PLACEHOLDER_RE = /\{\{(\w+)(?::([^{}]*))?\}\}|\$\{(\w+)(?::([^{}]*))?\}/g;

/** 旧语法 `{key}` / `{key:...}`；`{"k":...}` 这类 JSON 对象不会命中 */
const LEGACY_RE = /^\{\w+(?::[^{}]*)?\}/;

/** minijinja 首行声明（与 `robotech-template/src/header.rs` 一致：只看第一行、忽略大小写） */
function isMiniJinja(text: string): boolean {
  const lineEnd = text.indexOf('\n');
  const firstLine = (lineEnd === -1 ? text : text.slice(0, lineEnd)).trim();
  return firstLine.toLowerCase() === '@engine: minijinja';
}

/** 一次过滤器调用 */
interface FilterCall {
  name: string;
  arg?: string;
}

/** 统一带位置的报错文案 */
function at(index: number): string {
  return `第 ${index + 1} 个字符处`;
}

/**
 * # 校验模板占位符与过滤器链
 *
 * 合法时返回 `null`。空模板按「不填」处理，不算错误。
 */
export function validateTemplate(text: string): string | null {
  if (!text) {
    return null;
  }

  // minijinja 的变量、控制流与 RAW 语法由服务端引擎负责，前端不复刻
  if (isMiniJinja(text)) {
    return null;
  }

  let cursor = 0;
  while (cursor < text.length) {
    if (text.startsWith('{{', cursor)) {
      const close = text.indexOf('}}', cursor + 2);
      if (close === -1) {
        return `${at(cursor)}有未闭合的 “{{”`;
      }
      const body = text.slice(cursor + 2, close);
      const error = validatePlaceholderBody(body, cursor);
      if (error) {
        return error;
      }
      cursor = close + 2;
      continue;
    }

    if (text.startsWith('${', cursor)) {
      const close = text.indexOf('}', cursor + 2);
      if (close === -1) {
        return `${at(cursor)}有未闭合的 “\${”`;
      }
      const body = text.slice(cursor + 2, close);
      const error = validatePlaceholderBody(body, cursor);
      if (error) {
        return error;
      }
      cursor = close + 1;
      continue;
    }

    if (text[cursor] === '{') {
      const legacy = LEGACY_RE.exec(text.slice(cursor));
      if (legacy) {
        return `${at(cursor)}发现旧语法 ${legacy[0]}，请改用 {{...}} 或 \${...}`;
      }
    }

    cursor += 1;
  }

  return null;
}

/** 校验占位符内部：变量名 + 可选过滤器链 */
function validatePlaceholderBody(body: string, start: number): string | null {
  const colon = body.indexOf(':');
  const key = colon === -1 ? body : body.slice(0, colon);
  const spec = colon === -1 ? '' : body.slice(colon + 1);

  if (!/^\w+$/.test(key)) {
    return `${at(start)}占位符变量名只能包含字母、数字与下划线`;
  }
  if (!spec.trim()) {
    return null;
  }

  const parsed = parseFilterChain(spec);
  if (typeof parsed === 'string') {
    return `${at(start)}过滤器链语法错误：${parsed}`;
  }
  for (const call of parsed) {
    if (!FILTER_SET.has(call.name)) {
      return `${at(start)}未知过滤器 “${call.name}”；可用：${TEMPLATE_FILTERS.join('、')}`;
    }
    if (NO_ARG_FILTERS.has(call.name) && call.arg !== undefined) {
      return `${at(start)}过滤器 “${call.name}” 不接受参数`;
    }
    if (REQUIRE_ARG_FILTERS.has(call.name) && call.arg === undefined) {
      return `${at(start)}过滤器 “${call.name}” 需要参数，如 ${call.name}(...)`;
    }
  }
  return null;
}

/** 解析过滤器链；失败时返回原因字符串 */
function parseFilterChain(spec: string): FilterCall[] | string {
  const calls: FilterCall[] = [];
  let cursor = 0;

  while (cursor < spec.length) {
    while (cursor < spec.length && (spec[cursor] === '|' || /\s/.test(spec[cursor]))) {
      cursor += 1;
    }
    if (cursor >= spec.length) {
      break;
    }

    const nameStart = cursor;
    while (cursor < spec.length && /[A-Za-z0-9_-]/.test(spec[cursor])) {
      cursor += 1;
    }
    if (nameStart === cursor) {
      return `${spec.slice(nameStart, nameStart + 8)} 缺少过滤器名`;
    }
    const name = spec.slice(nameStart, cursor);

    while (cursor < spec.length && /\s/.test(spec[cursor])) {
      cursor += 1;
    }

    let arg: string | undefined;
    if (spec[cursor] === '(') {
      const open = cursor;
      const close = spec.indexOf(')', open + 1);
      if (close === -1) {
        return `过滤器 ${name} 的参数缺少右括号`;
      }
      arg = spec.slice(open + 1, close);
      cursor = close + 1;
    }

    calls.push({ name, arg });
  }

  return calls;
}

/**
 * # 校验普通 JSON 文本
 *
 * 合法或空串返回 `null`，否则返回可以直接展示的错误文案。
 */
export function validateJsonText(text: string): string | null {
  if (!text.trim()) {
    return null;
  }
  try {
    JSON.parse(text);
    return null;
  } catch (reason) {
    return `JSON 格式不正确：${reason instanceof Error ? reason.message : String(reason)}`;
  }
}

/**
 * # 校验 JSON 模板
 *
 * 先把占位符替换成 JSON 值再解析：
 * - 带 `json` 过滤器的占位符替换成 `"__value__"`（字符串变量自带引号）；
 * - 其余占位符替换成 `null`（可同时出现在字符串内部与值位置）。
 *
 * 这样既能放过 `{"title": {{title:json}}}`，也能识别
 * `{"title": "{{title:json}}"}` 这种「字符串变量又被引号包住」的错误。
 */
export function validateJsonTemplate(text: string): string | null {
  if (!text.trim()) {
    return null;
  }

  const templateError = validateTemplate(text);
  if (templateError) {
    return templateError;
  }

  if (isMiniJinja(text)) {
    // 去掉控制流/注释，把所有 {{ ... }} 换成 null 后只校验静态 JSON 骨架
    const body = splitTemplate(text).body;
    const miniSample = body
      .replace(/\{%[\s\S]*?%\}/g, '')
      .replace(/\{#[\s\S]*?#\}/g, '')
      .replace(/\{\{[\s\S]*?\}\}/g, 'null');
    try {
      JSON.parse(miniSample);
      return null;
    } catch (reason) {
      const detail = reason instanceof Error ? reason.message : String(reason);
      return `JSON 模板不合法（minijinja 占位符代入后无法解析）：${detail}`;
    }
  }

  // 首行是引擎声明，不参与 JSON 结构校验
  const body = splitTemplate(text).body;
  const sample = body.replace(
    PLACEHOLDER_RE,
    (_whole, keyA: string | undefined, specA: string | undefined, keyB: string | undefined, specB: string | undefined) => {
      const key = keyA ?? keyB ?? '';
      const spec = specA ?? specB ?? '';
      if (!key) {
        return 'null';
      }
      const parsed = parseFilterChain(spec);
      const hasJsonFilter =
        Array.isArray(parsed) && parsed.some((call) => call.name === 'json');
      return hasJsonFilter ? '"__value__"' : 'null';
    },
  );

  try {
    JSON.parse(sample);
    return null;
  } catch (reason) {
    const detail = reason instanceof Error ? reason.message : String(reason);
    return `JSON 模板不合法（占位符代入后无法解析）：${detail}`;
  }
}
