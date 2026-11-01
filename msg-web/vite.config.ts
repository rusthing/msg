import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';

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

  return {
    base,
    plugins: [react()],
    server: {
      port: 5175,
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
      // antd 本身接近 1MB（gzip 后约 330KB），拆包后仍会超过默认 500KB 阈值
      chunkSizeWarningLimit: 1250,
      rollupOptions: {
        output: {
          manualChunks: {
            react: ['react', 'react-dom', 'react-router-dom'],
            antd: ['antd'],
            icons: ['@ant-design/icons'],
          },
        },
      },
    },
  };
});
