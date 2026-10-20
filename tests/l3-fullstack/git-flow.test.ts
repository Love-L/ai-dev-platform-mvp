import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { gitLog, gitGetStatus, gitGetDiff } from '../../src/lib/tauri';

const mockInvoke = vi.mocked(invoke);

describe('L3 Full-stack: Git 操作全链路', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // L3-025: Git Log 查询
  it('L3-025: gitLog 返回提交列表', async () => {
    const mockCommits = [
      { hash: 'abc123', short_hash: 'abc123', message: 'feat: 添加新功能', author: 'dev', date: '2026-01-01' },
      { hash: 'def456', short_hash: 'def456', message: 'fix: 修复bug', author: 'dev', date: '2026-01-02' },
    ];
    mockInvoke.mockResolvedValue(mockCommits);

    const result = await gitLog('/path/to/repo', 10);

    expect(mockInvoke).toHaveBeenCalledWith('git_log', { workDir: '/path/to/repo', limit: 10 });
    expect(result).toHaveLength(2);
    expect(result[0].message).toContain('feat');
  });

  // L3-026: Git Status 查询
  it('L3-026: gitGetStatus 返回分支和文件状态', async () => {
    const mockStatus = {
      branch: 'main',
      files: [
        { status: 'M', path: 'src/main.rs' },
        { status: 'A', path: 'src/new.rs' },
      ],
      total_changes: 2,
    };
    mockInvoke.mockResolvedValue(mockStatus);

    const result = await gitGetStatus('/path/to/repo');

    expect(result.branch).toBe('main');
    expect(result.files).toHaveLength(2);
    expect(result.total_changes).toBe(2);
  });

  // L3-027: Git Diff 查询
  it('L3-027: gitGetDiff 返回差异内容', async () => {
    const mockDiff = '+ new line\n- old line';
    mockInvoke.mockResolvedValue(mockDiff);

    const result = await gitGetDiff('/path/to/repo', 'src/main.rs');

    expect(mockInvoke).toHaveBeenCalledWith('git_get_diff', {
      workDir: '/path/to/repo',
      filePath: 'src/main.rs',
    });
    expect(result).toContain('+ new line');
  });
});
