import { test, expect } from "@playwright/test";

// L2-007: 导航到需求页面
test("L2-007: 导航到需求管理页面", async ({ page }) => {
  await page.goto("/#/");
  await page.waitForLoadState("networkidle");
  await page.locator('a[href="#/requirements"]').click();
  await page.waitForTimeout(500);
  await expect(page).toHaveURL(/requirements/);
  // 验证页面标题
  await expect(page.getByRole("heading", { name: "需求管理" })).toBeVisible();
});

// L2-008: 导航到看板页面
test("L2-008: 导航到看板页面", async ({ page }) => {
  await page.goto("/#/");
  await page.waitForLoadState("networkidle");
  await page.locator('a[href="#/board"]').click();
  await page.waitForTimeout(500);
  await expect(page).toHaveURL(/board/);
  await expect(page.getByRole("heading", { name: "任务看板" })).toBeVisible();
});

// L2-009: 导航到项目页面
test("L2-009: 导航到项目页面", async ({ page }) => {
  await page.goto("/#/");
  await page.waitForLoadState("networkidle");
  await page.locator('a[href="#/projects"]').click();
  await page.waitForTimeout(500);
  await expect(page).toHaveURL(/projects/);
});

// L2-010: 页面初始加载
test("L2-010: 首页加载显示工作台", async ({ page }) => {
  await page.goto("/#/");
  await page.waitForLoadState("networkidle");
  await expect(page.getByRole("heading", { name: "工作台" })).toBeVisible();
  await expect(page.locator("text=项目概览和快速操作")).toBeVisible();
});
