/**
 * # 消息中心登录页
 *
 * 视图统一走 `login/LoginShell`；本文件只保留 returnUrl 安全校验、认证调用与跳转。
 * 登录逻辑统一走 `lib/auth-core.ts`，与其余六个控制台共用一套身份。
 */

import { useNavigate, useSearchParams } from 'react-router-dom';
import { AuthError, loginWithPassword } from '../../lib/auth-core';
import { LoginShell } from '../../login/LoginShell';

/** 只接受应用内相对地址，避免 returnUrl 被用来跳外站 */
function safeReturnUrl(raw: string | null): string {
  if (!raw || !raw.startsWith('/') || raw.startsWith('//')) return '/';
  return raw;
}

export default function LoginPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const returnUrl = safeReturnUrl(searchParams.get('returnUrl'));

  async function submit(account: string, password: string) {
    try {
      await loginWithPassword(account, password);
    } catch (reason) {
      throw reason instanceof AuthError ? reason : new Error('登录失败，请稍后重试');
    }
    navigate(returnUrl, { replace: true });
  }

  return <LoginShell brand="msg" onSubmit={submit} />;
}
