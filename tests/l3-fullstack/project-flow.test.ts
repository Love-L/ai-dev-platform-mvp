import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import {
  createProject,
  listProjects,
  getProject,
  updateProject,
  deleteProject,
  addProjectSkill,
  removeProjectSkill,
} from '../../src/lib/tauri';

const mockInvoke = vi.mocked(invoke);

describe('L3 Full-stack: 项目管理全链路', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // L3-013: 创建项目
  it('L3-013: createProject 返回完整对象', async () => {
    const mockResult = {
      id: 'proj-001',
      name: '测试项目',
      description: '项目描述',
      key: 'TP',
      status: 'active',
    };
    mockInvoke.mockResolvedValue(mockResult);

    const result = await createProject('测试项目', '项目描述', 'TP');

    expect(mockInvoke).toHaveBeenCalledWith('create_project', {
      name: '测试项目',
      description: '项目描述',
      key: 'TP',
    });
    expect(result.id).toBe('proj-001');
  });

  // L3-014: 查询项目列表
  it('L3-014: listProjects 返回数组', async () => {
    mockInvoke.mockResolvedValue([
      { id: 'proj-001', name: '项目1' },
      { id: 'proj-002', name: '项目2' },
    ]);

    const result = await listProjects();

    expect(result).toHaveLength(2);
  });

  // L3-015: 获取单个项目
  it('L3-015: getProject 返回项目详情', async () => {
    mockInvoke.mockResolvedValue({ id: 'proj-001', name: '测试项目' });

    const result = await getProject('proj-001');

    expect(mockInvoke).toHaveBeenCalledWith('get_project', { id: 'proj-001' });
    expect(result?.name).toBe('测试项目');
  });

  // L3-016: 更新项目
  it('L3-016: updateProject 调用正确', async () => {
    mockInvoke.mockResolvedValue(undefined);

    await updateProject('proj-001', { name: '新名称', status: 'archived' });

    expect(mockInvoke).toHaveBeenCalledWith('update_project', {
      id: 'proj-001',
      name: '新名称',
      status: 'archived',
    });
  });

  // L3-017: 删除项目
  it('L3-017: deleteProject 调用正确', async () => {
    mockInvoke.mockResolvedValue(undefined);

    await deleteProject('proj-001');

    expect(mockInvoke).toHaveBeenCalledWith('delete_project', { id: 'proj-001' });
  });

  // L3-018: 项目 Skill 配置
  it('L3-018: addProjectSkill 返回配置对象', async () => {
    const mockResult = {
      id: 'ps-001',
      project_id: 'proj-001',
      skill_id: 'skill-001',
      is_default: true,
    };
    mockInvoke.mockResolvedValue(mockResult);

    const result = await addProjectSkill('proj-001', 'skill-001', true);

    expect(mockInvoke).toHaveBeenCalledWith('add_project_skill', {
      projectId: 'proj-001',
      skillId: 'skill-001',
      isDefault: true,
      customParams: undefined,
    });
    expect(result.id).toBe('ps-001');
  });

  // L3-019: 移除项目 Skill
  it('L3-019: removeProjectSkill 调用正确', async () => {
    mockInvoke.mockResolvedValue(undefined);

    await removeProjectSkill('ps-001');

    expect(mockInvoke).toHaveBeenCalledWith('remove_project_skill', { id: 'ps-001' });
  });
});
