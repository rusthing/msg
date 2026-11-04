/**
 * # 应用骨架
 *
 * 左侧按「消息定义 / 投递链路 / 队列」分组导航，顶部说明栏取自当前路由对应的实体元数据，
 * 避免每个页面各写一遍标题。
 *
 * 桌面端用固定 Sider，窄屏（<992px）隐藏 Sider，改用顶部菜单按钮打开 Drawer，
 * 避免 234px 的侧栏把内容区压到不可用。
 */

import { DashboardOutlined, MenuOutlined } from '@ant-design/icons';
import { Button, Drawer, Layout, Menu, Typography } from 'antd';
import type { MenuProps } from 'antd';
import { useState } from 'react';
import { Link, Outlet, useLocation } from 'react-router-dom';
import { ENTITIES, GROUP_LABELS, type EntityGroup } from '../entities';

const { Sider, Header, Content } = Layout;

/** 分组顺序 */
const GROUP_ORDER: EntityGroup[] = ['definition', 'delivery', 'queue'];

/** 顶部说明栏内容 */
const OVERVIEW = {
  title: '总览',
  desc: '消息中心全局：定义了多少消息、投递成功与否、死信有没有积压',
};

export default function AppLayout() {
  const location = useLocation();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  /**
   * 当前实体：**精确**匹配路径
   *
   * 不能用 `startsWith`：`/message` 是 `/message-queue`、`/message-target` 的前缀，
   * `/delivery` 是 `/delivery-*` 的前缀，前缀匹配会让这些页面的标题和高亮落到错的菜单项上。
   *
   * 末尾斜杠一并去掉：静态托管在刷新时偶尔会把 `/message` 规范成 `/message/`，
   * 留着斜杠就精确匹配不上了，会把高亮和标题错误地回退到「总览」。
   */
  const pathname = location.pathname.replace(/\/+$/, '') || '/';
  const current = ENTITIES.find((entity) => pathname === `/${entity.key}`);

  const menuItems: MenuProps['items'] = [
    {
      key: '/',
      icon: <DashboardOutlined />,
      label: <Link to="/">总览</Link>,
    },
    ...GROUP_ORDER.map((group) => ({
      type: 'group' as const,
      label: GROUP_LABELS[group],
      children: ENTITIES.filter((entity) => entity.group === group).map((entity) => ({
        key: `/${entity.key}`,
        icon: entity.icon,
        label: <Link to={`/${entity.key}`}>{entity.title}</Link>,
      })),
    })),
  ];

  const menu = (
    <Menu
      mode="inline"
      selectedKeys={[current ? `/${current.key}` : '/']}
      style={{ borderInlineEnd: 'none', paddingTop: 8 }}
      items={menuItems}
      onClick={() => setMobileMenuOpen(false)}
    />
  );

  return (
    <Layout style={{ height: '100vh' }}>
      <Sider width={234} theme="light" className="msg-sider">
        <div className="msg-brand">
          <div className="msg-brand-logo">msg</div>
          <div className="msg-brand-text">
            <span className="msg-brand-title">消息中心</span>
            <span className="msg-brand-sub">msg-svr</span>
          </div>
        </div>
        {menu}
      </Sider>

      <Layout>
        <Header className="msg-header">
          <Button
            type="text"
            className="msg-mobile-menu-btn"
            aria-label="打开菜单"
            icon={<MenuOutlined />}
            onClick={() => setMobileMenuOpen(true)}
          />
          <div className="msg-header-main">
            <div className="msg-header-title">{current?.title ?? OVERVIEW.title}</div>
            <div className="msg-header-desc">{current?.desc ?? OVERVIEW.desc}</div>
          </div>
          <Typography.Text type="secondary" className="msg-header-note" style={{ fontSize: 12 }}>
            列表筛选为等值匹配，模糊搜索请用「关键字」
          </Typography.Text>
        </Header>
        <Content className="msg-content">
          <Outlet />
        </Content>
      </Layout>

      <Drawer
        open={mobileMenuOpen}
        title="消息中心"
        placement="left"
        width={264}
        rootClassName="msg-mobile-drawer"
        onClose={() => setMobileMenuOpen(false)}
        styles={{ body: { padding: 0 } }}
      >
        {menu}
      </Drawer>
    </Layout>
  );
}
