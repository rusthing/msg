/**
 * # 应用入口
 */

import { App as AntdApp, ConfigProvider } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter, HashRouter } from 'react-router-dom';
import 'dayjs/locale/zh-cn';
import App from './App';
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
 * 这样「页面挂在哪个子路径」只需要改一处 `VITE_PUBLIC_BASE`。
 *
 * hash 模式例外：react-router 会拿 basename 去匹配 hash 里的路径，而 hash 里只有
 * `/message` 没有 `/msg-web`，带上去就整页空白，所以这里必须留空。
 */
const BASENAME = USE_HASH ? '' : import.meta.env.BASE_URL.replace(/\/+$/, '') || '/';

const Router = USE_HASH ? HashRouter : BrowserRouter;

ReactDOM.createRoot(container).render(
  <React.StrictMode>
    <ConfigProvider
      locale={zhCN}
      theme={{
        token: {
          colorPrimary: '#1677ff',
          borderRadius: 6,
          fontSize: 14,
        },
        components: {
          Layout: {
            headerBg: '#ffffff',
            bodyBg: '#f4f6fa',
            siderBg: '#ffffff',
          },
          Table: {
            headerBg: '#fafbfd',
          },
        },
      }}
    >
      <AntdApp>
        <Router basename={BASENAME}>
          <App />
        </Router>
      </AntdApp>
    </ConfigProvider>
  </React.StrictMode>,
);
