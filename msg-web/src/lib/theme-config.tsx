/**
 * # antd 主题配置（独立站与联邦入口共用）
 *
 * 抽成一份是为了避免「独立站换了令牌、桌面窗口里还是旧色」的漂移：
 * 独立入口从 embed-bridge 读主题，联邦入口从宿主 props 读主题，
 * 但生成 ConfigProvider 配置的逻辑只有这里一处。
 */
import { theme as antdTheme, type ThemeConfig } from 'antd';
import zhCN from 'antd/locale/zh_CN';

export interface AntdThemeOptions {
  dark: boolean;
}

export function antdConfig({ dark }: AntdThemeOptions): {
  locale: typeof zhCN;
  theme: ThemeConfig;
} {
  return {
    locale: zhCN,
    theme: {
      algorithm: dark ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
      token: {
        colorPrimary: '#1677ff',
        borderRadius: 6,
        fontSize: 14,
      },
      components: {
        Layout: {
          headerBg: dark ? '#1f1f1f' : '#ffffff',
          bodyBg: dark ? '#141414' : '#f4f6fa',
          siderBg: dark ? '#1f1f1f' : '#ffffff',
        },
        Table: {
          headerBg: dark ? '#262626' : '#fafbfd',
        },
      },
    },
  };
}
