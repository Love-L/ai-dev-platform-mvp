import { useEffect, useRef, useState, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { listen } from '@tauri-apps/api/event';
import { ptyWrite, ptyResize, ptyKill, ptyAttach, ptyReconnect, saveSessionAttachment } from '../lib/tauri';
import { X, Square, Maximize2, Minimize2, Search, Download, Paperclip } from 'lucide-react';
import '@xterm/xterm/css/xterm.css';

interface AgentTerminalProps {
  sessionId: string;
  agentName: string;
  taskTitle?: string;
  onClose?: () => void;
  onStatusChange?: (status: 'running' | 'completed' | 'failed') => void;
}

interface PtyEvent {
  type: 'data' | 'exit' | 'error';
  session_id: string;
  data?: string;
  exit_code?: number;
  error?: string;
}

export default function AgentTerminal({
  sessionId,
  agentName,
  taskTitle,
  onClose,
  onStatusChange,
}: AgentTerminalProps) {
  const terminalRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const [isRunning, setIsRunning] = useState(true);
  const [isMaximized, setIsMaximized] = useState(false);
  const [showSearch, setShowSearch] = useState(false);
  const [searchText, setSearchText] = useState('');
  const [outputLog, setOutputLog] = useState<string[]>([]);
  const fileInputRef = useRef<HTMLInputElement>(null);
  /**
   * 真实 PTY 会话 id —— 与数据库 sessionId **不同源**：
   * 后端 spawn_agent 自行生成 uuid，须经 ptyAttach 解析后才能收发 pty:event
   * 与调用 ptyWrite/ptyResize/ptyKill（此前误用数据库 id 导致黑屏且无法输入）。
   */
  const ptyIdRef = useRef<string | null>(null);
  const [ptyId, setPtyId] = useState<string | null>(null);
  /** ptyAttach 是否已返回（区分"连接中"与"进程未运行"两种覆盖层） */
  const [attached, setAttached] = useState(false);
  const onStatusRef = useRef(onStatusChange);
  onStatusRef.current = onStatusChange;

  useEffect(() => {
    if (!terminalRef.current) return;

    const terminal = new Terminal({
      theme: {
        background: '#1e1e2e',
        foreground: '#cdd6f4',
        cursor: '#f5e0dc',
        cursorAccent: '#1e1e2e',
        selectionBackground: '#585b70',
        black: '#45475a',
        red: '#f38ba8',
        green: '#a6e3a1',
        yellow: '#f9e2af',
        blue: '#89b4fa',
        magenta: '#f5c2e7',
        cyan: '#94e2d5',
        white: '#bac2de',
        brightBlack: '#585b70',
        brightRed: '#f38ba8',
        brightGreen: '#a6e3a1',
        brightYellow: '#f9e2af',
        brightBlue: '#89b4fa',
        brightMagenta: '#f5c2e7',
        brightCyan: '#94e2d5',
        brightWhite: '#a6adc8',
      },
      fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", monospace',
      fontSize: 14,
      lineHeight: 1.2,
      cursorBlink: true,
      scrollback: 10000,
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    terminal.loadAddon(fitAddon);
    terminal.loadAddon(webLinksAddon);
    terminal.open(terminalRef.current);

    const fitTimer = setTimeout(() => {
      fitAddon.fit();
    }, 100);

    xtermRef.current = terminal;
    fitAddonRef.current = fitAddon;

    const handleResize = () => {
      fitAddon.fit();
    };
    window.addEventListener('resize', handleResize);

    let disposed = false;

    const unlisten = listen<PtyEvent>('pty:event', (event) => {
      const id = ptyIdRef.current;
      if (!id || event.payload.session_id !== id) return;

      switch (event.payload.type) {
        case 'data':
          if (event.payload.data) {
            terminal.write(event.payload.data);
            setOutputLog((prev) => [...prev, event.payload.data!]);
          }
          break;
        case 'exit':
          ptyIdRef.current = null;
          setPtyId(null);
          setIsRunning(false);
          onStatusRef.current?.(event.payload.exit_code === 0 ? 'completed' : 'failed');
          terminal.write(
            `\r\n\x1b[33m[Agent exited with code ${event.payload.exit_code}]\x1b[0m\r\n`
          );
          break;
        case 'error':
          terminal.write(`\r\n\x1b[31m[Error: ${event.payload.error}]\x1b[0m\r\n`);
          break;
      }
    });

    terminal.onData((data) => {
      const id = ptyIdRef.current;
      if (!id) return;
      ptyWrite(id, data).catch((err) => {
        console.error('Failed to write to PTY:', err);
      });
    });

    terminal.onResize(({ rows, cols }) => {
      const id = ptyIdRef.current;
      if (!id) return;
      ptyResize(id, rows, cols).catch((err) => {
        console.error('Failed to resize PTY:', err);
      });
    });

    // 附着：换取真实 PTY id + 回放挂载前的输出（否则打开终端只见一片黑）
    (async () => {
      try {
        const att = await ptyAttach(sessionId);
        if (disposed) return;
        ptyIdRef.current = att.pty_id;
        setPtyId(att.pty_id);
        setIsRunning(true);
        if (att.replay) terminal.write(att.replay);
        try {
          fitAddon.fit();
        } catch {
          /* 容器尚未完成布局时忽略 */
        }
        ptyResize(att.pty_id, terminal.rows, terminal.cols).catch(() => {});
      } catch {
        if (disposed) return;
        ptyIdRef.current = null;
        setPtyId(null);
        setIsRunning(false);
      } finally {
        if (!disposed) setAttached(true);
      }
    })();

    return () => {
      disposed = true;
      clearTimeout(fitTimer);
      ptyIdRef.current = null;
      setPtyId(null);
      setAttached(false);
      window.removeEventListener('resize', handleResize);
      unlisten.then((fn) => fn());
      terminal.dispose();
    };
  }, [sessionId]);

  const handleSearch = useCallback(() => {
    if (!xtermRef.current || !searchText) return;
    console.log('Search:', searchText);
  }, [searchText]);

  const handleStop = async () => {
    const id = ptyIdRef.current;
    if (!id) return;
    try {
      await ptyKill(id);
      ptyIdRef.current = null;
      setPtyId(null);
      setIsRunning(false);
      onStatusChange?.('failed');
    } catch (err) {
      console.error('Failed to stop Agent:', err);
    }
  };

  /** 进程已退出/无活动进程时重启 Agent 进程，恢复可交互终端 */
  const handleRestart = async () => {
    try {
      const id = await ptyReconnect(sessionId);
      ptyIdRef.current = id;
      setPtyId(id);
      setIsRunning(true);
      xtermRef.current?.write('\r\n\x1b[36m[Agent 进程已重启]\x1b[0m\r\n');
      try {
        fitAddonRef.current?.fit();
      } catch {
        /* ignore */
      }
      ptyResize(id, xtermRef.current?.rows ?? 24, xtermRef.current?.cols ?? 80).catch(() => {});
    } catch (err) {
      xtermRef.current?.write(`\r\n\x1b[31m[重启失败: ${err}]\x1b[0m\r\n`);
    }
  };

  const handleExportLog = () => {
    const logContent = outputLog.join('\n');
    const blob = new Blob([logContent], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `agent-log-${sessionId.slice(0, 8)}-${new Date().toISOString().slice(0, 10)}.log`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  /** 保存附件并把路径插入终端输入行（不换行，便于补充说明后回车） */
  const uploadAndInsert = useCallback(
    async (file: File) => {
      try {
        if (file.size > 20 * 1024 * 1024) {
          xtermRef.current?.write(`\r\n\x1b[31m[附件过大: ${file.name}（上限 20MB）]\x1b[0m\r\n`);
          return;
        }
        const bytes = await new Promise<Uint8Array>((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
          reader.onerror = () => reject(reader.error ?? new Error("读取文件失败"));
          reader.readAsArrayBuffer(file);
        });
        const saved = await saveSessionAttachment(sessionId, file.name, Array.from(bytes));
        xtermRef.current?.write(
          `\r\n\x1b[36m[附件已保存: ${saved.name}，路径已插入输入行]\x1b[0m\r\n`
        );
        const id = ptyIdRef.current;
        if (id) await ptyWrite(id, `"${saved.path}"`);
        else xtermRef.current?.write(`\r\n\x1b[31m[进程未运行，路径未插入]\x1b[0m\r\n`);
      } catch (err) {
        xtermRef.current?.write(`\r\n\x1b[31m[附件导入失败: ${err}]\x1b[0m\r\n`);
      }
    },
    [sessionId]
  );

  /** 粘贴图片 → 保存附件并插入路径；文本粘贴仍交由 xterm 默认处理 */
  function handlePaste(e: React.ClipboardEvent<HTMLDivElement>) {
    const items = e.clipboardData?.items;
    if (!items) return;
    for (let i = 0; i < items.length; i++) {
      if (items[i].kind !== 'file') continue;
      const file = items[i].getAsFile();
      if (file && file.type.startsWith('image/')) {
        e.preventDefault();
        uploadAndInsert(file);
        return;
      }
    }
  }

  const termBg = '#1e1e2e';
  const termHeaderBg = '#313244';
  const termBorder = '#45475a';
  const termFg = '#cdd6f4';
  const termMuted = '#6c7086';

  return (
    <div
      className={`flex flex-col rounded-lg overflow-hidden ${
        isMaximized ? 'fixed inset-0 z-50' : 'h-full'
      }`}
      style={{ background: termBg }}
      onPaste={handlePaste}
    >
      {/* 头部栏 */}
      <div className="flex items-center justify-between px-4 py-2" style={{ background: termHeaderBg, borderBottom: `1px solid ${termBorder}` }}>
        <div className="flex items-center gap-3">
          <div
            className="w-2 h-2 rounded-full"
            style={{
              background: isRunning ? '#a6e3a1' : '#585b70',
              animation: isRunning ? 'pulse 2s infinite' : 'none',
            }}
          />
          <span className="text-sm font-medium" style={{ color: termFg }}>{agentName}</span>
          {taskTitle && (
            <span className="text-xs" style={{ color: termMuted }}>任务: {taskTitle}</span>
          )}
          <span className="text-xs" style={{ color: termMuted }}>
            会话: {sessionId.slice(0, 8)}
          </span>
        </div>

        <div className="flex items-center gap-2">
          {showSearch && (
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={searchText}
                onChange={(e) => setSearchText(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                placeholder="搜索..."
                className="px-2 py-1 text-xs rounded"
                style={{
                  background: termBorder,
                  color: termFg,
                  border: `1px solid #585b70`,
                  outline: 'none',
                }}
                autoFocus
              />
              <button
                onClick={() => setShowSearch(false)}
                className="p-1 rounded"
                style={{ color: termMuted }}
                onMouseEnter={(e) => e.currentTarget.style.background = termBorder}
                onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
              >
                <X className="w-3 h-3" />
              </button>
            </div>
          )}

          <input
            ref={fileInputRef}
            type="file"
            multiple
            hidden
            accept="image/*,.pdf,.md,.txt,.csv,.json,.doc,.docx,.xls,.xlsx,.zip"
            onChange={(e) => {
              const files = Array.from(e.target.files ?? []);
              e.target.value = '';
              files.forEach((f) => uploadAndInsert(f));
            }}
          />

          <button
            onClick={() => fileInputRef.current?.click()}
            disabled={!isRunning}
            className="p-1.5 rounded transition-colors disabled:opacity-40"
            style={{ color: termMuted }}
            onMouseEnter={(e) => { if (isRunning) e.currentTarget.style.background = termBorder; }}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
            title="添加附件（可直接 Ctrl+V 粘贴图片）"
          >
            <Paperclip className="w-4 h-4" />
          </button>

          <button
            onClick={() => setShowSearch(!showSearch)}
            className="p-1.5 rounded transition-colors"
            style={{ color: termMuted }}
            onMouseEnter={(e) => e.currentTarget.style.background = termBorder}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
            title="搜索"
          >
            <Search className="w-4 h-4" />
          </button>

          <button
            onClick={handleExportLog}
            className="p-1.5 rounded transition-colors"
            style={{ color: termMuted }}
            onMouseEnter={(e) => e.currentTarget.style.background = termBorder}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
            title="导出日志"
          >
            <Download className="w-4 h-4" />
          </button>

          <button
            onClick={() => setIsMaximized(!isMaximized)}
            className="p-1.5 rounded transition-colors"
            style={{ color: termMuted }}
            onMouseEnter={(e) => e.currentTarget.style.background = termBorder}
            onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
            title={isMaximized ? '还原' : '最大化'}
          >
            {isMaximized ? (
              <Minimize2 className="w-4 h-4" />
            ) : (
              <Maximize2 className="w-4 h-4" />
            )}
          </button>

          {isRunning && (
            <button
              onClick={handleStop}
              className="p-1.5 rounded transition-colors"
              style={{ color: '#f38ba8' }}
              onMouseEnter={(e) => e.currentTarget.style.background = 'rgba(243, 139, 168, 0.15)'}
              onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
              title="停止"
            >
              <Square className="w-4 h-4" />
            </button>
          )}

          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 rounded transition-colors"
              style={{ color: termMuted }}
              onMouseEnter={(e) => e.currentTarget.style.background = termBorder}
              onMouseLeave={(e) => e.currentTarget.style.background = 'transparent'}
              title="关闭"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>

      {/* 终端内容 */}
      <div className="relative flex-1 flex">
        <div ref={terminalRef} className="flex-1 p-2 overflow-hidden" />
        {(!attached || !ptyId) && (
          <div
            className="absolute inset-0 flex flex-col items-center justify-center gap-3 text-center px-6"
            style={{ background: 'rgba(30, 30, 46, 0.92)' }}
          >
            {!attached ? (
              <p className="text-sm" style={{ color: termMuted }}>
                正在连接 Agent 终端…
              </p>
            ) : (
              <>
                <p className="text-sm" style={{ color: termFg }}>
                  Agent 进程未运行
                </p>
                <p className="text-xs" style={{ color: termMuted }}>
                  进程随应用退出而结束，重启后可继续在本窗口交互
                </p>
                <button
                  type="button"
                  className="btn-primary text-xs px-3 py-1.5"
                  onClick={handleRestart}
                >
                  重启 Agent 进程
                </button>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
