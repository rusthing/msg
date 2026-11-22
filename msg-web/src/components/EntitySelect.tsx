/**
 * # 外键/主体下拉
 *
 * 和用户管理表格一样的信息密度：候选项分两行展示（名称 + 工号/ID/路径），
 * 支持输入名称、工号或 ID 过滤。统一封装是为了让「要填 ID」的地方都得到同样的体验。
 */

import { Select } from 'antd';
import type { CSSProperties } from 'react';

export interface EntitySelectOption {
  value: string;
  /** 主文案：名称 */
  label: string;
  /** 次文案：工号、完整 ID、路径等 */
  detail?: string;
  disabled?: boolean;
}

interface EntitySelectProps {
  value?: string | number;
  options: EntitySelectOption[];
  onChange?: (value: string | undefined) => void;
  loading?: boolean;
  placeholder?: string;
  allowClear?: boolean;
  className?: string;
  style?: CSSProperties;
}

export function EntitySelect({
  value,
  options,
  onChange,
  loading = false,
  placeholder,
  allowClear = true,
  className,
  style,
}: EntitySelectProps) {
  return (
    <Select
      value={value === undefined || value === null ? undefined : String(value)}
      options={options}
      loading={loading}
      placeholder={placeholder}
      allowClear={allowClear}
      className={className}
      style={style}
      showSearch
      optionFilterProp="label"
      filterOption={(input, option) => {
        const keyword = input.trim().toLowerCase();
        if (!keyword) {
          return true;
        }
        const data = option as EntitySelectOption | undefined;
        return [data?.label, data?.detail]
          .filter(Boolean)
          .some((text) => String(text).toLowerCase().includes(keyword));
      }}
      optionRender={(option) => {
        const data = option.data as EntitySelectOption;
        return (
          <div className="msg-entity-option">
            <span className="msg-entity-option-label">{data.label}</span>
            {data.detail ? <span className="msg-entity-option-detail">{data.detail}</span> : null}
          </div>
        );
      }}
      onChange={(next) => onChange?.(next === undefined || next === null ? undefined : String(next))}
    />
  );
}
