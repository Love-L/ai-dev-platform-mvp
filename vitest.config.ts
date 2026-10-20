import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // 通过 subst 等方式把项目映射为 ASCII 盘符运行时，避免 realpath 把
  // 模块 id 还原成含中文的原始路径，从而导致 setupFiles 加载失败。
  resolve: {
    preserveSymlinks: true,
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    css: false,
    testTimeout: 10000,
    hookTimeout: 10000,
    // 必须保持文件级隔离：`isolate: false` 会让某个测试文件对
    // `@tauri-apps/api/core` 的 mock 泄漏到其它文件，导致
    // "Cannot read properties of undefined (reading 'items')" 等假失败。
    isolate: true,
    reporters: ["verbose"],
    /**
     * 只收集 Vitest 可执行的用例，显式排除 Playwright / CDP 用例，
     * 避免 `npm test` 误收集 e2e 文件导致"文件内无用例"报错。
     *
     * 注意：Windows 下若项目路径包含中文或空格，vitest 1.6 会出现
     * "No test suite found in file"（用例全部不注册）。此为该版本在
     * 非 ASCII 路径下的已知问题，可先将项目路径映射为纯 ASCII 盘符后运行，
     * 例如：`subst X: "<project>" && cd /d X:\ && npm run test:l1`。
     */
    include: [
      "src/**/*.{test,spec}.{ts,tsx}",
      "tests/l3-fullstack/**/*.test.ts",
      "tests/l5-chaos/**/*.test.ts",
      "tests/l6-ipc/**/*.test.ts",
    ],
    exclude: ["**/node_modules/**", "**/dist/**", "**/target/**", "tests/e2e/**", "tests/l4-cdp/**"],
  },
});
