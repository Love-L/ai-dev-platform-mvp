/**
 * L4 CDP - WebView2 真实运行验证
 *
 * 前置条件（三步）：
 *   1. `src-tauri/Cargo.toml` 已启用 devtools feature（已启用）；
 *   2. 启动应用时暴露 CDP 端口：
 *        PowerShell: `$env:TAURI_DEVTOOLS_SERVER_PORT=9222; npm run tauri dev`
 *   3. 以同一端口运行本用例：
 *        `npm run test:l4`
 *
 * 未设置 `TAURI_DEVTOOLS_SERVER_PORT` 时，全部用例自动跳过（不影响常规流水线）。
 * 连接目标：`http://127.0.0.1:<port>`（Tauri devtools 暴露的 CDP 端点）。
 *
 * 注意：本文件被 `vitest.config.ts` 的 `exclude` 排除在默认 `npm test` 之外，
 * 需通过 `npm run test:l4` 显式运行。
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { chromium, type Browser, type Page } from "playwright-core";

const CDP_PORT = process.env.TAURI_DEVTOOLS_SERVER_PORT;
const CDP_ENDPOINT = CDP_PORT ? `http://127.0.0.1:${CDP_PORT}` : "";

describe.skipIf(!CDP_PORT)("L4 CDP: WebView2 真实运行验证", () => {
  let browser: Browser;
  let page: Page;

  beforeAll(async () => {
    browser = await chromium.connectOverCDP(CDP_ENDPOINT);
    const context = browser.contexts()[0];
    page = context.pages()[0] ?? (await context.newPage());
  });

  afterAll(async () => {
    // 只断开连接，不关闭被调试的应用
    await browser?.close();
  });

  it("L4-001: 控制台无 error 级日志", async () => {
    const errors: string[] = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(msg.text());
    });
    page.on("pageerror", (err) => errors.push(String(err)));

    await page.reload({ waitUntil: "domcontentloaded" });
    await page.waitForTimeout(1500);

    expect(errors, `控制台错误:\n${errors.join("\n")}`).toHaveLength(0);
  });

  it("L4-002: 主文档与静态资源请求均无 4xx/5xx", async () => {
    const failed: string[] = [];
    page.on("response", (res) => {
      if (res.status() >= 400) failed.push(`${res.status()} ${res.url()}`);
    });

    await page.reload({ waitUntil: "networkidle" });

    expect(failed, `失败请求:\n${failed.join("\n")}`).toHaveLength(0);
  });

  it("L4-003: 关键性能指标已产出", async () => {
    const metrics = await page.evaluate(() => {
      const nav = performance.getEntriesByType("navigation")[0] as PerformanceNavigationTiming | undefined;
      return {
        domContentLoaded: nav?.domContentLoadedEventEnd ?? 0,
        loadComplete: nav?.loadEventEnd ?? 0,
        resourceCount: performance.getEntriesByType("resource").length,
      };
    });

    expect(metrics.domContentLoaded).toBeGreaterThan(0);
    expect(metrics.loadComplete).toBeGreaterThanOrEqual(metrics.domContentLoaded);
    expect(metrics.resourceCount).toBeGreaterThan(0);
  });

  it("L4-004: 可拦截并统计 Tauri IPC 调用", async () => {
    const intercepted = await page.evaluate(async () => {
      // 通过包装 __TAURI_INTERNALS__.invoke 统计真实 IPC 调用
      const internals = (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__;
      if (!internals || typeof internals.invoke !== "function") return null;

      const calls: string[] = [];
      const original = internals.invoke as (cmd: string, args?: unknown) => Promise<unknown>;
      internals.invoke = (cmd: string, args?: unknown) => {
        calls.push(cmd);
        return original(cmd, args);
      };

      // 触发一次必然发生的 IPC（导航会触发统计/监控拉取）
      window.location.hash = "#/statistics";
      await new Promise((resolve) => setTimeout(resolve, 1200));

      internals.invoke = original;
      return calls;
    });

    if (intercepted === null) {
      // 未暴露 __TAURI_INTERNALS__ 说明当前不是 Tauri 运行时
      throw new Error("未检测到 Tauri 运行时：请通过 `npm run tauri dev` 启动应用后再运行 L4");
    }
    expect(intercepted.length).toBeGreaterThan(0);
  });

  it("L4-005: IPC 往返耗时可测量（< 2000ms）", async () => {
    const cost = await page.evaluate(async () => {
      const internals = (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__;
      if (!internals || typeof internals.invoke !== "function") return null;

      const invoke = internals.invoke as (cmd: string, args?: unknown) => Promise<unknown>;
      const started = performance.now();
      await invoke("get_statistics", {});
      return performance.now() - started;
    });

    if (cost === null) {
      throw new Error("未检测到 Tauri 运行时：请通过 `npm run tauri dev` 启动应用后再运行 L4");
    }
    expect(cost).toBeLessThan(2000);
  });
});
