import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5174,
    strictPort: true,
    watch: {
      // 排除 Rust 构建产物：tauri dev 并发编译时 cargo 会锁定 target/ 下的文件，
      // 监视器扫描到被锁文件会抛 EBUSY 导致 dev server 崩溃（黑屏根因之一）
      ignored: ["**/src-tauri/target/**", "**/node_modules/**", "**/dist/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: process.env.TAURI_PLATFORM == "windows" ? "chrome105" : "safari14",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
}));
