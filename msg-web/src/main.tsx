/**
 * # 应用入口（独立访问）
 *
 * 联邦入口在 src/federation/MsgFederatedApp.tsx，两者共用 AppRoutes 与主题配置。
 */

import { App as AntdApp, ConfigProvider } from 'antd';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter, HashRouter } from 'react-router-dom';
import 'dayjs/locale/zh-cn';
import App from './App';
import { initEmbed, useEmbedTheme } from './lib/embed-bridge';
import { antdConfig, ThemeModeContext } from './lib/theme-config';
import { FederatedOverlayRoot } from './lib/federated-overlay';
import './styles/global.css';

const container = document.getElementById('root');
if (!container) {
  throw new Error('找不到挂载节点 #root');
}

/**
 * # 路由模式
 *
 * 静态服务器不方便配 SPA 回退（`try_files $uri /msg-web/index.html`）时，设
 * `VITE_ROUTER=hash` 退化成 hash 路由：路径永远停在目录首页，路由挂在 `#` 后面，
 * 这样在子菜单页按 F5 刷新也不会 404、侧边栏不会整片消失。
 * 默认仍走 history 模式（更干净的路径）。
 */
const USE_HASH = import.meta.env.VITE_ROUTER === 'hash';

/**
 * # 路由前缀（basename）
 *
 * 直接取 Vite 的 `BASE_URL`（就是 `vite.config.ts` 里的 `base`，形如 `/msg-web/`），
 * 去掉末尾斜杠给 `BrowserRouter`；根路径部署时为空串，按 `/` 处理。
 *
 * hash 模式例外：react-router 会拿 basename 去匹配 hash 里的路径，而 hash 里只有
 * `/message` 没有 `/msg-web`，带上去就整页空白，所以这里必须留空。
 */
const BASENAME = USE_HASH ? '' : import.meta.env.BASE_URL.replace(/\/+$/, '') || '/';

const Router = USE_HASH ? HashRouter : BrowserRouter;

/** 嵌入态由宿主下发主题；独立访问时保持原有视觉 */
function Root() {
  const embeddedTheme = useEmbedTheme();
  const config = antdConfig({ dark: embeddedTheme === 'dark' });
  return (
    <React.StrictMode>
      <ThemeModeContext.Provider value={embeddedTheme === 'dark'}>
        <ConfigProvider {...config}>
          <AntdApp>
            <Router basename={BASENAME}>
              <App />
            </Router>
          </AntdApp>
        </ConfigProvider>
      </ThemeModeContext.Provider>
    </React.StrictMode>
  );
}

// 必须在 createRoot 之前装桥接监听，否则宿主的 init 消息会错过
initEmbed('msg');

ReactDOM.createRoot(container).render(
  // 独立站的对话框也挂到 #root：与联邦窗口里「挂应用根节点」保持同一套行为
  <FederatedOverlayRoot resolve={() => container}>
    <Root />
  </FederatedOverlayRoot>,
);
