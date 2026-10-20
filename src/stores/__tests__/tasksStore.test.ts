import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useTasksStore } from '../tasksStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('useTasksStore (L1-011)', () => {
  beforeEach(() => {
    useTasksStore.setState({ tasks: [], loading: false, error: null });
  });

  it('initializes with empty state', () => {
    const state = useTasksStore.getState();
    expect(state.tasks).toEqual([]);
    expect(state.loading).toBe(false);
  });

  it('fetchTasks loads tasks', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue([{ id: '1', title: '任务1' }]);

    await useTasksStore.getState().fetchTasks();

    const state = useTasksStore.getState();
    expect(state.tasks).toHaveLength(1);
    expect(state.loading).toBe(false);
  });

  it('fetchTasks handles errors', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockRejectedValue(new Error('DB error'));

    await useTasksStore.getState().fetchTasks();

    const state = useTasksStore.getState();
    expect(state.error).toContain('DB error');
  });

  it('addTask creates and refreshes', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke)
      .mockResolvedValueOnce({ id: '2', title: '新任务' })
      .mockResolvedValueOnce([{ id: '2', title: '新任务' }]);

    const result = await useTasksStore.getState().addTask('新任务');
    expect(result).toEqual({ id: '2', title: '新任务' });
  });

  it('updateStatus calls IPC and refreshes', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce([]);

    await useTasksStore.getState().updateStatus('1', 'completed');
    expect(invoke).toHaveBeenCalledWith('update_task_status', { id: '1', status: 'completed' });
  });
});
