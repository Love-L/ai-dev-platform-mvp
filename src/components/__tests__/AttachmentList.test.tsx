import { render } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import AttachmentList from '../AttachmentList';

describe('AttachmentList component', () => {
  it('renders empty state when no attachments', () => {
    const { container } = render(<AttachmentList attachments={[]} />);
    expect(container.textContent).toContain('暂无附件');
  });

  it('renders attachment items', () => {
    const { container } = render(
      <AttachmentList
        attachments={[
          { name: 'file1.txt', path: '/path/to/file1.txt' },
          { name: 'file2.pdf', path: '/path/to/file2.pdf' },
        ]}
      />
    );
    expect(container.textContent).toContain('file1.txt');
    expect(container.textContent).toContain('file2.pdf');
  });
});
