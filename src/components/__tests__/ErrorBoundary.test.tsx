import type { ReactElement } from 'react';
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import ErrorBoundary from '../ErrorBoundary';

// Suppress console.error for expected errors
vi.spyOn(console, 'error').mockImplementation(() => {});

// 声明返回类型为 ReactElement，以满足 JSX 组件签名（实现中直接抛出，不会真正返回）
function ThrowingComponent(): ReactElement {
  throw new Error('Test error');
}

describe('ErrorBoundary', () => {
  it('renders children when no error', () => {
    render(
      <ErrorBoundary>
        <div>Normal content</div>
      </ErrorBoundary>
    );
    expect(screen.getByText('Normal content')).toBeInTheDocument();
  });

  it('renders error UI when child throws', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>
    );
    // ErrorBoundary should catch and display error UI
    expect(screen.getByText(/页面出现错误/)).toBeInTheDocument();
  });

  it('renders retry button', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent />
      </ErrorBoundary>
    );
    expect(screen.getByRole('button', { name: /重试/ })).toBeInTheDocument();
  });
});
