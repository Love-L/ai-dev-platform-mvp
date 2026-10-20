import { test, expect } from "@playwright/test";

// L2-004: 任务看板加载
test("L2-004: 任务看板页面加载", async ({ page }) => {
  await page.goto("/#/board");
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(1000);
  // 验证导航链接
  const navLink = page.locator('a[href="#/board"]');
  await expect(navLink).toBeVisible();
  // 验证主内容区域
  const main = page.locator("main");
  await expect(main).toBeVisible();
});

// L2-005: 任务看板导航验证
test("L2-005: 任务看板导航链接正确", async ({ page }) => {
  await page.goto("/#/");
  await page.waitForLoadState("networkidle");
  const navLink = page.locator('a[href="#/board"]');
  await expect(navLink).toBeVisible();
  await expect(navLink).toContainText("任务看板");
});

// L2-006: 任务看板主内容
test("L2-006: 任务看板主内容区域有内容", async ({ page }) => {
  await page.goto("/#/board");
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(1000);
  const main = page.locator("main");
  const text = await main.textContent();
  expect(text?.length).toBeGreaterThan(0);
});
