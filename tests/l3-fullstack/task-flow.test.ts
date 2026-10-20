import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  createTask,
  listTasks,
  updateTaskStatus,
  deleteTask,
  createTaskComment,
  listTaskComments,
} from "../../src/lib/tauri";

const mockInvoke = vi.mocked(invoke);

describe("L3 Full-stack: 任务管理全链路", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // L3-006: 创建任务
  it("L3-006: createTask 返回完整对象", async () => {
    const mockResult = {
      id: "task-001",
      title: "测试任务",
      task_type: "development",
      priority: "P1",
      status: "todo",
    };
    mockInvoke.mockResolvedValue(mockResult);

    const result = await createTask("测试任务", undefined, "development", "P1");

    expect(mockInvoke).toHaveBeenCalledWith("create_task", {
      title: "测试任务",
      requirementId: undefined,
      taskType: "development",
      priority: "P1",
      assignee: undefined,
      dueDate: undefined,
      estimatedHours: undefined,
      priorityLevel: undefined,
      blockedReason: undefined,
      tags: undefined,
      notes: undefined,
    });
    expect(result.id).toBe("task-001");
  });

  // L3-007: 查询任务列表
  it("L3-007: listTasks 返回数组", async () => {
    mockInvoke.mockResolvedValue([
      { id: "task-001", title: "任务1", status: "todo" },
      { id: "task-002", title: "任务2", status: "doing" },
    ]);

    const result = await listTasks();

    expect(result).toHaveLength(2);
    expect(result[0].status).toBe("todo");
  });

  // L3-008: 更新任务状态
  it("L3-008: updateTaskStatus 调用正确", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await updateTaskStatus("task-001", "done");

    expect(mockInvoke).toHaveBeenCalledWith("update_task_status", {
      id: "task-001",
      status: "done",
    });
  });

  // L3-009: 任务状态流转完整链路
  it("L3-009: 任务状态 todo -> doing -> done", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await updateTaskStatus("task-001", "doing");
    await updateTaskStatus("task-001", "done");

    expect(mockInvoke).toHaveBeenCalledTimes(2);
    expect(mockInvoke).toHaveBeenNthCalledWith(1, "update_task_status", {
      id: "task-001",
      status: "doing",
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "update_task_status", {
      id: "task-001",
      status: "done",
    });
  });

  // L3-010: 创建任务评论
  it("L3-010: createTaskComment 返回评论对象", async () => {
    const mockComment = {
      id: "comment-001",
      task_id: "task-001",
      author: "测试者",
      content: "评论内容",
      created_at: "2026-01-01T00:00:00Z",
    };
    mockInvoke.mockResolvedValue(mockComment);

    const result = await createTaskComment("task-001", "测试者", "评论内容");

    expect(mockInvoke).toHaveBeenCalledWith("create_task_comment", {
      taskId: "task-001",
      author: "测试者",
      content: "评论内容",
    });
    expect(result.id).toBe("comment-001");
  });

  // L3-011: 查询任务评论列表
  it("L3-011: listTaskComments 返回数组", async () => {
    mockInvoke.mockResolvedValue([
      { id: "comment-001", content: "评论1" },
      { id: "comment-002", content: "评论2" },
    ]);

    const result = await listTaskComments("task-001");

    expect(result).toHaveLength(2);
  });

  // L3-012: 删除任务
  it("L3-012: deleteTask 调用正确", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await deleteTask("task-001");

    expect(mockInvoke).toHaveBeenCalledWith("delete_task", { id: "task-001" });
  });
});
