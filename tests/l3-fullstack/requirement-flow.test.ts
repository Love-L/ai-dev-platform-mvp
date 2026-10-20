import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock Tauri IPC
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import {
  createRequirement,
  listRequirements,
  updateRequirement,
  deleteRequirement,
  generatePrd,
} from '../../src/lib/tauri';

const mockInvoke = vi.mocked(invoke);

describe('L3 Full-stack: 需求管理全链路', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // L3-001: 创建需求全链路
  it('L3-001: 创建需求返回完整对象', async () => {
    const mockResult = {
      id: 'req-001',
      title: '测试需求',
      description: '需求描述',
      priority: 'P1',
      status: 'draft',
      created_at: '2026-01-01T00:00:00Z',
    };
    mockInvoke.mockResolvedValue(mockResult);

    const result = await createRequirement('测试需求', '需求描述', 'P1');

    expect(mockInvoke).toHaveBeenCalledWith('create_requirement', {
      title: '测试需求',
      description: '需求描述',
      priority: 'P1',
      module: undefined,
      source: undefined,
      sourceDetail: undefined,
      productOwner: undefined,
      developer: undefined,
      tester: undefined,
      businessPriority: undefined,
      estimatedEffort: undefined,
      tags: undefined,
      notes: undefined,
    });
    expect(result).toEqual(mockResult);
    expect(result.id).toBe('req-001');
  });

  // L3-002: 查询需求列表（PagedResult 解析）
  it('L3-002: listRequirements 解析 PagedResult', async () => {
    const mockPagedResult = {
      items: [
        { id: 'req-001', title: '需求1' },
        { id: 'req-002', title: '需求2' },
      ],
      total: 2,
      page: 1,
      page_size: 20,
    };
    mockInvoke.mockResolvedValue(mockPagedResult);

    const result = await listRequirements();

    expect(mockInvoke).toHaveBeenCalledWith('list_requirements', { status: undefined });
    expect(result).toHaveLength(2);
    expect(result[0].id).toBe('req-001');
  });

  // L3-003: 更新需求状态
  it('L3-003: updateRequirement 调用正确 IPC', async () => {
    mockInvoke.mockResolvedValue(undefined);

    await updateRequirement('req-001', { status: 'in_development', title: '更新标题' });

    expect(mockInvoke).toHaveBeenCalledWith('update_requirement', {
      id: 'req-001',
      status: 'in_development',
      title: '更新标题',
    });
  });

  // L3-004: 删除需求
  it('L3-004: deleteRequirement 调用正确 IPC', async () => {
    mockInvoke.mockResolvedValue(undefined);

    await deleteRequirement('req-001');

    expect(mockInvoke).toHaveBeenCalledWith('delete_requirement', { id: 'req-001' });
  });

  // L3-005: 生成 PRD
  it('L3-005: generatePrd 返回内容', async () => {
    mockInvoke.mockResolvedValue('# PRD 文档\n\n这是生成的PRD内容');

    const result = await generatePrd('req-001', 'skill-001');

    expect(mockInvoke).toHaveBeenCalledWith('generate_prd', {
      requirementId: 'req-001',
      skillId: 'skill-001',
    });
    expect(result).toContain('PRD 文档');
  });
});
