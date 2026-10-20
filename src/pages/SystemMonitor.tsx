import { useEffect, useState } from "react";
import { Cpu, HardDrive, Clock, AlertTriangle, RefreshCw, Trash2 } from "lucide-react";
import { getCacheStats, clearCache, getMonitorData, checkAlerts } from "../lib/tauri";
import type { CacheStats } from "../lib/types";
import { showToast } from "../components/Toast";

interface MonitorData {
  cpu_usage: number;
  memory_used_mb: number;
  memory_usage_percent: number;
  uptime_seconds: number;
  active_sessions: number;
}

interface AlertItem {
  level: string;
  message: string;
}

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days}天 ${hours}小时`;
  if (hours > 0) return `${hours}小时 ${minutes}分钟`;
  return `${minutes}分钟`;
}

export default function SystemMonitor() {
  const [data, setData] = useState<MonitorData | null>(null);
  const [alerts, setAlerts] = useState<AlertItem[]>([]);
  const [cacheStats, setCacheStats] = useState<CacheStats | null>(null);
  const [clearing, setClearing] = useState(false);
  const [loading, setLoading] = useState(true);
  const [lastRefresh, setLastRefresh] = useState<Date>(new Date());

  async function fetchData() {
    try {
      const [monitorData, alertData, cacheData] = await Promise.all([
        getMonitorData(),
        checkAlerts(),
        getCacheStats().catch(() => null),
      ]);
      setData(monitorData);
      setAlerts(alertData);
      setCacheStats(cacheData);
      setLastRefresh(new Date());
    } catch (error) {
      console.error("获取监控数据失败:", error);
    } finally {
      setLoading(false);
    }
  }

  /** F045 清空内存缓存 */
  async function handleClearCache() {
    setClearing(true);
    try {
      await clearCache();
      const stats = await getCacheStats().catch(() => null);
      setCacheStats(stats);
      showToast("success", "缓存已清空");
    } catch (error) {
      console.error("清空缓存失败:", error);
      showToast("error", "清空缓存失败: " + (error instanceof Error ? error.message : String(error)));
    } finally {
      setClearing(false);
    }
  }

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 10000); // 每10秒刷新
    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-sm text-gray-400">加载中...</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-gray-900">系统监控</h1>
          <p className="text-sm text-gray-500 mt-0.5">实时系统资源使用情况</p>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-gray-400">
            上次刷新: {lastRefresh.toLocaleTimeString()}
          </span>
          <button
            onClick={fetchData}
            className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <RefreshCw className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* 告警信息 */}
      {alerts.length > 0 && (
        <div className="space-y-2">
          {alerts.map((alert, i) => (
            <div
              key={i}
              className={`flex items-center gap-3 p-3 rounded-lg border ${
                alert.level === "critical"
                  ? "bg-red-50 border-red-200 text-red-800"
                  : "bg-yellow-50 border-yellow-200 text-yellow-800"
              }`}
            >
              <AlertTriangle className="w-4 h-4 flex-shrink-0" />
              <span className="text-sm">{alert.message}</span>
            </div>
          ))}
        </div>
      )}

      {/* 监控卡片 */}
      {data && (
        <div className="grid grid-cols-3 gap-4">
          {/* CPU */}
          <div className="card p-5">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2.5 bg-blue-50 rounded-lg">
                <Cpu className="w-5 h-5 text-blue-600" />
              </div>
              <div>
                <p className="text-sm font-medium text-gray-700">CPU 使用率</p>
                <p className="text-2xl font-bold text-gray-900">{data.cpu_usage.toFixed(1)}%</p>
              </div>
            </div>
            <div className="w-full bg-gray-100 rounded-full h-2">
              <div
                className={`h-2 rounded-full transition-all duration-500 ${
                  data.cpu_usage > 80 ? "bg-red-500" : data.cpu_usage > 50 ? "bg-yellow-500" : "bg-blue-500"
                }`}
                style={{ width: `${Math.min(data.cpu_usage, 100)}%` }}
              />
            </div>
          </div>

          {/* Memory */}
          <div className="card p-5">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2.5 bg-purple-50 rounded-lg">
                <HardDrive className="w-5 h-5 text-purple-600" />
              </div>
              <div>
                <p className="text-sm font-medium text-gray-700">内存使用</p>
                <p className="text-2xl font-bold text-gray-900">{data.memory_usage_percent.toFixed(1)}%</p>
              </div>
            </div>
            <div className="w-full bg-gray-100 rounded-full h-2">
              <div
                className={`h-2 rounded-full transition-all duration-500 ${
                  data.memory_usage_percent > 80 ? "bg-red-500" : data.memory_usage_percent > 50 ? "bg-yellow-500" : "bg-purple-500"
                }`}
                style={{ width: `${Math.min(data.memory_usage_percent, 100)}%` }}
              />
            </div>
            <p className="text-xs text-gray-400 mt-2">
              应用内存: {data.memory_used_mb} MB
            </p>
          </div>

          {/* Uptime & Sessions */}
          <div className="card p-5">
            <div className="flex items-center gap-3 mb-4">
              <div className="p-2.5 bg-emerald-50 rounded-lg">
                <Clock className="w-5 h-5 text-emerald-600" />
              </div>
              <div>
                <p className="text-sm font-medium text-gray-700">运行状态</p>
                <p className="text-2xl font-bold text-gray-900">{data.active_sessions} 会话</p>
              </div>
            </div>
            <div className="space-y-2">
              <div className="flex justify-between text-xs">
                <span className="text-gray-500">运行时间</span>
                <span className="text-gray-700 font-medium">{formatUptime(data.uptime_seconds)}</span>
              </div>
              <div className="flex justify-between text-xs">
                <span className="text-gray-500">活跃会话</span>
                <span className="text-gray-700 font-medium">{data.active_sessions}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 缓存统计（F045） */}
      <div className="card">
        <div className="px-4 py-3 border-b border-gray-100 flex items-center justify-between">
          <h2 className="text-sm font-medium text-gray-700">内存缓存</h2>
          <button
            onClick={() => void handleClearCache()}
            disabled={clearing}
            className="flex items-center gap-1.5 text-xs px-3 py-1.5 rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-50"
          >
            <Trash2 className="w-3.5 h-3.5" />
            {clearing ? "清理中..." : "清空缓存"}
          </button>
        </div>
        <div className="p-4">
          {cacheStats ? (
            <div className="grid grid-cols-4 gap-4">
              <div>
                <p className="text-xs text-gray-500">命中 / 未命中</p>
                <p className="text-lg font-semibold text-gray-900">
                  {cacheStats.hits} / {cacheStats.misses}
                </p>
              </div>
              <div>
                <p className="text-xs text-gray-500">命中率</p>
                <p className="text-lg font-semibold text-gray-900">{cacheStats.hit_rate}</p>
              </div>
              <div>
                <p className="text-xs text-gray-500">缓存条目</p>
                <p className="text-lg font-semibold text-gray-900">{cacheStats.size}</p>
              </div>
              <div>
                <p className="text-xs text-gray-500">状态</p>
                <p className="text-lg font-semibold text-gray-900">
                  {cacheStats.enabled ? "已启用" : "已禁用"}
                </p>
              </div>
            </div>
          ) : (
            <p className="text-xs text-gray-400">缓存统计不可用</p>
          )}
        </div>
      </div>

      {/* 系统信息 */}
      <div className="card">
        <div className="px-4 py-3 border-b border-gray-100">
          <h2 className="text-sm font-medium text-gray-700">系统信息</h2>
        </div>
        <div className="p-4 grid grid-cols-2 gap-4">
          <div className="space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-gray-500">平台</span>
              <span className="text-gray-700">{navigator.platform}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-500">语言</span>
              <span className="text-gray-700">{navigator.language}</span>
            </div>
          </div>
          <div className="space-y-3">
            <div className="flex justify-between text-sm">
              <span className="text-gray-500">CPU 核心数</span>
              <span className="text-gray-700">{navigator.hardwareConcurrency || "未知"}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-gray-500">设备内存</span>
              <span className="text-gray-700">
                {(navigator as any).deviceMemory
                  ? `${(navigator as any).deviceMemory} GB`
                  : "未知"}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
