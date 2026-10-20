import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import TaskForm from '../TaskForm';

describe('TaskForm (L1-007 ~ L1-012)', () => {
  const mockOnSubmit = vi.fn();
  const mockOnCancel = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockOnSubmit.mockResolvedValue(undefined);
  });

  // L1-007: 渲染所有11个字段
  it('L1-007: renders all form fields', () => {
    render(<TaskForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);

    expect(screen.getByPlaceholderText('输入任务标题')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('输入任务描述')).toBeInTheDocument();
    expect(screen.getByDisplayValue('开发')).toBeInTheDocument(); // task_type
    expect(screen.getByDisplayValue('P1 - 高')).toBeInTheDocument(); // priority
    expect(screen.getByPlaceholderText('执行人')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('0')).toBeInTheDocument(); // estimated_hours
    expect(screen.getByPlaceholderText('如果任务被阻塞，请说明原因')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('用逗号分隔多个标签')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('备注信息')).toBeInTheDocument();
  });

  // L1-008: 空标题提交被拦截
  it('L1-008: blocks submit when title is empty', () => {
    render(<TaskForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);
    const submitButton = screen.getByRole('button', { name: /保存/ });
    expect(submitButton).toBeDisabled();
  });

  // L1-009: 任务类型选择
  it('L1-009: allows selecting different task types', () => {
    render(<TaskForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);
    const typeSelect = screen.getByDisplayValue('开发');
    fireEvent.change(typeSelect, { target: { value: 'testing' } });
    expect(typeSelect).toHaveValue('testing');
  });

  // L1-010: 优先级选择
  it('L1-010: allows selecting different priorities', () => {
    render(<TaskForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);
    const prioritySelect = screen.getByDisplayValue('P1 - 高');
    fireEvent.change(prioritySelect, { target: { value: 'P0' } });
    expect(prioritySelect).toHaveValue('P0');
  });

  // L1-011: 合法提交调用onSubmit
  it('L1-011: calls onSubmit with form data when valid', async () => {
    render(<TaskForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);

    const titleInput = screen.getByPlaceholderText('输入任务标题');
    fireEvent.change(titleInput, { target: { value: '测试任务' } });

    const submitButton = screen.getByRole('button', { name: /保存/ });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(mockOnSubmit).toHaveBeenCalledWith(
        expect.objectContaining({ title: '测试任务', task_type: 'development' })
      );
    });
  });

  // L1-012: 工时字段输入数字
  it('L1-012: estimated_hours accepts numeric input', () => {
    render(<TaskForm onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);
    const hoursInput = screen.getByPlaceholderText('0');
    fireEvent.change(hoursInput, { target: { value: '8' } });
    expect(hoursInput).toHaveValue(8);
  });

  // 编辑模式
  it('pre-fills fields when editing existing task', () => {
    const existingTask = {
      id: '1',
      title: '已有任务',
      description: '任务描述',
      task_type: 'bug_fix' as const,
      priority: 'P0' as const,
      status: 'pending' as const,
      assignee: '张三',
      estimated_hours: 4,
      tags: 'bug,urgent',
      token_usage: 0,
      created_at: '2026-01-01',
      updated_at: '2026-01-01',
    };

    render(<TaskForm task={existingTask} onSubmit={mockOnSubmit} onCancel={mockOnCancel} />);

    expect(screen.getByDisplayValue('已有任务')).toBeInTheDocument();
    expect(screen.getByDisplayValue('任务描述')).toBeInTheDocument();
    expect(screen.getByDisplayValue('缺陷修复')).toBeInTheDocument();
    expect(screen.getByDisplayValue('P0 - 紧急')).toBeInTheDocument();
    expect(screen.getByDisplayValue('张三')).toBeInTheDocument();
  });
});
