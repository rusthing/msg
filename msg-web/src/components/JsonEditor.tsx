/**
 * # JSON 编辑器
 *
 * 渠道配置、路由标签、消息模板的 json 字段如果允许自由输入，很容易写出非法 JSON，
 * 等到服务端解析才失败。这里在失焦时先校验、并提供一键格式化，把错误挡在提交之前。
 *
 * 组件是受控的（`value` / `onChange`），可以直接放进 `Form.Item`。
 */

import { Button, Input, Space, Typography } from 'antd';
import { useState } from 'react';

interface JsonEditorProps {
  value?: string;
  onChange?: (value: string) => void;
  /** 输入框行数 */
  rows?: number;
}

/** 校验 JSON 文本，合法时返回 `null`；空串按「不填」处理，不算错误 */
function validate(text: string): string | null {
  if (!text.trim()) {
    return null;
  }
  try {
    JSON.parse(text);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : 'JSON 格式不正确';
  }
}

export default function JsonEditor({ value = '', onChange, rows = 8 }: JsonEditorProps) {
  const [error, setError] = useState<string | null>(null);

  /** 格式化：解析失败时把错误说出来，不要静默不动 */
  const handleFormat = () => {
    if (!value.trim()) {
      setError(null);
      return;
    }
    try {
      const pretty = JSON.stringify(JSON.parse(value), null, 2);
      onChange?.(pretty);
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'JSON 格式不正确');
    }
  };

  return (
    <Space direction="vertical" size={4} style={{ width: '100%' }}>
      <Input.TextArea
        className="msg-json-editor"
        value={value}
        rows={rows}
        spellCheck={false}
        status={error ? 'error' : undefined}
        onChange={(event) => {
          onChange?.(event.target.value);
          // 边输入边清掉旧错误，避免改好了红框还挂着
          if (error) {
            setError(null);
          }
        }}
        onBlur={() => setError(validate(value))}
      />
      <div className="msg-json-bar">
        {error ? (
          <Typography.Text type="danger" style={{ fontSize: 12 }}>
            {error}
          </Typography.Text>
        ) : (
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            留空表示不配置
          </Typography.Text>
        )}
        <Button size="small" type="link" onClick={handleFormat} disabled={!value.trim()}>
          格式化
        </Button>
      </div>
    </Space>
  );
}
