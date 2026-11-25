/**
 * 联邦入口：给 uiam 桌面用的页面级模块。
 *
 * 与独立访问的区别只在「外壳」：这里用 MemoryRouter 把路由关在窗口内部，不碰宿主
 * 地址栏；页面清单、数据接口和鉴权仍复用 App.tsx 的同一份实现。主题通过 props
 * 传入（React 单例后 props 就是最可靠的通道），标题由当前路由推导后回报给宿主。
 */
import { useEffect, useRef } from 'react';
import { App as AntdApp, ConfigProvider } from 'antd';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { AppRoutes } from '../App';
import { ENTITIES } from '../entities';
import { antdConfig, ThemeModeContext } from '../lib/theme-config';
import { FederatedOverlayRoot } from '../lib/federated-overlay';
import '../styles/global.css';
import './federation.css';

export interface MsgFederatedProps {
  theme?: 'light' | 'dark';
  initialPath?: string;
  onRoute?: (path: string, title: string) => void;
}

/** 路由 → 标题：实体页用实体的 title，首页用固定标题 */
function titleOf(pathname: string): string {
  const slug = pathname.replace(/^\/+/, '');
  if (!slug) return '消息中心';
  const hit = ENTITIES.find((entity) => entity.key === slug);
  return hit ? hit.title : '消息中心';
}

function RouteReporter({ onRoute }: { onRoute?: MsgFederatedProps['onRoute'] }) {
  const { pathname } = useLocation();
  useEffect(() => {
    onRoute?.(pathname, titleOf(pathname));
  }, [pathname, onRoute]);
  return null;
}

export default function MsgFederatedApp({
  theme = 'light',
  initialPath = '/',
  onRoute,
}: MsgFederatedProps) {
  const entry = initialPath.startsWith('/') ? initialPath : '/';
  const config = antdConfig({ dark: theme === 'dark' });
  // Modal / Drawer 挂到这个节点上：遮罩只盖本窗口，而不是整个 uiam 桌面
  const rootRef = useRef<HTMLDivElement>(null);
  return (
    <FederatedOverlayRoot resolve={() => rootRef.current}>
      <div ref={rootRef} className="msg-federated" data-theme={theme}>
        <ThemeModeContext.Provider value={theme === 'dark'}>
          <ConfigProvider {...config}>
            <AntdApp>
              <MemoryRouter initialEntries={[entry]}>
                <RouteReporter onRoute={onRoute} />
                <AppRoutes />
              </MemoryRouter>
            </AntdApp>
          </ConfigProvider>
        </ThemeModeContext.Provider>
      </div>
    </FederatedOverlayRoot>
  );
}
