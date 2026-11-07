/**
 * # 消息中心登录页
 *
 * 与消息域「蓝色 + 浅灰」的视觉保持一致；登录逻辑统一走 `lib/auth-core.ts`。
 */

import { Alert, Button, Card, Input, Typography } from 'antd';
import { useState, type FormEvent } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { AuthError, loginWithPassword } from '../../lib/auth-core';

/** 只接受应用内相对地址，避免 returnUrl 被用来跳外站 */
function safeReturnUrl(raw: string | null): string {
  if (!raw || !raw.startsWith('/') || raw.startsWith('//')) return '/';
  return raw;
}

export default function LoginPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const returnUrl = safeReturnUrl(searchParams.get('returnUrl'));
  const [account, setAccount] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setError(null);
    if (!account.trim() || !password) {
      setError('请输入账号与密码');
      return;
    }
    setLoading(true);
    try {
      await loginWithPassword(account, password);
      navigate(returnUrl, { replace: true });
    } catch (reason) {
      setError(reason instanceof AuthError ? reason.message : '登录失败，请稍后重试');
    } finally {
      setLoading(false);
    }
  }

  return (
    <div
      className="flex min-h-screen items-center justify-center px-4"
      style={{ background: 'linear-gradient(135deg, #eff6ff 0%, #f4f6fa 60%, #eef2fb 100%)' }}
    >
      <Card style={{ width: 380, boxShadow: '0 24px 60px -24px rgba(22, 119, 255, 0.35)' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 20 }}>
          <span
            style={{
              display: 'inline-flex',
              width: 36,
              height: 36,
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: 10,
              background: '#1677ff',
              color: '#fff',
              fontSize: 18,
              fontWeight: 600,
            }}
          >
            消
          </span>
          <div>
            <Typography.Title level={4} style={{ margin: 0 }}>
              消息中心
            </Typography.Title>
            <Typography.Text type="secondary">登录后管理消息定义与投递链路</Typography.Text>
          </div>
        </div>

        <form onSubmit={submit} style={{ display: 'grid', gap: 12 }}>
          <Input
            size="large"
            placeholder="账号"
            autoFocus
            value={account}
            onChange={(event) => setAccount(event.target.value)}
          />
          <Input.Password
            size="large"
            placeholder="密码"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
          {error && <Alert type="error" showIcon message={error} />}
          <Button type="primary" htmlType="submit" size="large" block loading={loading}>
            登录
          </Button>
        </form>
      </Card>
    </div>
  );
}
