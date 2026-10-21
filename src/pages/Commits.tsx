import { useEffect, useState, useCallback } from "react";
import { GitCommit, GitBranch, RefreshCw, Search, Copy, ChevronDown, ChevronRight, Send, Link2 } from "lucide-react";
import {
  gitLog,
  gitGetDiff,
  gitGetStatus,
  gitCreateCommit,
  gitListBranches,
  gitSwitchBranch,
  gitShowFiles,
  gitCountCommits,
  getSetting,
  listGitCommits,
  listTasks,
  getAgentSessions,
  type GitCommitRecord,
} from "../lib/tauri";
import type { Task, AgentSession } from "../lib/types";
import { showToast } from "../components/Toast";

interface CommitRecord {
  hash: string;
  short_hash: string;
  message: string;
  author: string;
  date: string;
}

export default function Commits() {
  const [commits, setCommits] = useState<CommitRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [selectedBranch, setSelectedBranch] = useState<string>("");
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedCommit, setExpandedCommit] = useState<string | null>(null);
  const [commitDiff, setCommitDiff] = useState<string>("");
  const [diffLoading, setDiffLoading] = useState(false);
  const [workDir, setWorkDir] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const [commitMessage, setCommitMessage] = useState("");
  const [committing, setCommitting] = useState(false);
  const [uncommittedChanges, setUncommittedChanges] = useState(0);
  // 平台落库的提交记录（含"提交 ↔ 任务/会话"关联）
  const [platformCommits, setPlatformCommits] = useState<GitCommitRecord[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [sessions, setSessions] = useState<AgentSession[]>([]);
  const [linkTaskId, setLinkTaskId] = useState("");
  const [linkSessionId, setLinkSessionId] = useState("");
  // 分支切换
  const [branches, setBranches] = useState<string[]>([]);
  const [switching, setSwitching] = useState(false);
  // 真实提交计数（P1-C3：git rev-list 统计，非 log 窗口估算）
  const [commitStats, setCommitStats] = useState<{ total: number; today: number; week: number } | null>(null);
  // 展开提交时该提交涉及的文件列表
  const [commitFiles, setCommitFiles] = useState<Array<{ status: string; path: string }>>([]);

  const loadCommits = useCallback(async (dir: string) => {
    try {
      setError(null);
      const data = await gitLog(dir, 100);
      const records: CommitRecord[] = data.map((c) => ({
        hash: c.hash,
        short_hash: c.short_hash,
        message: c.message,
        author: c.author,
        date: c.date,
      }));
      setCommits(records);
    } catch (err) {
      console.error("Failed to load commits:", err);
      setError(`加载提交历史失败: ${err}`);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadBranch = useCallback(async (dir: string) => {
    try {
      const status = await gitGetStatus(dir);
      setSelectedBranch(status.branch);
      setUncommittedChanges(status.total_changes);
    } catch (err) {
      console.error("Failed to get branch:", err);
    }
  }, []);

  /** 加载本地分支列表（用于切换） */
  const loadBranches = useCallback(async (dir: string) => {
    try {
      const result = await gitListBranches(dir);
      setBranches(result?.branches ?? []);
    } catch (err) {
      console.error("Failed to load branches:", err);
      setBranches([]);
    }
  }, []);

  /** 加载真实提交计数 */
  const loadStats = useCallback(async (dir: string) => {
    try {
      const stats = await gitCountCommits(dir);
      setCommitStats(typeof stats?.total === "number" ? stats : null);
    } catch (err) {
      console.error("Failed to load commit stats:", err);
      setCommitStats(null);
    }
  }, []);

  /** 加载平台落库的提交记录（与 git log 不同：这里带任务/会话关联） */
  const loadPlatformCommits = useCallback(async () => {
    try {
      const records = await listGitCommits(undefined, undefined, 50);
      setPlatformCommits(records);
    } catch (err) {
      console.error("Failed to load platform commits:", err);
    }
  }, []);

  /** 加载可用于关联的任务与会话（会话仅取运行中的） */
  const loadLinkOptions = useCallback(async () => {
    try {
      const [taskList, sessionList] = await Promise.all([listTasks(), getAgentSessions()]);
      setTasks(taskList);
      setSessions(sessionList.filter((s) => s.status === "running"));
    } catch (err) {
      console.error("Failed to load link options:", err);
    }
  }, []);

  useEffect(() => {
    (async () => {
      const dir = await getSetting("project_path");
      if (!dir) {
        setLoading(false);
        setError("No project path configured. Please set project path in Settings.");
        return;
      }
      setWorkDir(dir);
      await Promise.all([
        loadCommits(dir),
        loadBranch(dir),
        loadBranches(dir),
        loadStats(dir),
        loadPlatformCommits(),
        loadLinkOptions(),
      ]);
    })();
  }, [loadCommits, loadBranch, loadBranches, loadStats, loadPlatformCommits, loadLinkOptions]);

  async function handleRefresh() {
    if (!workDir) return;
    setRefreshing(true);
    try {
      await Promise.all([
        loadCommits(workDir),
        loadBranch(workDir),
        loadStats(workDir),
        loadPlatformCommits(),
        loadLinkOptions(),
      ]);
    } finally {
      setRefreshing(false);
    }
  }

  async function handleCommit() {
    if (!workDir || !commitMessage.trim()) return;
    setCommitting(true);
    try {
      // 携带任务/会话关联，使提交可追溯到具体工作项（F060）
      const result = await gitCreateCommit(
        workDir,
        commitMessage.trim(),
        linkTaskId || undefined,
        linkSessionId || undefined,
      );
      setCommitMessage("");
      setLinkTaskId("");
      setLinkSessionId("");
      // 追溯记录落库失败时透出警告（Git 提交本身已成功）
      if (result?.db_warning) {
        showToast("warning", result.db_warning);
      } else {
        showToast("success", linkTaskId || linkSessionId ? "提交成功，并已关联任务/会话" : "提交成功");
      }
      await Promise.all([loadCommits(workDir), loadBranch(workDir), loadPlatformCommits()]);
    } catch (err) {
      console.error("Failed to commit:", err);
      showToast("error", `提交失败: ${err}`);
    } finally {
      setCommitting(false);
    }
  }

  function taskTitleOf(taskId?: string | null) {
    if (!taskId) return null;
    return tasks.find((t) => t.id === taskId)?.title ?? taskId.slice(0, 8);
  }

  async function handleViewDiff(commit: CommitRecord) {
    if (expandedCommit === commit.hash) {
      setExpandedCommit(null);
      setCommitDiff("");
      return;
    }

    setExpandedCommit(commit.hash);
    setDiffLoading(true);

    try {
      // 传 commit hash 查看该提交本身的差异（此前误查工作区，所有提交显示同一份 diff）+ 涉及文件
      const [diff, files] = await Promise.all([
        gitGetDiff(workDir, undefined, commit.hash),
        gitShowFiles(workDir, commit.hash).catch(() => []),
      ]);
      setCommitDiff(diff);
      setCommitFiles(files);
    } catch (err) {
      console.error("Failed to load diff:", err);
      setCommitDiff(`加载该提交的 diff 失败: ${err}`);
      setCommitFiles([]);
    } finally {
      setDiffLoading(false);
    }
  }

  /** 切换分支并刷新提交历史 */
  async function handleSwitchBranch(branch: string) {
    if (!workDir || !branch || branch === selectedBranch || switching) return;
    setSwitching(true);
    try {
      const result = await gitSwitchBranch(workDir, branch);
      showToast("success", result.message);
      setSelectedBranch(result.branch);
      await Promise.all([loadCommits(workDir), loadBranch(workDir), loadBranches(workDir)]);
    } catch (err) {
      showToast("error", `切换分支失败: ${err}`);
    } finally {
      setSwitching(false);
    }
  }

  function handleCopyHash(hash: string) {
    navigator.clipboard.writeText(hash);
  }

  const filteredCommits = commits.filter((commit) => {
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        commit.message.toLowerCase().includes(query) ||
        commit.short_hash.toLowerCase().includes(query) ||
        commit.author.toLowerCase().includes(query)
      );
    }
    return true;
  });

  function formatTime(dateString: string) {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();

    if (diff < 60 * 1000) {
      return "刚刚";
    } else if (diff < 60 * 60 * 1000) {
      return `${Math.floor(diff / (60 * 1000))} 分钟前`;
    } else if (diff < 24 * 60 * 60 * 1000) {
      return `${Math.floor(diff / (60 * 60 * 1000))} 小时前`;
    } else if (diff < 7 * 24 * 60 * 60 * 1000) {
      return `${Math.floor(diff / (24 * 60 * 60 * 1000))} 天前`;
    } else {
      return date.toLocaleDateString("zh-CN");
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div style={{ color: "var(--color-text-tertiary)" }}>加载中...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 overflow-auto p-6">
        <div className="card p-8 text-center">
          <GitCommit className="w-12 h-12 mx-auto mb-4" style={{ color: "var(--color-border)" }} />
          <p style={{ color: "var(--color-text-secondary)" }} className="mb-2">
            {error}
          </p>
          <button onClick={handleRefresh} className="btn-primary mt-4">
            <RefreshCw className="w-4 h-4" />
            重试
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto p-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
            提交记录
          </h1>
          <p className="mt-1" style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>
            查看代码提交历史
          </p>
        </div>
        <button onClick={handleRefresh} disabled={refreshing} className="btn-secondary">
          <RefreshCw className={`w-4 h-4 ${refreshing ? "animate-spin" : ""}`} />
          刷新
        </button>
      </div>

      {/* Stats cards（P1-C3：git rev-list 真实统计，非 log 窗口估算） */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 mb-6">
        <div className="card p-4">
          <div style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>总提交</div>
          <div className="text-2xl font-bold mt-1" style={{ color: "var(--color-text)" }}>
            {commitStats ? commitStats.total : "-"}
          </div>
        </div>
        <div className="card p-4">
          <div style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>今日</div>
          <div className="text-2xl font-bold mt-1" style={{ color: "var(--color-primary)" }}>
            {commitStats ? commitStats.today : "-"}
          </div>
        </div>
        <div className="card p-4">
          <div style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>近7天</div>
          <div className="text-2xl font-bold mt-1" style={{ color: "var(--color-success)" }}>
            {commitStats ? commitStats.week : "-"}
          </div>
        </div>
        <div className="card p-4">
          <div style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>当前分支</div>
          <div className="text-2xl font-bold mt-1 truncate" style={{ color: "var(--color-primary)" }}>
            {selectedBranch || "unknown"}
          </div>
        </div>
      </div>

      {/* Search */}
      <div className="flex items-center gap-4 mb-6">
        {selectedBranch && (
          <div
            className="flex items-center gap-2 px-3 py-2 rounded-lg"
            style={{ background: "var(--color-bg-subtle)", border: "1px solid var(--color-border)" }}
          >
            <GitBranch className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
            <select
              value={selectedBranch}
              onChange={(e) => handleSwitchBranch(e.target.value)}
              disabled={switching}
              className="bg-transparent outline-none"
              style={{ color: "var(--color-text-secondary)", fontSize: "13px" }}
              title={branches.length > 0 ? "切换分支" : "仅当前分支"}
            >
              {[selectedBranch, ...branches.filter((b) => b !== selectedBranch)].map((b) => (
                <option key={b} value={b}>
                  {b}
                </option>
              ))}
            </select>
            {switching && (
              <RefreshCw className="w-3 h-3 animate-spin" style={{ color: "var(--color-text-tertiary)" }} />
            )}
          </div>
        )}

        <div className="flex-1 relative">
          <Search
            className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4"
            style={{ color: "var(--color-text-tertiary)" }}
          />
          <input
            type="text"
            placeholder="搜索提交..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="input pl-10"
          />
        </div>
      </div>

      {/* Commit form */}
      {uncommittedChanges > 0 && (
        <div className="card p-4 mb-6">
          <div className="flex items-center gap-2 mb-3">
            <GitCommit className="w-4 h-4" style={{ color: "var(--color-primary)" }} />
            <span className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
              {uncommittedChanges} 个未提交的变更
            </span>
          </div>
          <div className="flex gap-3 mb-3">
            <select
              value={linkTaskId}
              onChange={(e) => setLinkTaskId(e.target.value)}
              className="input flex-1"
              title="将本次提交关联到任务，便于后续追溯"
            >
              <option value="">不关联任务</option>
              {tasks.map((task) => (
                <option key={task.id} value={task.id}>
                  {task.title}
                </option>
              ))}
            </select>
            <select
              value={linkSessionId}
              onChange={(e) => setLinkSessionId(e.target.value)}
              className="input flex-1"
              title="将本次提交关联到 Agent 会话"
            >
              <option value="">不关联会话</option>
              {sessions.map((session) => (
                <option key={session.id} value={session.id}>
                  {session.title || `会话 ${session.id.slice(0, 8)}`}
                </option>
              ))}
            </select>
          </div>
          <div className="flex gap-3">
            <input
              type="text"
              placeholder="输入提交信息..."
              value={commitMessage}
              onChange={(e) => setCommitMessage(e.target.value)}
              className="input flex-1"
              onKeyDown={(e) => {
                if (e.key === "Enter" && commitMessage.trim()) handleCommit();
              }}
            />
            <button onClick={handleCommit} disabled={!commitMessage.trim() || committing} className="btn-primary">
              <Send className={`w-4 h-4 ${committing ? "animate-spin" : ""}`} />
              提交
            </button>
          </div>
        </div>
      )}

      {/* 平台提交记录（含任务/会话关联） */}
      <div className="card overflow-hidden mb-6">
        <div
          className="px-4 py-3 flex items-center justify-between"
          style={{ borderBottom: "1px solid var(--color-border-subtle)" }}
        >
          <div className="flex items-center gap-2">
            <Link2 className="w-4 h-4" style={{ color: "var(--color-primary)" }} />
            <span className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
              平台提交记录（可按任务/会话追溯）
            </span>
          </div>
          <span style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>{platformCommits.length} 条</span>
        </div>
        {platformCommits.length === 0 ? (
          <div className="empty-state py-6">
            <p className="empty-state-desc">暂无平台提交记录。在本页提交代码时会自动登记，并可按任务/会话追溯。</p>
          </div>
        ) : (
          platformCommits.map((record) => (
            <div key={record.id} className="px-4 py-3" style={{ borderBottom: "1px solid var(--color-border-subtle)" }}>
              <div className="flex items-center gap-2 min-w-0">
                <span className="font-mono text-xs" style={{ color: "var(--color-primary)" }}>
                  {record.short_hash}
                </span>
                <span className="truncate" style={{ color: "var(--color-text)" }}>
                  {record.message || "(无提交信息)"}
                </span>
              </div>
              <div
                className="flex items-center gap-3 mt-1 flex-wrap"
                style={{ fontSize: "12px", color: "var(--color-text-tertiary)" }}
              >
                <span>{record.branch_name || "unknown"}</span>
                <span>{record.files_changed} 个文件</span>
                <span>{formatTime(record.created_at)}</span>
                {taskTitleOf(record.task_id) && (
                  <span
                    className="px-1.5 py-0.5 rounded"
                    style={{ background: "var(--color-primary-subtle)", color: "var(--color-primary)" }}
                  >
                    任务: {taskTitleOf(record.task_id)}
                  </span>
                )}
                {record.session_id && (
                  <span
                    className="px-1.5 py-0.5 rounded"
                    style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-secondary)" }}
                  >
                    会话: {record.session_id.slice(0, 8)}
                  </span>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Commit list */}
      <div className="card overflow-hidden">
        <div>
          {filteredCommits.length === 0 ? (
            <div className="empty-state py-8">
              <GitCommit className="w-12 h-12 mb-4" style={{ color: "var(--color-border)" }} />
              <p className="empty-state-title">暂无提交记录</p>
            </div>
          ) : (
            filteredCommits.map((commit) => (
              <div
                key={commit.hash}
                style={{ borderBottom: "1px solid var(--color-border-subtle)" }}
                className="hover:bg-[var(--color-primary-subtle)]"
              >
                <div className="px-4 py-3 cursor-pointer" onClick={() => handleViewDiff(commit)}>
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        {expandedCommit === commit.hash ? (
                          <ChevronDown className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
                        ) : (
                          <ChevronRight className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
                        )}
                        <span className="font-medium" style={{ color: "var(--color-text)" }}>
                          {commit.message}
                        </span>
                      </div>
                      <div
                        className="flex items-center gap-4 mt-1 ml-6"
                        style={{ fontSize: "13px", color: "var(--color-text-tertiary)" }}
                      >
                        <span className="font-mono">{commit.short_hash}</span>
                        <span>{commit.author}</span>
                        <span>{formatTime(commit.date)}</span>
                      </div>
                    </div>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleCopyHash(commit.hash);
                      }}
                      className="p-1 rounded-lg transition-colors"
                      style={{ color: "var(--color-text-tertiary)" }}
                      onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-bg-subtle)")}
                      onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
                      title="复制完整哈希"
                    >
                      <Copy className="w-4 h-4" />
                    </button>
                  </div>
                </div>

                {/* Diff area */}
                {expandedCommit === commit.hash && (
                  <div className="px-4 pb-4 ml-6">
                    {/* 该提交涉及的文件 */}
                    {commitFiles.length > 0 && (
                      <div className="mb-2 flex flex-wrap gap-2">
                        {commitFiles.map((f) => (
                          <span
                            key={f.path}
                            className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-mono"
                            style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-secondary)" }}
                            title={f.path}
                          >
                            <span
                              className="px-1 rounded"
                              style={{
                                background: f.status.startsWith("A")
                                  ? "var(--color-success-light)"
                                  : f.status.startsWith("D")
                                    ? "var(--color-danger-light)"
                                    : "var(--color-primary-subtle)",
                                color: f.status.startsWith("A")
                                  ? "var(--color-success)"
                                  : f.status.startsWith("D")
                                    ? "var(--color-danger)"
                                    : "var(--color-primary)",
                                fontSize: "10px",
                              }}
                            >
                              {f.status}
                            </span>
                            <span className="truncate max-w-[260px]">{f.path}</span>
                          </span>
                        ))}
                      </div>
                    )}
                    <div className="rounded-lg p-4 overflow-x-auto" style={{ background: "#1e1e2e" }}>
                      {diffLoading ? (
                        <div style={{ color: "#6c7086" }}>加载差异...</div>
                      ) : (
                        <pre className="text-sm font-mono whitespace-pre-wrap" style={{ color: "#cdd6f4" }}>
                          {commitDiff || "暂无差异数据"}
                        </pre>
                      )}
                    </div>
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
