/**
 * Toast组件单元测试
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import ToastContainer, { showToast } from '../Toast';

// Mock timers
beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('ToastContainer', () => {
  it('renders empty when no toasts', () => {
    const { container } = render(<ToastContainer />);
    expect(container.firstChild).toBeNull();
  });

  it('shows toast when showToast is called', () => {
    render(<ToastContainer />);

    act(() => {
      showToast('success', '操作成功');
    });

    expect(screen.getByText('操作成功')).toBeInTheDocument();
  });

  it('shows error toast', () => {
    render(<ToastContainer />);

    act(() => {
      showToast('error', '操作失败');
    });

    expect(screen.getByText('操作失败')).toBeInTheDocument();
  });

  it('auto-removes toast after 3 seconds', () => {
    render(<ToastContainer />);

    act(() => {
      showToast('success', '自动消失');
    });

    expect(screen.getByText('自动消失')).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(3000);
    });

    expect(screen.queryByText('自动消失')).not.toBeInTheDocument();
  });

  it('can show multiple toasts', () => {
    render(<ToastContainer />);

    act(() => {
      showToast('success', '第一个');
      showToast('error', '第二个');
    });

    expect(screen.getByText('第一个')).toBeInTheDocument();
    expect(screen.getByText('第二个')).toBeInTheDocument();
  });
});
