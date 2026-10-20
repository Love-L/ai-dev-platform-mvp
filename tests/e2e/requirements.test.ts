import { test, expect } from '@playwright/test';

// L2-001: 需求管理页面加载
test('L2-001: 需求管理页面加载成功', async ({ page }) => {
  await page.goto('/#/requirements');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(1000);
  // 验证页面导航高亮
  const navLink = page.locator('a[href="#/requirements"]');
  await expect(navLink).toBeVisible();
  // 验证主内容区域加载
  const main = page.locator('main');
  await expect(main).toBeVisible();
});

// L2-002: 需求列表区域显示
test('L2-002: 需求列表区域存在', async ({ page }) => {
  await page.goto('/#/requirements');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(1000);
  // 验证主内容区域有内容
  const main = page.locator('main');
  const text = await main.textContent();
  expect(text?.length).toBeGreaterThan(0);
});

// L2-003: 需求页面导航验证
test('L2-003: 需求管理导航链接正确', async ({ page }) => {
  await page.goto('/#/');
  await page.waitForLoadState('networkidle');
  // 验证侧边栏有需求管理链接
  const navLink = page.locator('a[href="#/requirements"]');
  await expect(navLink).toBeVisible();
  await expect(navLink).toContainText('需求管理');
});
