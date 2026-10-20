import { test, expect } from '@playwright/test';

/**
 * L2 页面覆盖冒烟测试（补足计划中的 22 例）。
 *
 * 说明：本文件在**浏览器环境**下运行（Playwright 访问 Vite dev server），
 * 因此只断言"路由可达 + 布局与主内容区渲染"，不断言依赖 Tauri IPC 的数据。
 * 涉及真实后端数据的验证请使用 L6 IPC 用例或 L4 CDP 用例。
 */

const PAGES: Array<{ nav: string; hash: string; label: string }> = [
  { nav: 'a[href="#/skills"]', hash: '/#/skills', label: 'Skill 管理' },
  { nav: 'a[href="#/agents"]', hash: '/#/agents', label: 'Agent' },
  { nav: 'a[href="#/sessions"]', hash: '/#/sessions', label: '会话管理' },
  { nav: 'a[href="#/commits"]', hash: '/#/commits', label: '代码提交' },
  { nav: 'a[href="#/cost"]', hash: '/#/cost', label: '成本分析' },
  { nav: 'a[href="#/statistics"]', hash: '/#/statistics', label: '统计分析' },
  { nav: 'a[href="#/audit-log"]', hash: '/#/audit-log', label: '审计日志' },
  { nav: 'a[href="#/settings"]', hash: '/#/settings', label: '设置' },
  { nav: 'a[href="#/system-monitor"]', hash: '/#/system-monitor', label: '系统监控' },
  { nav: 'a[href="#/import-export"]', hash: '/#/import-export', label: '数据导入导出' },
];

for (const { nav, hash, label } of PAGES) {
  test(`L2-页面覆盖: ${label} 路由可达且主内容区渲染`, async ({ page }) => {
    await page.goto(hash);
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(500);

    // 侧边导航入口存在且可见
    await expect(page.locator(nav)).toBeVisible();

    // 主内容区已渲染（非空白）
    const main = page.locator('main');
    await expect(main).toBeVisible();
    const text = await main.textContent();
    expect((text ?? '').trim().length).toBeGreaterThan(0);
  });
}
