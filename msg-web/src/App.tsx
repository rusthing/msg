/**
 * # 路由表
 *
 * 14 张表的路由由 `ENTITIES` 直接展开，不逐个写 `<Route>`：加一个实体只改元数据，
 * 路由、导航、页面一起生效，也不会出现「页面写了但忘了挂路由」。
 */

import { Navigate, Route, Routes } from 'react-router-dom';
import { ENTITIES } from './entities';
import AppLayout from './layouts/AppLayout';
import Dashboard from './pages/Dashboard';
import EntityPage from './pages/EntityPage';

export default function App() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<Dashboard />} />
        {ENTITIES.map((entity) => (
          /*
           * `key` 必须挂在 EntityPage 上（不是只挂在 Route 上）：
           * 切换实体时如果沿用同一个组件实例，React 只会更新 props、保留内部 state，
           * `usePagedQuery` 的 effect 依赖里又没有 `config.path`，于是列表停在上一页的数据，
           * 要手动点「刷新」才变。加 key 强制重建，切菜单即按新实体重新拉数据。
           */
          <Route
            key={entity.key}
            path={entity.key}
            element={<EntityPage key={entity.key} config={entity} />}
          />
        ))}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
