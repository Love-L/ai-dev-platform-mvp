import { render } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import Layout from "../Layout";

vi.mock("react-router-dom", () => ({
  Outlet: () => null,
  NavLink: ({ children }: any) => children,
  useLocation: () => ({ pathname: "/" }),
  useNavigate: () => vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({
    cpu_usage: 5.0,
    memory_used_mb: 200,
    memory_usage_percent: 25,
    active_sessions: 1,
    uptime_seconds: 3600,
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(() => Promise.resolve()),
}));

vi.mock("../../stores/themeStore", () => ({
  useThemeStore: () => ({ theme: "dark", toggleTheme: vi.fn() }),
}));

describe("Layout component", () => {
  it("renders without crashing", () => {
    const { container } = render(<Layout />);
    expect(container).toBeTruthy();
  });
});
