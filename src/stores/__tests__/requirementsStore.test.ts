import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useRequirementsStore } from '../requirementsStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('useRequirementsStore (L1-006)', () => {
  beforeEach(() => {
    useRequirementsStore.setState({ requirements: [], loading: false, error: null });
  });

  it('initializes with empty state', () => {
    const state = useRequirementsStore.getState();
    expect(state.requirements).toEqual([]);
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('fetchRequirements sets loading and data', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue({
      items: [{ id: '1', title: '需求1' }],
      total: 1,
    });

    const store = useRequirementsStore.getState();
    await store.fetchRequirements();

    const state = useRequirementsStore.getState();
    expect(state.loading).toBe(false);
    expect(state.requirements).toHaveLength(1);
  });

  it('fetchRequirements handles errors', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockRejectedValue(new Error('IPC error'));

    const store = useRequirementsStore.getState();
    await store.fetchRequirements();

    const state = useRequirementsStore.getState();
    expect(state.loading).toBe(false);
    expect(state.error).toContain('IPC error');
  });

  it('addRequirement creates and refreshes', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke)
      .mockResolvedValueOnce({ id: '2', title: '新需求' })
      .mockResolvedValueOnce({ items: [{ id: '2', title: '新需求' }], total: 1 });

    const store = useRequirementsStore.getState();
    const result = await store.addRequirement('新需求');

    expect(result).toEqual({ id: '2', title: '新需求' });
  });
});
