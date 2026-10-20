import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

describe("L5 Chaos: Store 故障注入", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // L5-001: IPC 调用超时
  it("L5-001: invoke 超时应抛出异常", async () => {
    mockInvoke.mockImplementation(
      () => new Promise((_, reject) => setTimeout(() => reject(new Error("timeout")), 100)),
    );

    await expect(import("../../src/lib/tauri").then((m) => m.listRequirements())).rejects.toThrow("timeout");
  });

  // L5-002: IPC 返回 null
  it("L5-002: invoke 返回 null 不崩溃", async () => {
    mockInvoke.mockResolvedValue(null);

    const { getRequirement } = await import("../../src/lib/tauri");
    const result = await getRequirement("non-existent");
    expect(result).toBeNull();
  });

  // L5-003: IPC 返回空数组
  it("L5-003: invoke 返回空数组正常处理", async () => {
    mockInvoke.mockResolvedValue({ items: [], total: 0 });

    const { listRequirements } = await import("../../src/lib/tauri");
    const result = await listRequirements();
    expect(result).toEqual([]);
  });

  // L5-004: IPC 返回格式异常
  it("L5-004: PagedResult 缺少 items 字段", async () => {
    mockInvoke.mockResolvedValue({ total: 0 }); // 缺少 items

    const { listRequirements } = await import("../../src/lib/tauri");
    const result = await listRequirements();
    // 应该回退到 result 本身
    expect(result).toBeDefined();
  });

  // L5-005: 网络错误模拟
  it("L5-005: invoke 抛出网络错误", async () => {
    mockInvoke.mockRejectedValue(new Error("Network Error"));

    const { createRequirement } = await import("../../src/lib/tauri");
    await expect(createRequirement("test", "desc")).rejects.toThrow("Network Error");
  });

  // L5-006: 并发调用同一 IPC
  it("L5-006: 并发 invoke 调用不互相干扰", async () => {
    mockInvoke
      .mockResolvedValueOnce({ items: [{ id: 1 }], total: 1 })
      .mockResolvedValueOnce({ items: [{ id: 2 }], total: 1 });

    const { listRequirements } = await import("../../src/lib/tauri");
    const [r1, r2] = await Promise.all([listRequirements(), listRequirements()]);

    expect(r1[0].id).toBe(1);
    expect(r2[0].id).toBe(2);
  });

  // L5-007: 大数据量响应
  it("L5-007: 大量数据不导致内存溢出", async () => {
    const largeArray = Array.from({ length: 10000 }, (_, i) => ({
      id: `item-${i}`,
      title: `Item ${i}`,
    }));
    mockInvoke.mockResolvedValue(largeArray);

    const { listTasks } = await import("../../src/lib/tauri");
    const result = await listTasks();
    expect(result).toHaveLength(10000);
  });

  // L5-008: 特殊字符输入
  it("L5-008: 特殊字符输入正常处理", async () => {
    const xssTitle = '<script>alert("xss")</script>';
    mockInvoke.mockResolvedValue({ id: "req-001", title: xssTitle });

    const { createRequirement } = await import("../../src/lib/tauri");
    const result = await createRequirement(xssTitle);
    expect(result.title).toBe(xssTitle);
  });

  // L5-009: Unicode 字符
  it("L5-009: Unicode 字符正常处理", async () => {
    const unicodeTitle = "日本語テスト 🎉 émojis";
    mockInvoke.mockResolvedValue({ id: "req-001", title: unicodeTitle });

    const { createRequirement } = await import("../../src/lib/tauri");
    const result = await createRequirement(unicodeTitle);
    expect(result.title).toBe(unicodeTitle);
  });

  // L5-010: 空字符串输入
  it("L5-010: 空字符串输入正常处理", async () => {
    mockInvoke.mockResolvedValue(undefined);

    const { setSetting } = await import("../../src/lib/tauri");
    await expect(setSetting("key", "")).resolves.toBeUndefined();
  });
});
