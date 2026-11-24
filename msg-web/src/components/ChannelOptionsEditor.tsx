/**
 * # 渠道配置编辑器
 *
 * `msg_channel.options` 以前是一个裸 JSON 文本框，每种渠道的字段只能靠文档记。
 * 这里按同表单的「渠道类型」字段取 schema，渲染成图形化表单；类型没登记时回退
 * 原始 JSON，不阻塞运营配置。
 *
 * 组件是受控的（`value` / `onChange`，值是 JSON 字符串），可直接放进 `Form.Item`。
 * 只写用户改过的键：留空 = 用后端默认值，不会把一堆默认值固化进配置。
 */

import {
  Alert,
  Button,
  Col,
  Collapse,
  Form,
  Input,
  InputNumber,
  Row as AntRow,
  Select,
  Space,
  Switch,
  Typography,
} from 'antd';
import { useEffect, useMemo, useRef, useState } from 'react';
import JsonEditor from './JsonEditor';
import { channelOptionSchema } from '../lib/channel-options';
import type { ChannelOptionField } from '../lib/channel-options';

interface ChannelOptionsEditorProps {
  value?: string;
  onChange?: (value: string) => void;
}

interface ParsedOptions {
  obj: Record<string, unknown>;
  error?: string;
}

/** 空串表示「不配置」；其余必须是 JSON 对象 */
function parseOptions(text: string): ParsedOptions {
  if (!text.trim()) {
    return { obj: {} };
  }
  try {
    const parsed: unknown = JSON.parse(text);
    if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
      return { obj: {}, error: '渠道配置必须是 JSON 对象' };
    }
    return { obj: parsed as Record<string, unknown> };
  } catch {
    return { obj: {}, error: 'JSON 无法解析，请切到「原始 JSON」修正' };
  }
}

/** 短字段一行两个，多行字段占整行 */
function isWideOption(field: ChannelOptionField): boolean {
  return field.kind === 'textarea' || field.kind === 'json';
}

/**
 * # JSON 对象子字段
 *
 * 如短信的 `headers`：保存的是对象，但编辑过程必须允许「暂时不是合法 JSON」。
 * 所以本地留一份文本，只有解析成功才向上提交；外部值变化（切类型、重置表单）
 * 时同步回来。
 */
function JsonObjectField({
  field,
  value,
  onChange,
}: {
  field: ChannelOptionField;
  value: unknown;
  onChange: (value: unknown) => void;
}) {
  const serialized = value === undefined ? '' : JSON.stringify(value, null, 2);
  const [text, setText] = useState(serialized);
  const [error, setError] = useState('');
  const emitted = useRef(serialized);

  useEffect(() => {
    if (serialized === emitted.current) {
      return;
    }
    setText(serialized);
    setError('');
    emitted.current = serialized;
  }, [serialized]);

  const handleChange = (next: string) => {
    setText(next);
    if (!next.trim()) {
      setError('');
      emitted.current = '';
      onChange(undefined);
      return;
    }
    try {
      const parsed: unknown = JSON.parse(next);
      if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
        setError('必须是 JSON 对象');
        return;
      }
      setError('');
      emitted.current = JSON.stringify(parsed, null, 2);
      onChange(parsed);
    } catch {
      setError('JSON 无法解析');
    }
  };

  return (
    <Space direction="vertical" size={2} style={{ width: '100%' }}>
      <Input.TextArea
        className="msg-json-editor"
        rows={field.rows ?? 3}
        spellCheck={false}
        placeholder={field.placeholder}
        value={text}
        onChange={(event) => handleChange(event.target.value)}
      />
      {error ? (
        <Typography.Text type="danger" style={{ fontSize: 12 }}>
          {error}
        </Typography.Text>
      ) : (
        field.tip && (
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {field.tip}
          </Typography.Text>
        )
      )}
    </Space>
  );
}

