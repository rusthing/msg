/// <reference types="vite/client" />

/** # 环境变量 */
interface ImportMetaEnv {
  /** 接口前缀，留空表示同源 */
  readonly VITE_API_BASE?: string;
  /** 前端自身被访问的路径前缀，如 `/msg-web`；留空表示部署在根路径 */
  readonly VITE_PUBLIC_BASE?: string;
  /** 开发态代理目标 */
  readonly VITE_PROXY_TARGET?: string;
  /** 认证模式：`required`（缺省）或 `disabled`（仅开发环境免登录） */
  readonly VITE_AUTH_MODE?: 'required' | 'disabled';
  /** 认证接口前缀（登录 / me / 登出），如 `/adm` 或 `/admrs-svr/adm` */
  readonly VITE_AUTH_BASE?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
