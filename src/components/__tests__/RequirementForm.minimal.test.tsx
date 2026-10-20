import { render } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import RequirementForm from '../RequirementForm';

describe('RequirementForm minimal', () => {
  it('renders', () => {
    const { container } = render(
      <RequirementForm onSubmit={vi.fn()} onCancel={vi.fn()} />
    );
    expect(container).toBeTruthy();
  });
});
