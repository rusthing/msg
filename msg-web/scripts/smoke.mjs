/**
 * # msg-web 冒烟脚本
 *
 * 逐个打开所有页面，等首屏数据渲染完再截图，同时收集控制台报错。
 * 只能验证「页面能出来、数据能到、没有运行时异常」，不做像素比对。
 *
 * 用法：
 *   pnpm smoke                      # 需要先启动 pnpm dev
 *   pnpm smoke --base http://localhost:5175 --out /tmp/msg-shots
 */
import { mkdir } from 'node:fs/promises';
import path from 'node:path';
import { chromium } from 'playwright';

const argv = process.argv.slice(2);
const argOf = (name, fallback) => {
  const i = argv.indexOf(`--${name}`);
  return i >= 0 && argv[i + 1] ? argv[i + 1] : fallback;
};

const BASE = argOf('base', process.env.SMOKE_BASE ?? 'http://localhost:5175');
const OUT = argOf('out', process.env.SMOKE_OUT ?? '.smoke');
const VIEWPORT = { width: 1440, height: 900 };

/** 路由与期望：`list` 表示页面里应该出现表格（可能有数据也可能空表） */
const PAGES = [
  { path: '/', name: '00-dashboard' },
  { path: '/event-source', name: '01-event-source', list: true },
  { path: '/message-category', name: '02-message-category', list: true },
  { path: '/target-category', name: '03-target-category', list: true },
  { path: '/channel', name: '04-channel', list: true },
  { path: '/message', name: '05-message', list: true },
  { path: '/message-target', name: '06-message-target', list: true },
  { path: '/target-category-channel', name: '07-target-category-channel', list: true },
  { path: '/delivery', name: '08-delivery', list: true },
  { path: '/delivery-target', name: '09-delivery-target', list: true },
  { path: '/delivery-channel', name: '10-delivery-channel', list: true },
  { path: '/delivery-channel-log', name: '11-delivery-channel-log', list: true },
  { path: '/dead-letter', name: '12-dead-letter', list: true },
  { path: '/message-queue', name: '13-message-queue', list: true },
  { path: '/delivery-queue', name: '14-delivery-queue', list: true },
];

const failures = [];
const consoleErrors = [];

const browser = await chromium.launch();
const context = await browser.newContext({ viewport: VIEWPORT, deviceScaleFactor: 2 });
const page = await context.newPage();

page.on('console', (msg) => {
  if (msg.type() === 'error') consoleErrors.push({ url: page.url(), text: msg.text() });
});
page.on('pageerror', (err) => consoleErrors.push({ url: page.url(), text: `pageerror: ${err.message}` }));

await mkdir(OUT, { recursive: true });

for (const target of PAGES) {
  const url = `${BASE}${target.path}`;
  try {
    await page.goto(url, { waitUntil: 'domcontentloaded' });
    // 等 antd 骨架消失、内容区出现：有表格的页面等数据行或空态，总览等统计卡
    if (target.list) {
      // 只认 `.ant-table-row` / 空态行：`.ant-table-tbody tr` 的首个匹配是 antd 量
      // 列宽用的隐藏行（高度 0），waitForSelector 只看第一个匹配元素，等不到 visible；
      // 而空表用自定义 emptyText，没有 `.ant-empty`，空态落在 `.ant-table-placeholder`
      await page.waitForSelector('.ant-table-row, .ant-table-placeholder', { timeout: 15000 });
      // 表格首行渲染完之前会有 loading 遮罩，等它散掉再截
      await page.waitForFunction(() => !document.querySelector('.ant-spin-spinning'), null, { timeout: 15000 });
    } else {
      await page.waitForSelector('.ant-statistic, .ant-card', { timeout: 15000 });
      await page.waitForFunction(() => !document.querySelector('.ant-spin-spinning'), null, { timeout: 15000 });
    }
    await page.waitForTimeout(400);

    const file = path.join(OUT, `${target.name}.png`);
    await page.screenshot({ path: file, fullPage: true });

    const rowCount = await page.locator('.ant-table-tbody tr.ant-table-row').count();
    console.log(`ok   ${target.path.padEnd(26)} rows=${rowCount}  -> ${file}`);
  } catch (err) {
    failures.push({ path: target.path, message: err.message.split('\n')[0] });
    console.log(`FAIL ${target.path.padEnd(26)} ${err.message.split('\n')[0]}`);
    await page.screenshot({ path: path.join(OUT, `${target.name}-fail.png`), fullPage: true }).catch(() => {});
  }
}

// 交互一遍：新增弹窗、详情抽屉、关键词搜索
try {
  await page.goto(`${BASE}/channel`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.ant-table-row, .ant-table-placeholder', { timeout: 15000 });

  await page.getByRole('button', { name: /新增/ }).first().click();
  await page.waitForSelector('.ant-modal-content', { timeout: 8000 });
  await page.waitForTimeout(500);
  await page.screenshot({ path: path.join(OUT, '90-modal-add.png'), fullPage: false });
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  const detail = page.getByRole('button', { name: /详情/ }).first();
  if (await detail.count()) {
    await detail.click();
    await page.waitForSelector('.ant-drawer-content', { timeout: 8000 });
    await page.waitForTimeout(500);
    await page.screenshot({ path: path.join(OUT, '91-drawer-detail.png'), fullPage: false });
    await page.keyboard.press('Escape');
  }
  console.log('ok   交互：新增弹窗 / 详情抽屉');
} catch (err) {
  failures.push({ path: 'interaction', message: err.message.split('\n')[0] });
  console.log(`FAIL 交互：${err.message.split('\n')[0]}`);
}

await browser.close();

console.log(`\n截图目录：${path.resolve(OUT)}`);
if (consoleErrors.length) {
  console.log('\n控制台报错：');
  for (const item of consoleErrors) console.log(`  [${item.url}] ${item.text.slice(0, 300)}`);
}
if (failures.length) {
  console.log(`\n失败 ${failures.length} 项`);
  process.exit(1);
}
console.log('\n全部页面通过');