export default function ChannelOptionsEditor({
  value = '',
  onChange,
}: ChannelOptionsEditorProps) {
  const form = Form.useFormInstance();
  const channelType = Form.useWatch('channelType', form) as string | undefined;
  const schema = channelOptionSchema(channelType);
  const { obj, error } = useMemo(() => parseOptions(value), [value]);
  /** 不属于当前类型的键：多半是切换类型后留下的，后端解析时同样会被忽略 */
  const extraKeys = useMemo(() => {
    if (!schema) {
      return [];
    }
    const known = new Set(schema.fields.map((field) => field.key));
    return Object.keys(obj).filter((key) => !known.has(key));
  }, [obj, schema]);

  /** 写一个键；`undefined` 表示删除该键（回到后端默认值） */
  const write = (key: string, next: unknown) => {
    const draft: Record<string, unknown> = { ...obj };
    if (next === undefined) {
      delete draft[key];
    } else {
      draft[key] = next;
    }
    onChange?.(Object.keys(draft).length === 0 ? '' : JSON.stringify(draft, null, 2));
  };

  const renderControl = (field: ChannelOptionField) => {
    const current = obj[field.key];
    switch (field.kind) {
      case 'number':
        return (
          <InputNumber
            style={{ width: '100%' }}
            min={field.min}
            max={field.max}
            placeholder={field.placeholder}
            value={typeof current === 'number' ? current : undefined}
            onChange={(next) => write(field.key, next === null ? undefined : next)}
          />
        );
      case 'bool':
        return (
          <Space size={8}>
            <Switch checked={Boolean(current)} onChange={(checked) => write(field.key, checked)} />
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              {current === undefined ? `默认：${field.placeholder ?? '否'}` : ''}
            </Typography.Text>
          </Space>
        );
      case 'select':
        return (
          <Select
            allowClear
            style={{ width: '100%' }}
            options={field.options}
            placeholder={field.placeholder}
            value={typeof current === 'string' ? current : undefined}
            onChange={(next) => write(field.key, next)}
          />
        );
      case 'secret':
        return (
          <Input.Password
            autoComplete="off"
            placeholder={field.placeholder}
            value={typeof current === 'string' ? current : ''}
            onChange={(event) => write(field.key, event.target.value)}
          />
        );
      case 'textarea':
        return (
          <Input.TextArea
            className="msg-json-editor"
            rows={field.rows ?? 3}
            spellCheck={false}
            placeholder={field.placeholder}
            value={typeof current === 'string' ? current : ''}
            onChange={(event) => write(field.key, event.target.value)}
          />
        );
      case 'json':
        return (
          <JsonObjectField
            field={field}
            value={current}
            onChange={(next) => write(field.key, next)}
          />
        );
      default:
        return (
          <Input
            autoComplete="off"
            placeholder={field.placeholder}
            value={typeof current === 'string' ? current : ''}
            onChange={(event) => write(field.key, event.target.value)}
          />
        );
    }
  };

  return (
    <Space direction="vertical" size={8} style={{ width: '100%' }}>
      {!channelType && (
        <Alert type="info" showIcon title="先选择渠道类型，再配置这个渠道的参数" />
      )}
      {channelType && !schema && (
        <Alert
          type="warning"
          showIcon
          title={`类型「${channelType}」没有内置表单`}
          description="用下面的原始 JSON 配置。后端没有对应发送器时，投递会报「渠道未实现」。"
        />
      )}
      {schema && (
        <Alert type="info" showIcon title={schema.label} description={schema.desc} />
      )}
      {schema && extraKeys.length > 0 && (
        <Alert
          type="warning"
          showIcon
          title={`有 ${extraKeys.length} 个键不属于「${channelType}」`}
          description={`${extraKeys.join('、')}：后端解析时会忽略，但留着会让人误以为生效，建议清掉。`}
          action={
            <Button
              size="small"
              onClick={() =>
                onChange?.(
                  JSON.stringify(
                    Object.fromEntries(
                      Object.entries(obj).filter(([key]) => !extraKeys.includes(key)),
                    ),
                    null,
                    2,
                  ),
                )
              }
            >
              清掉
            </Button>
          }
        />
      )}
      {error && (
        <Alert type="error" showIcon title={error} description="图形化表单已停用，避免覆盖坏掉的配置。" />
      )}

      {schema && !error && (
        <AntRow gutter={[12, 8]}>
          {schema.fields.map((field) => (
            <Col span={isWideOption(field) ? 24 : 12} key={field.key}>
              <Space direction="vertical" size={2} style={{ width: '100%' }}>
                <Typography.Text>{field.label}</Typography.Text>
                {renderControl(field)}
                {!isWideOption(field) &&
                  field.tip &&
                  (field.kind === 'json' ? null : (
                    <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                      {field.tip}
                    </Typography.Text>
                  ))}
              </Space>
            </Col>
          ))}
        </AntRow>
      )}

      <Collapse
        ghost
        size="small"
        items={[
          {
            key: 'raw',
            label: '原始 JSON（高级）',
            children: (
              <JsonEditor
                rows={8}
                placeholder='留空表示全部用默认值，如 {"token":"xxx"}'
                value={value}
                onChange={(next) => onChange?.(next)}
              />
            ),
          },
        ]}
      />
    </Space>
  );
}
