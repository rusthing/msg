/**
 * # antd 主题配置（独立站与联邦入口共用）
 *
 * 抽成一份是为了避免「独立站换了令牌、桌面窗口里还是旧色」的漂移：
 * 独立入口从 embed-bridge 读主题，联邦入口从宿主 props 读主题，
 * 但生成 ConfigProvider 配置的逻辑只有这里一处。
 */
import { theme as antdTheme, type ThemeConfig } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import { createContext, useContext } from 'react';

export interface AntdThemeOptions {
  dark: boolean;
}

/**
 * # 明暗模式上下文
 *
 * antd 令牌由 `ConfigProvider` 提供给组件，但有些原生元素（Sider 的 `theme`、模板
 * 编辑器的高亮配色、portal 出去的选择面板）需要直接知道当前是不是暗色。两种入口
 * （独立站读宿主消息、联邦读 props）都在这里写入，组件只读、不各自判断 DOM。
 */
export const ThemeModeContext = createContext(false);

/** # 当前是否为暗色主题 */
export function useThemeMode(): boolean {
  return useContext(ThemeModeContext);
}

export function antdConfig({ dark }: AntdThemeOptions): {
  locale: typeof zhCN;
  theme: ThemeConfig;
  getPopupContainer: (triggerNode?: HTMLElement) => HTMLElement;
} {
  return {
    locale: zhCN,
    /**
     * 弹出层挂到应用根节点，而不是 document.body。
     *
     * 挂 body 时 antd 的可见区域按整个视口算：嵌进 uiam 桌面后，下拉框会画到
     * 窗口外面、并且拿不到 `.msg-federated` 上的主题变量；挂到根节点后，弹层
     * 的滚动祖先链里就有窗口内容区，滚动与边界都会跟着窗口走。
     * 反例也要顾着：Modal / Drawer 本身挂在 body 上，里面的下拉框跟着回 body，
     * 否则弹层会被窗口的层叠上下文压到遮罩下面，看不见。
     */
    getPopupContainer: (triggerNode?: HTMLElement) =>
      triggerNode?.closest<HTMLElement>('.msg-federated') ?? document.body,
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
