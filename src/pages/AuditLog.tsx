import { useEffect, useState } from "react";
import { History, Search } from "lucide-react";
import { getAuditLogs, type AuditLogEntry } from "../lib/tauri";

export default function AuditLog() {
  const [logs, setLogs] = useState<AuditLogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [filterType, setFilterType] = useState("all");
  const [searchQuery, setSearchQuery] = useState("");

  useEffect(() => {
    loadLogs();
  }, []);

  async function loadLogs() {
    try {
      const data = await getAuditLogs(200);
      setLogs(data);
    } catch (error) {
      console.error("加载审计日志失败:", error);
    } finally {
      setLoading(false);
    }
  }

  const filteredLogs = logs.filter((log) => {
    if (filterType !== "all" && log.entity_type !== filterType) {
      return false;
    }
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        log.entity_id.toLowerCase().includes(query) ||
        log.action.toLowerCase().includes(query) ||
        log.changes.toLowerCase().includes(query)
      );
    }
    return true;
  });

  function getActionStyle(action: string): { color: string; background: string } {
    switch (action) {
      case "create":
        return { color: "var(--color-success)", background: "var(--color-success-light)" };
      case "update":
        return { color: "var(--color-primary)", background: "var(--color-primary-subtle)" };
      case "delete":
        return { color: "var(--color-danger)", background: "var(--color-danger-light)" };
      case "status_change":
        return { color: "var(--color-warning)", background: "var(--color-warning-light)" };
      default:
        return { color: "var(--color-text-secondary)", background: "var(--color-bg-subtle)" };
    }
  }

  function getActionText(action: string) {
    switch (action) {
      case "create":
        return "创建";
      case "update":
        return "更新";
      case "delete":
        return "删除";
      case "status_change":
        return "状态变更";
      default:
        return action;
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="flex items-center gap-3 text-sm" style={{ color: "var(--color-text-tertiary)" }}>
          <div className="w-4 h-4 border-2 rounded-full animate-spin" style={{ borderColor: "var(--color-border)", borderTopColor: "var(--color-primary)" }} />
          加载中...
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-5">
      {/* 页面标题 */}
      <div>
        <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>操作审计</h1>
        <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>查看系统操作历史记录</p>
      </div>

      {/* 筛选栏 */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center gap-3">
        <div className="relative flex-1 w-full sm:max-w-xs">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="搜索日志..."
            className="input pl-9"
          />
        </div>
        <select
          value={filterType}
          onChange={(e) => setFilterType(e.target.value)}
          className="select w-full sm:w-auto"
        >
          <option value="all">全部类型</option>
          <option value="requirement">需求</option>
          <option value="task">任务</option>
          <option value="project">项目</option>
        </select>
      </div>

      {/* 日志列表 */}
      {filteredLogs.length === 0 ? (
        <div className="card p-12 text-center">
          <History className="w-10 h-10 mx-auto mb-3" style={{ color: "var(--color-text-tertiary)" }} />
          <p className="text-sm" style={{ color: "var(--color-text-tertiary)" }}>暂无操作记录</p>
        </div>
      ) : (
        <div className="space-y-2">
          {filteredLogs.map((log) => (
            <div key={log.id} className="card p-4">
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-3">
                  <span className="px-2 py-1 text-xs font-medium rounded" style={getActionStyle(log.action)}>
                    {getActionText(log.action)}
                  </span>
                  <div>
                    <p className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                      {log.entity_type} - {log.entity_id}
                    </p>
                    <p className="text-xs mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>{log.changes}</p>
                  </div>
                </div>
                <span className="text-xs" style={{ color: "var(--color-text-tertiary)" }}>
                  {new Date(log.created_at).toLocaleString("zh-CN")}
                </span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
