import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';
import { federation } from '@module-federation/vite';

/**
 * # 规范化 base
 *
 * 允许 `VITE_PUBLIC_BASE` 写成 `msg-web`、`/msg-web`、`/msg-web/` 三种形态，
 * 统一成 Vite 要求的 `/msg-web/`；空值或 `/` 表示部署在根路径。
 */
function normalizeBase(raw?: string): string {
  const trimmed = (raw ?? '').trim();
  if (!trimmed || trimmed === '/') {
    return '/';
  }
  return `/${trimmed.replace(/^\/+|\/+$/g, '')}/`;
}

/**
 * # Vite 配置
 *
 * 开发态把 `/msg/**` 反代到 msg-svr，避免跨域；生产态由网关或 nginx 承担同样的转发，
 * 因此前端代码里始终使用 `/msg/...` 这样的同源相对路径。
 *
 * 注意 `/msg` 是**服务端路由前缀**（`#[router]` 按 `MsgXxxRouter` 结构体名生成），
 * 不是服务名；改前缀只影响后端，不在这里改。
 */
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');
  const proxyTarget = env.VITE_PROXY_TARGET || 'http://127.0.0.1:9002';
  const base = normalizeBase(env.VITE_PUBLIC_BASE);

  // React 必须与 uiam / syn 单例共享，否则联邦窗口里会出现两套 hooks dispatcher。
  // 注意：这里**不能**再用 manualChunks 把 react 拆成独立 chunk —— 那样远程会带上
  // 自己的 react 副本，singleton 直接失效。federation 插件接管依赖共享。
  const sharedReact = { singleton: true, allowNodeModulesSuffixMatch: true } as const;

  return {
    base,
    plugins: [
      // 只暴露一个页面级模块给 uiam 桌面；独立站仍照常 createRoot，两者共用 AppRoutes。
      federation({
        name: 'msg-web',
        filename: 'remoteEntry.js',
        exposes: { './msg-app': './src/federation/MsgFederatedApp.tsx' },
        shared: {
          react: sharedReact,
          'react/jsx-runtime': sharedReact,
          'react-dom': sharedReact,
          'react-dom/client': sharedReact,
        },
        // antd 的样式（含 ConfigProvider 主题变量）必须随暴露模块一起出，
        // 否则宿主只拿到组件、没有令牌。
        bundleAllCSS: true,
        manifest: true,
      }),
      react(),
    ],
    server: {
      port: 5175,
      // dev 下联邦 runtime 要用绝对地址取 remoteEntry.js，否则 uiam 会从自己的端口找。
      origin: env.VITE_DEV_ORIGIN?.trim() || 'http://localhost:5175',
      proxy: {
        '/msg': {
          target: proxyTarget,
          changeOrigin: true,
        },
      },
    },
    build: {
      outDir: 'dist',
      sourcemap: false,
      chunkSizeWarningLimit: 1250,
    },
  };
});
