import type { ReactElement } from 'react';
import { render, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, it, expect, vi, beforeEach } from 'vitest';

// 页面统一通过 `@tauri-apps/api/core` 的 invoke 访问后端，这里统一 mock。
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
// 部分页面订阅 Tauri 事件（如 RequirementDetail 的流式 PRD）
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
// xterm（终端）与 tiptap（富文本）在 jsdom 下依赖较重，且与本冒烟测试目标无关
vi.mock('../../components/AgentTerminal', () => ({ default: () => null }));
vi.mock('../../components/RichTextEditor', () => ({ default: () => null }));

import { invoke } from '@tauri-apps/api/core';
import Workspace from '../Workspace';
import Projects from '../Projects';
import ProjectDetail from '../ProjectDetail';
import Requirements from '../Requirements';
import RequirementDetail from '../RequirementDetail';
import Board from '../Board';
import TaskDetail from '../TaskDetail';
import SkillManagement from '../SkillManagement';
import SkillEvolution from '../SkillEvolution';
import Agents from '../Agents';
import Sessions from '../Sessions';
import SessionDetail from '../SessionDetail';
import Commits from '../Commits';
import Cost from '../Cost';
import Statistics from '../Statistics';
import AuditLog from '../AuditLog';
import ImportExport from '../ImportExport';
import SystemMonitor from '../SystemMonitor';
import Settings from '../Settings';
import Dashboard from '../Dashboard';

const invokeMock = vi.mocked(invoke);

/**
 * 命令默认返回值。
 *
 * 未列出的命令统一返回 `[]`（多数页面在挂载时会拉取列表），
 * 返回对象的命令在此显式声明，避免页面读取字段时出现 `undefined` 访问。
 */
const COMMAND_DEFAULTS: Record<string, unknown> = {
  // 详情类：无数据时页面展示空态
  get_setting: null,
  get_requirement: null,
  get_task: null,
  get_project: null,
  get_prd_skill: null,

  // 列表类分页结构（list_requirements 返回 PagedResult）
  list_requirements: { items: [], total: 0, page: 1, page_size: 20, total_pages: 0 },

  // 统计 / 成本
  get_statistics: {
    tasks: {
      total: 0,
      completed: 0,
      in_progress: 0,
      pending: 0,
      by_type: { development: 0, testing: 0, bug_fix: 0 },
    },
    requirements: { total: 0 },
    tokens: { total: 0, cost: 0, input: 0, output: 0 },
  },
  get_token_usage_summary: {
    total_tokens: 0,
    total_cost: 0,
    input_tokens: 0,
    output_tokens: 0,
    cache_tokens: 0,
    thinking_tokens: 0,
    record_count: 0,
  },
  get_token_budget: {
    id: 'budget-1',
    scope: 'global',
    monthly_limit: 2000000,
    alert_threshold: 0.8,
    current_usage: 0,
  },
  get_cache_stats: {
    hits: 0,
    misses: 0,
    evictions: 0,
    size: 0,
    l1_size: 0,
    hit_rate: 0,
  },

  // 系统监控
  get_monitor_data: {
    cpu_usage: 0,
    memory_used_mb: 0,
    memory_usage_percent: 0,
    uptime_seconds: 0,
    active_sessions: 0,
  },

  // Git
  git_get_status: { branch: 'main', files: [], total_changes: 0 },
  git_get_diff: '',
};

interface PageCase {
  name: string;
  entry: string;
  element: ReactElement;
}

/** 覆盖 App.tsx 注册的全部路由页面，以及未挂载的 Dashboard 孤儿页面 */
const PAGES: PageCase[] = [
  { name: 'Workspace', entry: '/', element: <Workspace /> },
  { name: 'Projects', entry: '/projects', element: <Projects /> },
  { name: 'ProjectDetail', entry: '/projects/proj-1', element: <ProjectDetail /> },
  { name: 'Requirements', entry: '/requirements', element: <Requirements /> },
  { name: 'RequirementDetail', entry: '/requirements/req-1', element: <RequirementDetail /> },
  { name: 'Board', entry: '/board', element: <Board /> },
  { name: 'TaskDetail', entry: '/tasks/task-1', element: <TaskDetail /> },
  { name: 'SkillManagement', entry: '/skills', element: <SkillManagement /> },
  { name: 'SkillEvolution', entry: '/skills/skill-1/evolution', element: <SkillEvolution /> },
  { name: 'Agents', entry: '/agents', element: <Agents /> },
  { name: 'Sessions', entry: '/sessions', element: <Sessions /> },
  { name: 'SessionDetail', entry: '/sessions/session-1', element: <SessionDetail /> },
  { name: 'Commits', entry: '/commits', element: <Commits /> },
  { name: 'Cost', entry: '/cost', element: <Cost /> },
  { name: 'Statistics', entry: '/statistics', element: <Statistics /> },
  { name: 'AuditLog', entry: '/audit-log', element: <AuditLog /> },
  { name: 'ImportExport', entry: '/import-export', element: <ImportExport /> },
  { name: 'SystemMonitor', entry: '/system-monitor', element: <SystemMonitor /> },
  { name: 'Settings', entry: '/settings', element: <Settings /> },
  { name: 'Dashboard（未挂载）', entry: '/dashboard', element: <Dashboard /> },
];

describe('页面渲染冒烟测试', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    invokeMock.mockImplementation(async (cmd: string) =>
      cmd in COMMAND_DEFAULTS ? COMMAND_DEFAULTS[cmd] : []
    );
  });

  PAGES.forEach(({ name, entry, element }) => {
    it(`${name} 可正常渲染且不抛错`, async () => {
      const { container } = render(
        <MemoryRouter initialEntries={[entry]}>
          <Routes>
            <Route path={entry} element={element} />
          </Routes>
        </MemoryRouter>
      );

      await waitFor(() => {
        expect(container.firstChild).not.toBeNull();
      });
    });
  });
});
