import { test, expect } from "@playwright/test";

// L2-017: 状态栏显示
test("L2-017: 状态栏区域存在", async ({ page }) => {
  await page.goto("/#/");
  await page.waitForLoadState("networkidle");
  // 验证状态栏包含就绪文字
  await expect(page.locator("text=就绪").first()).toBeVisible();
});

// L2-018: 状态栏信息
test("L2-018: 状态栏包含版本号", async ({ page }) => {
  await page.goto("/#/");
  await page.waitForLoadState("networkidle");
  // 验证版本号显示在状态栏
  await expect(page.locator('span.font-mono:has-text("v2.3.1")')).toBeVisible();
});
