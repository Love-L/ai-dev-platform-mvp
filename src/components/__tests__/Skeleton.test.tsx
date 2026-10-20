import { render } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { CardSkeleton, StatCardSkeleton } from '../Skeleton';

// Mock framer-motion
vi.mock('framer-motion', () => ({
  motion: {
    div: ({ children, ...props }: any) => <div {...props}>{children}</div>,
  },
}));

describe('Skeleton components', () => {
  it('CardSkeleton renders', () => {
    const { container } = render(<CardSkeleton />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it('StatCardSkeleton renders', () => {
    const { container } = render(<StatCardSkeleton />);
    expect(container.firstChild).toBeInTheDocument();
  });

  it('CardSkeleton contains animated structure', () => {
    const { container } = render(<CardSkeleton />);
    const pulseDivs = container.querySelectorAll('.animate-pulse');
    expect(pulseDivs.length).toBeGreaterThan(0);
  });
});
