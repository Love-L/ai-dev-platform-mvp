import { describe, it, expect, beforeEach, vi } from "vitest";
import { useThemeStore } from "../themeStore";

/** jsdom 未实现 matchMedia，这里提供可控实现 */
function mockMatchMedia(matches: boolean) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

describe("useThemeStore (F072)", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove("dark");
    useThemeStore.setState({ theme: "light" });
    mockMatchMedia(false);
  });

  it("默认主题为 light", () => {
    expect(useThemeStore.getState().theme).toBe("light");
  });

  it('setTheme("dark") 添加 html.dark 并持久化到 ace-theme', () => {
    useThemeStore.getState().setTheme("dark");

    expect(useThemeStore.getState().theme).toBe("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(localStorage.getItem("ace-theme") ?? "").toContain("dark");
  });

  it('setTheme("light") 移除 html.dark', () => {
    useThemeStore.getState().setTheme("dark");
    useThemeStore.getState().setTheme("light");

    expect(useThemeStore.getState().theme).toBe("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("system 模式跟随系统深色偏好", () => {
    mockMatchMedia(true);
    useThemeStore.getState().setTheme("system");

    expect(useThemeStore.getState().theme).toBe("system");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("system 模式在系统为浅色时不添加 html.dark", () => {
    mockMatchMedia(false);
    useThemeStore.getState().setTheme("system");

    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });
});
