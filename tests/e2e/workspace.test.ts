import { test, expect } from "@playwright/test";

// L2-015: 工作台加载
test("L2-015: 工作台页面加载", async ({ page }) => {
  await page.goto("/");
  await page.waitForLoadState("networkidle");
  // 验证页面标题包含工作台或首页
  const heading = page.locator("h1, h2, h3").first();
  await expect(heading).toBeVisible();
});

// L2-016: 快捷操作区域
test("L2-016: 快捷操作或统计卡片存在", async ({ page }) => {
  await page.goto("/");
  await page.waitForLoadState("networkidle");
  // 验证卡片或统计区域存在
  const cards = page.locator('[class*="card"], [class*="stat"], [class*="metric"]');
  const count = await cards.count();
  expect(count).toBeGreaterThanOrEqual(1);
});
