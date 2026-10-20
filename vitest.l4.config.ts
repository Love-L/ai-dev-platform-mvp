import { defineConfig } from "vitest/config";

/**
 * L4 CDP 测试专用配置。
 *
 * L4 用例需要连接**正在运行**的 Tauri 应用（devtools CDP 端点），
 * 因此被主 `vitest.config.ts` 的 `exclude` 排除，需单独运行：
 *
 *   PowerShell: `$env:TAURI_DEVTOOLS_SERVER_PORT=9222; npm run tauri dev`
 *   另开终端:   `npm run test:l4`
 *
 * 未设置 `TAURI_DEVTOOLS_SERVER_PORT` 时用例自动跳过。
 */
export default defineConfig({
  test: {
    globals: true,
    // 用例通过 CDP 与真实应用通信，使用 node 环境
    environment: "node",
    include: ["tests/l4-cdp/**/*.test.ts"],
    testTimeout: 30000,
    hookTimeout: 30000,
    reporters: ["verbose"],
  },
});
