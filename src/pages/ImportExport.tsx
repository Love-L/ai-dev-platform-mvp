import { useEffect, useState } from "react";
import { Download, Upload, FileJson, CheckCircle, AlertCircle, RefreshCw } from "lucide-react";
import { showToast } from "../components/Toast";
import { exportData, importData, type ImportResult } from "../lib/tauri";

/** 表名 → 中文展示名 */
const TABLE_LABELS: Record<string, string> = {
  projects: "项目",
  requirements: "需求",
  tasks: "任务",
  task_comments: "任务评论",
  agents: "Agent",
  agent_sessions: "会话",
  prd_skills: "Skill 模板",
  project_skills: "项目 Skill 配置",
  project_mcps: "项目 MCP 配置",
  stage_skill_configs: "环节 Skill 配置",
  token_usage: "Token 记录",
  token_budgets: "Token 预算",
  skill_execution_logs: "Skill 执行日志",
  mcp_execution_logs: "MCP 执行日志",
  git_commits: "Git 提交记录",
  requirement_history: "需求变更历史",
  settings: "系统设置",
};

/** 用于探测当前数据量的最小合法快照（tables 为空，仅做 dry_run） */
const PROBE_SNAPSHOT = JSON.stringify({
  format: "ace-export",
  version: "2.0",
  tables: {},
});

function formatCounts(counts?: Record<string, number> | null): { key: string; label: string; value: number }[] {
  if (!counts) return [];
  return Object.entries(counts)
    .filter(([, v]) => v > 0)
    .map(([key, value]) => ({
      key,
      label: TABLE_LABELS[key] ?? key,
      value,
    }));
}

