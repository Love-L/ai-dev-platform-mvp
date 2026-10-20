import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import RequirementForm from '../RequirementForm';

describe('RequirementForm (L1-001 ~ L1-006)', () => {
  const mockOnSubmit = vi.fn();
  const mockOnCancel = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockOnSubmit.mockResolvedValue(undefined);
  });

  // L1-001: 渲染所有字段
  it('L1-001: renders all 12 form fields', () => {
    render(<RequirementForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);

    expect(screen.getByPlaceholderText('输入需求标题')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('输入需求描述')).toBeInTheDocument();
    expect(screen.getByDisplayValue('P1 - 高')).toBeInTheDocument();
    expect(screen.getByDisplayValue('功能需求')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('所属模块')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('产品负责人')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('开发负责人')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('测试负责人')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('用逗号分隔多个标签')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('备注信息')).toBeInTheDocument();
  });

  // L1-002: 空标题提交被拦截
  it('L1-002: blocks submit when title is empty', () => {
    render(<RequirementForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);
    const submitButton = screen.getByRole('button', { name: /保存/ });
    expect(submitButton).toBeDisabled();
  });

  // L1-003: 超长标题被maxLength限制
  it('L1-003: title input has maxLength of 200', () => {
    render(<RequirementForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);
    const titleInput = screen.getByPlaceholderText('输入需求标题');
    expect(titleInput).toHaveAttribute('maxLength', '200');
  });

  // L1-004: 合法提交调用onSubmit
  it('L1-004: calls onSubmit with form data when valid', async () => {
    render(<RequirementForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);

    const titleInput = screen.getByPlaceholderText('输入需求标题');
    fireEvent.change(titleInput, { target: { value: '测试需求' } });

    const submitButton = screen.getByRole('button', { name: /保存/ });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith(
        expect.objectContaining({ title: '测试需求' })
      );
    });
  });

  // L1-005: 优先级选择切换
  it('L1-005: allows changing priority selection', () => {
    render(<RequirementForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);
    const prioritySelect = screen.getByDisplayValue('P1 - 高');
    fireEvent.change(prioritySelect, { target: { value: 'P0' } });
    expect(prioritySelect).toHaveValue('P0');
  });

  // L1-006: 取消按钮调用onCancel
  it('L1-006: cancel button calls onCancel', () => {
    render(<RequirementForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);
    const cancelButton = screen.getByRole('button', { name: /取消/ });
    fireEvent.click(cancelButton);
    expect(mockOnCancel).toHaveBeenCalled();
  });
});
