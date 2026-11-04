/**
 * 模板与 JSON 校验的单元测试。
 *
 * 运行：`pnpm test`（Node 内置测试运行器 + 原生 TS 类型剥离）。
 */

import assert from 'node:assert/strict';
import test from 'node:test';
import {
  validateJsonTemplate,
  validateJsonText,
  validateTemplate,
} from '../src/utils/validation.ts';

test('模板：合法占位符与过滤器链', () => {
  assert.equal(validateTemplate('你好 {{name}}'), null);
  assert.equal(validateTemplate('你好 ${name}'), null);
  assert.equal(validateTemplate('{{name:trim|truncate(10)}}'), null);
  assert.equal(validateTemplate('{{amount:fixed(2)}}'), null);
  assert.equal(validateTemplate('{{phone:mask(3,4)}}'), null);
  assert.equal(validateTemplate('{{time:time}}'), null);
  assert.equal(validateTemplate('JSON 对象 {"k": 1} 不是占位符'), null);
  assert.equal(validateTemplate(''), null);
});

test('模板：未闭合占位符', () => {
  assert.match(validateTemplate('{{name') ?? '', /未闭合/);
  assert.match(validateTemplate('${name') ?? '', /未闭合/);
  assert.match(validateTemplate('{{name:trim(10}}') ?? '', /右括号/);
});

test('模板：旧单花括号语法', () => {
  assert.match(validateTemplate('{name}') ?? '', /旧语法/);
  assert.match(validateTemplate('{eventCode:json}') ?? '', /旧语法/);
});

test('模板：未知过滤器与非法变量名', () => {
  assert.match(validateTemplate('{{name:date}}') ?? '', /未知过滤器/);
  assert.match(validateTemplate('{{user-name}}') ?? '', /变量名/);
});

test('模板：过滤器参数个数', () => {
  assert.match(validateTemplate('{{name:json(foo)}}') ?? '', /不接受参数/);
  assert.match(validateTemplate('{{name:default}}') ?? '', /需要参数/);
  assert.match(validateTemplate('{{name:truncate}}') ?? '', /需要参数/);
  assert.equal(validateTemplate('{{name:time(YYYY-MM-DD)}}'), null);
});

test('JSON 文本', () => {
  assert.equal(validateJsonText(''), null);
  assert.equal(validateJsonText('{"token": ""}'), null);
  assert.equal(validateJsonText('[1, 2, 3]'), null);
  assert.match(validateJsonText('{"token": }') ?? '', /JSON 格式不正确/);
});

test('JSON 模板：占位符按 JSON 值代入', () => {
  assert.equal(
    validateJsonTemplate('{"eventCode": {{eventCode:json}}, "at": {{businessTriggerMs}}}'),
    null,
  );
  assert.equal(validateJsonTemplate('{"title": "{{title}}"}'), null);
  assert.match(validateJsonTemplate('{"title": "{{title:json}}"}') ?? '', /JSON 模板不合法/);
  assert.match(validateJsonTemplate('{"title": {{title:json}') ?? '', /未闭合/);
  assert.match(validateJsonTemplate('{"title": {{title:json}}') ?? '', /JSON 模板不合法/);
  assert.match(validateJsonTemplate('{eventCode:json}') ?? '', /旧语法/);
});
