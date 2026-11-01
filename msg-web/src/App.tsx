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
          <Route key={entity.key} path={entity.key} element={<EntityPage config={entity} />} />
        ))}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
