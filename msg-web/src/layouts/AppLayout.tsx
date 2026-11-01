/**
 * # 应用骨架
 *
 * 左侧按「消息定义 / 投递链路 / 队列」分组导航，顶部说明栏取自当前路由对应的实体元数据，
 * 避免每个页面各写一遍标题。
 */

import { DashboardOutlined } from '@ant-design/icons';
import { Layout, Menu, Typography } from 'antd';
import type { MenuProps } from 'antd';
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

  /**
   * 当前实体：**精确**匹配路径
   *
   * 不能用 `startsWith`：`/message` 是 `/message-queue`、`/message-target` 的前缀，
   * `/delivery` 是 `/delivery-*` 的前缀，前缀匹配会让这些页面的标题和高亮落到错的菜单项上。
   */
  const current = ENTITIES.find((entity) => location.pathname === `/${entity.key}`);

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
        <Menu
          mode="inline"
          selectedKeys={[current ? `/${current.key}` : '/']}
          style={{ borderInlineEnd: 'none', paddingTop: 8 }}
          items={menuItems}
        />
      </Sider>

      <Layout>
        <Header className="msg-header">
          <div>
            <div className="msg-header-title">{current?.title ?? OVERVIEW.title}</div>
            <div className="msg-header-desc">{current?.desc ?? OVERVIEW.desc}</div>
          </div>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            列表筛选为等值匹配，模糊搜索请用「关键字」
          </Typography.Text>
        </Header>
        <Content className="msg-content">
          <Outlet />
        </Content>
      </Layout>
    </Layout>
  );
}
