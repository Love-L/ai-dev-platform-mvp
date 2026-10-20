import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import Commits from '../Commits';
import { useAgentsStore } from '../../stores/agentsStore';

const invokeMock = vi.mocked(invoke);

const TASKS = [
  { id: 'task-1', title: '实现登录接口', status: 'pending', task_type: 'development', priority: 'P1', token_usage: 0 },
];
const SESSIONS = [
  {
    id: 'session-1',
    agent_id: 'agent-1',
    status: 'running',
    started_at: '2026-09-21T10:00:00Z',
    title: '登录接口开发会话',
  },
];

const PLATFORM_COMMITS = [
  {
    id: 'commit-1',
    task_id: 'task-1',
    session_id: 'session-1',
    commit_hash: 'abcdef1234567890',
    short_hash: 'abcdef1',
    branch_name: 'main',
    message: 'feat: 登录接口',
    files_changed: 3,
    created_at: '2026-09-21T11:00:00Z',
  },
];

function mockBackend() {
  invokeMock.mockImplementation(async (cmd: string, args?: unknown) => {
    const params = args as Record<string, unknown> | undefined;
    switch (cmd) {
      case 'get_setting':
        return params?.key === 'project_path' ? '/tmp/repo' : null;
      case 'git_log':
        return [
          {
            hash: 'abcdef1234567890',
            short_hash: 'abcdef1',
            message: 'feat: 登录接口',
            author: 'dev',
            date: '2026-09-21T11:00:00Z',
          },
        ];
      case 'git_get_status':
        return { branch: 'main', files: [], total_changes: 2 };
      case 'list_tasks':
        return TASKS;
      case 'get_agent_sessions':
        return SESSIONS;
      case 'list_git_commits':
        return PLATFORM_COMMITS;
      case 'git_create_commit':
        return { commit_hash: 'deadbeef', short_hash: 'deadbee', files_changed: 3 };
      default:
        return [];
    }
  });
}

describe('Commits 页面（F060 提交记录落库与追溯）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockBackend();
  });

  it('F060-01: 渲染平台提交记录并展示任务/会话关联', async () => {
    render(
      <MemoryRouter>
        <Commits />
      </MemoryRouter>
    );

    expect(await screen.findByText('平台提交记录（可按任务/会话追溯）')).toBeInTheDocument();
    expect(await screen.findByText('任务: 实现登录接口')).toBeInTheDocument();
    // 会话以 8 位短 id 展示（真实场景为 UUID）
    expect(await screen.findByText(/会话: session-/)).toBeInTheDocument();
  });

  it('F060-02: 提交时携带所选任务与会话，建立可追溯关系', async () => {
    render(
      <MemoryRouter>
        <Commits />
      </MemoryRouter>
    );

    // 存在未提交变更时才会出现提交表单
    expect(await screen.findByPlaceholderText('输入提交信息...')).toBeInTheDocument();

    fireEvent.change(screen.getByTitle('将本次提交关联到任务，便于后续追溯'), {
      target: { value: 'task-1' },
    });
    fireEvent.change(screen.getByTitle('将本次提交关联到 Agent 会话'), {
      target: { value: 'session-1' },
    });
    fireEvent.change(screen.getByPlaceholderText('输入提交信息...'), {
      target: { value: 'feat: 登录接口' },
    });
    fireEvent.click(screen.getByRole('button', { name: /提交/ }));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        'git_create_commit',
        expect.objectContaining({
          workDir: '/tmp/repo',
          message: 'feat: 登录接口',
          taskId: 'task-1',
          sessionId: 'session-1',
        })
      );
    });
  });

  it('F060-03: 未选择关联对象时不传 taskId/sessionId', async () => {
    render(
      <MemoryRouter>
        <Commits />
      </MemoryRouter>
    );

    const messageInput = await screen.findByPlaceholderText('输入提交信息...');
    fireEvent.change(messageInput, { target: { value: 'chore: 无关联提交' } });
    fireEvent.click(screen.getByRole('button', { name: /提交/ }));

    await waitFor(() => {
      const call = invokeMock.mock.calls.find(([cmd]) => cmd === 'git_create_commit');
      expect(call).toBeDefined();
      expect(call?.[1]).toMatchObject({ taskId: undefined, sessionId: undefined });
    });
  });
});

describe('agentsStore 状态治理（F021）', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAgentsStore.setState({ agents: [], sessions: [], loading: false, refreshing: false, error: null });
  });

  it('F021-01: verify() 调用 verify_agents 并刷新列表', async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === 'verify_agents') {
        return [
          {
            id: 'agent-1',
            name: 'claude',
            display_name: 'Claude Code',
            binary_path: '/usr/bin/claude',
            status: 'unavailable',
            created_at: '2026-09-21T10:00:00Z',
          },
        ];
      }
      return [];
    });

    await useAgentsStore.getState().verify();

    expect(invokeMock).toHaveBeenCalledWith('verify_agents');
    expect(useAgentsStore.getState().agents[0]?.status).toBe('unavailable');
  });
});
