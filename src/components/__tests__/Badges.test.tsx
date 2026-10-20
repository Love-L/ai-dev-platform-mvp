/**
 * Badges组件单元测试
 */
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Badge, StatusBadge, PriorityBadge, TaskTypeBadge, ReqTypeBadge } from '../Badges';

describe('Badge', () => {
  it('renders with default variant', () => {
    render(<Badge>Test Badge</Badge>);
    const badge = screen.getByText('Test Badge');
    expect(badge).toBeInTheDocument();
  });

  it('renders with success variant', () => {
    render(<Badge variant="success">Success</Badge>);
    const badge = screen.getByText('Success');
    expect(badge).toBeInTheDocument();
  });

  it('renders with danger variant', () => {
    render(<Badge variant="danger">Danger</Badge>);
    const badge = screen.getByText('Danger');
    expect(badge).toBeInTheDocument();
  });

  it('renders with warning variant', () => {
    render(<Badge variant="warning">Warning</Badge>);
    const badge = screen.getByText('Warning');
    expect(badge).toBeInTheDocument();
  });

  it('renders with info variant', () => {
    render(<Badge variant="info">Info</Badge>);
    const badge = screen.getByText('Info');
    expect(badge).toBeInTheDocument();
  });

  it('renders with purple variant', () => {
    render(<Badge variant="purple">Purple</Badge>);
    const badge = screen.getByText('Purple');
    expect(badge).toBeInTheDocument();
  });

  it('renders with dot indicator', () => {
    render(<Badge dot>With Dot</Badge>);
    const badge = screen.getByText('With Dot');
    expect(badge).toBeInTheDocument();
  });

  it('renders with small size', () => {
    render(<Badge size="sm">Small</Badge>);
    const badge = screen.getByText('Small');
    expect(badge).toBeInTheDocument();
  });
});

describe('StatusBadge', () => {
  it('renders draft status', () => {
    render(<StatusBadge status="draft" />);
    expect(screen.getByText('草稿')).toBeInTheDocument();
  });

  it('renders reviewing status', () => {
    render(<StatusBadge status="reviewing" />);
    expect(screen.getByText('评审中')).toBeInTheDocument();
  });

  it('renders approved status', () => {
    render(<StatusBadge status="approved" />);
    expect(screen.getByText('已批准')).toBeInTheDocument();
  });

  it('renders completed status', () => {
    render(<StatusBadge status="completed" />);
    expect(screen.getByText('已完成')).toBeInTheDocument();
  });

  it('renders running status', () => {
    render(<StatusBadge status="running" />);
    expect(screen.getByText('运行中')).toBeInTheDocument();
  });

  it('renders failed status', () => {
    render(<StatusBadge status="failed" />);
    expect(screen.getByText('失败')).toBeInTheDocument();
  });
});

describe('PriorityBadge', () => {
  it('renders P0 priority', () => {
    render(<PriorityBadge priority="P0" />);
    expect(screen.getByText('P0 紧急')).toBeInTheDocument();
  });

  it('renders P1 priority', () => {
    render(<PriorityBadge priority="P1" />);
    expect(screen.getByText('P1 高')).toBeInTheDocument();
  });

  it('renders P2 priority', () => {
    render(<PriorityBadge priority="P2" />);
    expect(screen.getByText('P2 中')).toBeInTheDocument();
  });

  it('renders P3 priority', () => {
    render(<PriorityBadge priority="P3" />);
    expect(screen.getByText('P3 低')).toBeInTheDocument();
  });
});

describe('TaskTypeBadge', () => {
  it('renders requirement type', () => {
    render(<TaskTypeBadge type="requirement" />);
    expect(screen.getByText('需求')).toBeInTheDocument();
  });

  it('renders development type', () => {
    render(<TaskTypeBadge type="development" />);
    expect(screen.getByText('开发')).toBeInTheDocument();
  });

  it('renders testing type', () => {
    render(<TaskTypeBadge type="testing" />);
    expect(screen.getByText('测试')).toBeInTheDocument();
  });

  it('renders bug_fix type', () => {
    render(<TaskTypeBadge type="bug_fix" />);
    expect(screen.getByText('Bug修复')).toBeInTheDocument();
  });
});

describe('ReqTypeBadge', () => {
  it('renders feature type', () => {
    render(<ReqTypeBadge type="feature" />);
    expect(screen.getByText('功能')).toBeInTheDocument();
  });

  it('renders bug type', () => {
    render(<ReqTypeBadge type="bug" />);
    expect(screen.getByText('Bug')).toBeInTheDocument();
  });

  it('renders optimization type', () => {
    render(<ReqTypeBadge type="optimization" />);
    expect(screen.getByText('优化')).toBeInTheDocument();
  });
});
