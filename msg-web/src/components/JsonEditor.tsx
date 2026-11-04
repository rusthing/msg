/**
 * # JSON 编辑器
 *
 * 渠道配置、路由标签这类 JSON 字段如果允许自由输入，很容易写出非法 JSON，
 * 等到服务端解析才失败。表单校验由 `Form.Item` 的规则统一负责（提交前会拦住），
 * 这里只额外提供一键格式化；格式化失败时给出具体原因。
 *
 * 组件是受控的（`value` / `onChange`），可以直接放进 `Form.Item`。
 */

import { App as AntdApp, Button, Input, Space, Typography } from 'antd';
import { validateJsonText } from '../utils/validation';

interface JsonEditorProps {
  value?: string;
  onChange?: (value: string) => void;
  /** 输入框行数 */
  rows?: number;
  /** 占位文案 */
  placeholder?: string;
}

export default function JsonEditor({
  value = '',
  onChange,
  rows = 8,
  placeholder = '留空表示不配置',
}: JsonEditorProps) {
  const { message } = AntdApp.useApp();

  /** 格式化：解析失败时把错误说出来，不要静默不动 */
  const handleFormat = () => {
    const error = validateJsonText(value);
    if (error) {
      message.error(error);
      return;
    }
    const pretty = JSON.stringify(JSON.parse(value), null, 2);
    onChange?.(pretty);
  };

  return (
    <Space direction="vertical" size={4} style={{ width: '100%' }}>
      <Input.TextArea
        className="msg-json-editor"
        value={value}
        rows={rows}
        spellCheck={false}
        placeholder={placeholder}
        onChange={(event) => onChange?.(event.target.value)}
      />
      <div className="msg-json-bar">
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          留空表示不配置
        </Typography.Text>
        <Button size="small" type="link" onClick={handleFormat} disabled={!value.trim()}>
          格式化
        </Button>
      </div>
    </Space>
  );
}