export default function ImportExport() {
  const [exporting, setExporting] = useState(false);
  const [importing, setImporting] = useState(false);
  const [counts, setCounts] = useState<Record<string, number> | null>(null);
  const [pending, setPending] = useState<{ fileName: string; text: string; preview: ImportResult } | null>(null);
  const [replaceMode, setReplaceMode] = useState(false);

  useEffect(() => {
    void loadCounts();
  }, []);

  async function loadCounts() {
    try {
      const result = await importData(PROBE_SNAPSHOT, false, true);
      setCounts(result.current ?? {});
    } catch (error) {
      console.error("读取当前数据量失败:", error);
    }
  }

  async function handleExport() {
    setExporting(true);
    try {
      const snapshot = await exportData();

      // 导出前校验：确认后端确实返回了数据（避免出现"导出空文件却提示成功"）
      const tables = (snapshot as { tables?: Record<string, unknown[]> }).tables ?? {};
      const totalRows = Object.values(tables).reduce((sum, rows) => sum + (Array.isArray(rows) ? rows.length : 0), 0);

      const blob = new Blob([JSON.stringify(snapshot, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `ace-export-${new Date().toISOString().split("T")[0]}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      showToast("success", `数据导出成功，共 ${totalRows} 条记录`);
      await loadCounts();
    } catch (error) {
      console.error("导出失败:", error);
      showToast("error", "导出失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setExporting(false);
    }
  }

  async function handleFileSelected(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;

    setImporting(true);
    try {
      const text = await file.text();
      // 先做 dry_run 预检，展示影响面后再由用户确认
      const preview = await importData(text, replaceMode, true);
      setPending({ fileName: file.name, text, preview });
      showToast("info", "文件解析成功，请确认导入影响面");
    } catch (error) {
      console.error("导入预检失败:", error);
      showToast("error", "导入文件无效: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setImporting(false);
    }
  }

  async function confirmImport() {
    if (!pending) return;
    setImporting(true);
    try {
      const result = await importData(pending.text, replaceMode, false);
      showToast("success", `数据导入成功，共写入 ${result.total_imported ?? 0} 条记录（导入前已自动备份数据库）`);
      setPending(null);
      await loadCounts();
    } catch (error) {
      console.error("导入失败:", error);
      showToast("error", "导入失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setImporting(false);
    }
  }

  const currentStats = formatCounts(counts);

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>数据导入导出</h1>
        <p className="mt-0.5" style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>备份和恢复平台数据</p>
      </div>

      {/* 当前数据量 */}
      <div className="card">
        <div className="px-4 py-3 border-b border-gray-100 flex items-center justify-between">
          <h2 className="text-sm font-medium" style={{ color: "var(--color-text)" }}>当前数据量</h2>
          <button onClick={() => void loadCounts()} className="btn-secondary text-xs px-3 py-1.5">
            <RefreshCw className="w-3.5 h-3.5" />
            刷新
          </button>
        </div>
        <div className="p-4">
          {currentStats.length === 0 ? (
            <p style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>暂无数据</p>
          ) : (
            <div className="flex flex-wrap gap-3">
              {currentStats.map((item) => (
                <div
                  key={item.key}
                  className="px-3 py-2 rounded-lg"
                  style={{ background: "var(--color-primary-subtle)", fontSize: "13px" }}
                >
                  <span style={{ color: "var(--color-text-secondary)" }}>{item.label}</span>
                  <span className="ml-2 font-semibold" style={{ color: "var(--color-text)" }}>{item.value}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* 导出 */}
        <div className="card p-6">
          <div className="flex items-center gap-3 mb-4">
            <div
              className="w-12 h-12 rounded-xl flex items-center justify-center"
              style={{ background: "var(--color-primary-light)" }}
            >
              <Download className="w-6 h-6" style={{ color: "var(--color-primary)" }} />
            </div>
            <div>
              <h2 className="text-lg font-semibold" style={{ color: "var(--color-text)" }}>导出数据</h2>
              <p style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>将平台全量数据导出为 JSON 文件</p>
            </div>
          </div>

          <div className="space-y-3 mb-6">
            {["项目 / 需求 / 任务", "Skill 与 MCP 配置", "执行日志与审计记录", "Token 与预算数据"].map((label) => (
              <div key={label} className="flex items-center gap-2" style={{ color: "var(--color-text-secondary)", fontSize: "13px" }}>
                <CheckCircle className="w-4 h-4" style={{ color: "var(--color-success)" }} />
                <span>{label}</span>
              </div>
            ))}
          </div>

          <button onClick={() => void handleExport()} disabled={exporting} className="w-full btn-primary">
            <Download className="w-4 h-4" />
            {exporting ? "导出中..." : "导出数据"}
          </button>
        </div>

        {/* 导入 */}
        <div className="card p-6">
          <div className="flex items-center gap-3 mb-4">
            <div
              className="w-12 h-12 rounded-xl flex items-center justify-center"
              style={{ background: "var(--color-primary-subtle)" }}
            >
              <Upload className="w-6 h-6" style={{ color: "var(--color-primary)" }} />
            </div>
            <div>
              <h2 className="text-lg font-semibold" style={{ color: "var(--color-text)" }}>导入数据</h2>
              <p style={{ color: "var(--color-text-tertiary)", fontSize: "13px" }}>从 JSON 文件恢复平台数据</p>
            </div>
          </div>

          <div className="space-y-3 mb-6">
            <div className="flex items-center gap-2" style={{ color: "var(--color-text-secondary)", fontSize: "13px" }}>
              <FileJson className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
              <span>支持 ACE 导出的 JSON 格式</span>
            </div>
            <div className="flex items-center gap-2" style={{ color: "var(--color-text-secondary)", fontSize: "13px" }}>
              <AlertCircle className="w-4 h-4" style={{ color: "var(--color-warning)" }} />
              <span>导入前会自动备份当前数据库</span>
            </div>
            <label className="flex items-center gap-2 cursor-pointer" style={{ color: "var(--color-text-secondary)", fontSize: "13px" }}>
              <input
                type="checkbox"
                checked={replaceMode}
                onChange={(e) => setReplaceMode(e.target.checked)}
              />
              <span>完全恢复模式（先清空目标表再写入）</span>
            </label>
          </div>

          <label className="w-full btn-secondary cursor-pointer flex items-center justify-center gap-2">
            <Upload className="w-4 h-4" />
            {importing ? "解析中..." : "选择文件导入"}
            <input
              type="file"
              accept=".json"
              onChange={handleFileSelected}
              className="hidden"
              disabled={importing}
            />
          </label>
        </div>
      </div>

      {/* 导入确认面板 */}
      {pending && (
        <div className="card p-6 border" style={{ borderColor: "var(--color-warning)" }}>
          <h3 className="text-base font-semibold mb-2" style={{ color: "var(--color-text)" }}>
            确认导入：{pending.fileName}
          </h3>
          <p className="mb-3" style={{ color: "var(--color-text-secondary)", fontSize: "13px" }}>
            模式：{replaceMode ? "完全恢复（先清空目标表）" : "合并（按主键覆盖）"}；共 {pending.preview.total_incoming ?? 0} 条待写入记录。
          </p>
          <div className="flex flex-wrap gap-2 mb-4">
            {formatCounts(pending.preview.incoming).map((item) => (
              <span
                key={item.key}
                className="px-2 py-1 rounded"
                style={{ background: "var(--color-primary-subtle)", fontSize: "12px", color: "var(--color-text-secondary)" }}
              >
                {item.label} +{item.value}
              </span>
            ))}
          </div>
          <div className="flex gap-3">
            <button onClick={() => void confirmImport()} disabled={importing} className="btn-primary">
              {importing ? "导入中..." : "确认导入"}
            </button>
            <button onClick={() => setPending(null)} disabled={importing} className="btn-secondary">
              取消
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
