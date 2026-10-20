import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useSettingsStore } from '../settingsStore';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('useSettingsStore (L1-018, L1-035)', () => {
  beforeEach(() => {
    useSettingsStore.setState({ settings: {}, loading: false, error: null });
  });

  it('initializes with empty settings', () => {
    const state = useSettingsStore.getState();
    expect(state.settings).toEqual({});
  });

  it('fetchSetting loads a single key', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue('auto');

    await useSettingsStore.getState().fetchSetting('session_mode');

    const state = useSettingsStore.getState();
    expect(state.settings.session_mode).toBe('auto');
  });

  it('fetchSettings loads multiple keys', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke)
      .mockResolvedValueOnce('auto')
      .mockResolvedValueOnce('1024');

    await useSettingsStore.getState().fetchSettings(['session_mode', 'alert_memory_threshold']);

    const state = useSettingsStore.getState();
    expect(state.settings.session_mode).toBe('auto');
    expect(state.settings.alert_memory_threshold).toBe('1024');
  });

  it('saveSetting updates setting and cache', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockResolvedValue(undefined);

    await useSettingsStore.getState().saveSetting('session_mode', 'manual');

    const state = useSettingsStore.getState();
    expect(state.settings.session_mode).toBe('manual');
  });

  it('fetchSetting handles errors', async () => {
    const { invoke } = await import('@tauri-apps/api/core');
    vi.mocked(invoke).mockRejectedValue(new Error('not found'));

    await useSettingsStore.getState().fetchSetting('nonexistent');

    const state = useSettingsStore.getState();
    expect(state.error).toContain('not found');
  });
});
