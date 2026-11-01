/**
 * # 布尔开关
 *
 * 列表里的 `enabled` / `handled` 直接用开关切换，省掉一次「打开弹窗 → 改 → 保存」。
 *
 * 提交时只带 `{ id, 字段名: 新值 }`：服务端 ModifyDto 的字段全是 `Option`，没传的字段
 * 不会被写回，所以不必把整条记录带上。
 *
 * 开关先按点击后的状态翻过去（列表刷新回来有肉眼可见的延迟），失败了再退回原状。
 */

import { App as AntdApp, Switch } from 'antd';
import { useEffect, useState } from 'react';
import { errorMessage } from '../utils/format';

interface BoolSwitchProps {
  /** 当前值，来自列表数据 */
  value: boolean;
  /** 提交切换；成功后由调用方负责刷新列表 */
  onToggle: (next: boolean) => Promise<void>;
  /** 切换前的二次确认文案；不传则直接切 */
  confirmContent?: string;
}

export default function BoolSwitch({ value, onToggle, confirmContent }: BoolSwitchProps) {
  const { message, modal } = AntdApp.useApp();
  const [checked, setChecked] = useState(value);
  const [submitting, setSubmitting] = useState(false);

  // 列表刷新回来后以服务端为准
  useEffect(() => {
    setChecked(value);
  }, [value]);

  const submit = async (next: boolean) => {
    setChecked(next);
    setSubmitting(true);
    try {
      await onToggle(next);
    } catch (reason) {
      setChecked(!next);
      message.error(errorMessage(reason));
    } finally {
      setSubmitting(false);
    }
  };

  const handleChange = (next: boolean) => {
    if (confirmContent && !next) {
      modal.confirm({
        title: confirmContent,
        okText: '确认',
        cancelText: '取消',
        okButtonProps: { danger: true },
        onOk: () => submit(next),
      });
      return;
    }
    void submit(next);
  };

  return (
    <Switch
      size="small"
      checked={checked}
      loading={submitting}
      onChange={handleChange}
    />
  );
}
