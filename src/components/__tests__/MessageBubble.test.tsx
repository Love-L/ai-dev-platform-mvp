import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import MessageBubble from '../MessageBubble';
import type { SessionMessage } from '../../lib/types';

function msg(partial: Partial<SessionMessage>): SessionMessage {
  return {
    id: 'm1',
    session_id: 's1',
    seq: 1,
    role: 'assistant',
    block_type: 'text',
    status: 'completed',
    created_at: '2026-09-21T10:00:00Z',
    ...partial,
  };
}

describe('MessageBubble（模块 16 / F078–F080、F083）', () => {
  it('F079: 思考块默认折叠，展开后才显示内容', () => {
    render(
      <MessageBubble
        message={msg({ block_type: 'thinking', content: '先分析需求再动手', tokens: 12 })}
      />
    );

    // 折叠态：只显示标题与 token 估算，不显示正文
    expect(screen.getByText('思考过程')).toBeInTheDocument();
    expect(screen.getByText(/约 12 tokens/)).toBeInTheDocument();
    expect(screen.queryByText('先分析需求再动手')).not.toBeInTheDocument();

    fireEvent.click(screen.getByText('思考过程'));
    expect(screen.getByText('先分析需求再动手')).toBeInTheDocument();
  });

  it('F080: 工具调用块展示工具名与入参', () => {
    render(
      <MessageBubble
        message={msg({
          block_type: 'tool_use',
          tool_name: 'Bash',
          tool_use_id: 'call_1',
          content: JSON.stringify({ command: 'echo hi' }),
        })}
      />
    );

    expect(screen.getByText('调用工具 Bash')).toBeInTheDocument();
    fireEvent.click(screen.getByText('调用工具 Bash'));
    expect(screen.getByText(/"command": "echo hi"/)).toBeInTheDocument();
  });

  it('F083: 被拒卡片展示原因并提供"始终允许并重试"与"改用 acceptEdits"', () => {
    const onAllow = vi.fn();
    const onAcceptEdits = vi.fn();

    render(
      <MessageBubble
        message={msg({
          block_type: 'permission_denied',
          tool_name: 'Write',
          status: 'denied',
          content: "Claude requested permissions to write to X, but you haven't granted it yet.",
        })}
        onAllowAndRetry={onAllow}
        onUseAcceptEdits={onAcceptEdits}
      />,
    );

    expect(screen.getByText(/操作被权限策略拒绝（Write）/)).toBeInTheDocument();
    // 后端 message 原样展示，用户可据此判断被拒原因
    expect(screen.getByText(/haven't granted/)).toBeInTheDocument();

    fireEvent.click(screen.getByText('始终允许并重试'));
    expect(onAllow).toHaveBeenCalledWith('Write');

    fireEvent.click(screen.getByText('改用 acceptEdits'));
    expect(onAcceptEdits).toHaveBeenCalled();
  });

  it('F078: 正文按角色区分展示，并渲染围栏代码块', () => {
    const { container } = render(
      <MessageBubble
        message={msg({
          role: 'assistant',
          block_type: 'text',
          content: '示例：\n```ts\nconst a = 1;\n```\n结束',
        })}
      />
    );

    expect(container.querySelector('pre')?.textContent).toContain('const a = 1;');
  });

  it('F084: 单轮结果展示 token 与成本', () => {
    render(
      <MessageBubble
        message={msg({
          block_type: 'result',
          role: 'system',
          content: JSON.stringify({
            input_tokens: 1200,
            output_tokens: 80,
            cost_usd: 0.0123,
            duration_ms: 2500,
          }),
        })}
      />
    );

    expect(screen.getByText(/输入 1,200/)).toBeInTheDocument();
    expect(screen.getByText(/输出 80/)).toBeInTheDocument();
    expect(screen.getByText('$0.0123')).toBeInTheDocument();
  });

  it('上下文卡片展示任务/需求/权限预设（Q7）', () => {
    render(
      <MessageBubble
        message={msg({
          block_type: 'context',
          role: 'system',
          content: JSON.stringify({
            mode: 'structured',
            permission_mode: 'acceptEdits',
            task_id: 'task-1',
            requirement_id: 'req-1',
          }),
        })}
      />
    );

    expect(screen.getByText('会话上下文')).toBeInTheDocument();
    expect(screen.getByText('task-1')).toBeInTheDocument();
    expect(screen.getByText('req-1')).toBeInTheDocument();
    expect(screen.getByText('acceptEdits')).toBeInTheDocument();
  });
});
