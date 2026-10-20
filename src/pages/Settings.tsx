import { useEffect, useState } from "react";
import { Save, Key, FolderOpen, Info, FileText, ChevronDown, ChevronUp, Terminal, RefreshCw } from "lucide-react";
import { listPrdSkills, getAgents } from "../lib/tauri";
import type { PrdSkill } from "../lib/types";
import { useSettingsStore } from "../stores";

export default function Settings() {
  const { fetchSettings, saveSetting } = useSettingsStore();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [skills, setSkills] = useState<PrdSkill[]>([]);
  const [expandedSkill, setExpandedSkill] = useState<string | null>(null);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateMessage, setUpdateMessage] = useState("");

  const [apiKey, setApiKey] = useState("");
  const [projectName, setProjectName] = useState("默认项目");
  const [projectPath, setProjectPath] = useState("");
  const [commitTemplate, setCommitTemplate] = useState("feat({task_id}): {task_title}");
  const [autoCommit, setAutoCommit] = useState("auto");
  const [sessionMode, setSessionMode] = useState("auto");
  const [maxSessions, setMaxSessions] = useState("3");
  const [prdMode, setPrdMode] = useState("api");
  const [defaultAgent, setDefaultAgent] = useState("");
  const [allowedTools, setAllowedTools] = useState("");
  const [agentsList, setAgentsList] = useState<
    { id: string; display_name: string; binary_path: string; status: string }[]
  >([]);

  useEffect(() => {
    loadSettings();
  }, []);

  async function loadSettings() {
    try {
      const keys = [
        "claude_api_key",
        "project_name",
        "project_path",
        "commit_template",
        "auto_commit",
        "session_mode",
        "prd_generation_mode",
        "default_agent_path",
        "session_allowed_tools",
        "max_concurrent_sessions",
      ];
      await fetchSettings(keys);

      // Read fresh values from the store after batch fetch
      // API Key 由后端 keystore 统一解封（含历史数据的透明迁移），前端直接使用明文
      const fresh = useSettingsStore.getState().settings;
      setApiKey(fresh["claude_api_key"] || "");

      setProjectName(fresh["project_name"] || "默认项目");
      setProjectPath(fresh["project_path"] || "");
      setCommitTemplate(fresh["commit_template"] || "feat({task_id}): {task_title}");
      setAutoCommit(fresh["auto_commit"] || "auto");
      setSessionMode(fresh["session_mode"] || "auto");
      setPrdMode(fresh["prd_generation_mode"] || "api");
      setDefaultAgent(fresh["default_agent_path"] || "");
      setAllowedTools(fresh["session_allowed_tools"] || "");
      setMaxSessions(fresh["max_concurrent_sessions"] || "3");

      const skillsData = await listPrdSkills();
      setSkills(skillsData);

      // 默认 Agent 候选（MA-09）
      try {
        const agents = await getAgents();
        setAgentsList(
          agents.map((a) => ({
            id: a.id,
            display_name: a.display_name,
            binary_path: a.binary_path,
            status: a.status,
          })),
        );
      } catch {
        // Agent 列表加载失败不阻塞设置页
      }
    } catch (error) {
      console.error("加载设置失败:", error);
    } finally {
      setLoading(false);
    }
  }

  async function handleSave() {
    setSaving(true);
    try {
      // 明文交由后端 set_setting 用本地 keystore 加密落盘（enc:v1:）
      await Promise.all([
        saveSetting("claude_api_key", apiKey),
        saveSetting("project_name", projectName),
        saveSetting("project_path", projectPath),
        saveSetting("commit_template", commitTemplate),
        saveSetting("auto_commit", autoCommit),
        saveSetting("session_mode", sessionMode),
        saveSetting("prd_generation_mode", prdMode),
        saveSetting("default_agent_path", defaultAgent),
        saveSetting("session_allowed_tools", allowedTools),
        saveSetting("max_concurrent_sessions", String(Math.min(16, Math.max(1, Number(maxSessions) || 3)))),
      ]);
    } catch (error) {
      console.error("保存设置失败:", error);
    } finally {
      setSaving(false);
    }
  }

  const handleCheckUpdate = async () => {
    setUpdateChecking(true);
    setUpdateMessage("");
    try {
      // 动态加载：浏览器预览/测试环境无 Tauri updater 运行时，点击时才解析
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (update) {
        setUpdateMessage(`发现新版本 ${update.version}（当前 ${update.currentVersion}），正在下载安装…`);
        await update.downloadAndInstall();
        setUpdateMessage(`新版本 ${update.version} 已安装，重启应用即可生效。`);
      } else {
        setUpdateMessage("已是最新版本");
      }
    } catch (error) {
      setUpdateMessage(`检查更新失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setUpdateChecking(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="flex items-center gap-3 text-sm" style={{ color: "var(--color-text-tertiary)" }}>
          <div
            className="w-4 h-4 border-2 rounded-full animate-spin"
            style={{ borderColor: "var(--color-border)", borderTopColor: "var(--color-primary)" }}
          />
          加载中...
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-5 max-w-3xl">
      {/* 页面标题 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold" style={{ color: "var(--color-text)" }}>
            系统设置
          </h1>
          <p className="text-sm mt-0.5" style={{ color: "var(--color-text-tertiary)" }}>
            配置平台参数和集成
          </p>
        </div>
        <button onClick={handleSave} disabled={saving} className="btn-primary">
          <Save className="w-4 h-4" />
          {saving ? "保存中..." : "保存设置"}
        </button>
      </div>

      {/* LLM 设置 */}
      <div className="panel">
        <div className="flex items-center gap-2 mb-4">
          <div className="p-1.5 rounded-lg" style={{ background: "var(--color-primary-subtle)" }}>
            <Key className="w-4 h-4" style={{ color: "var(--color-primary)" }} />
          </div>
          <h2 className="panel-title">LLM 设置</h2>
        </div>
        <div>
          <label className="label">PRD 生成方式</label>
          <select value={prdMode} onChange={(e) => setPrdMode(e.target.value)} className="select">
            <option value="api">API 接入 - 调用 Anthropic API（需配置下方 API Key）</option>
            <option value="agent">Claude CLI - 使用本机 Claude 命令行登录态（无需 API Key）</option>
          </select>
          <p className="text-xs mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
            对「生成 PRD」「智能完善」统一生效；CLI 通道使用 Agent 页检测到的本机 Claude 命令行
          </p>
        </div>

        <div>
          <label className="label">Claude API Key</label>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="sk-ant-..."
            className="input"
          />
          <p className="text-xs mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
            用于调用 Claude API 生成 PRD 文档（生成方式为 API 接入时必填）
          </p>
        </div>
      </div>

      {/* 会话设置 */}
      <div className="panel">
        <div className="flex items-center gap-2 mb-4">
          <div className="p-1.5 rounded-lg" style={{ background: "var(--color-primary-subtle)" }}>
            <Terminal className="w-4 h-4" style={{ color: "var(--color-primary)" }} />
          </div>
          <h2 className="panel-title">会话设置</h2>
        </div>
        <div>
          <label className="label">会话模式</label>
          <select value={sessionMode} onChange={(e) => setSessionMode(e.target.value)} className="select">
            <option value="auto">自动模式 - 任务状态变更时自动创建会话</option>
            <option value="manual">手动模式 - 需要手动创建会话</option>
          </select>
          <p className="text-xs mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
            自动模式下，任务进入"开发中"状态时将自动创建 Agent 会话
          </p>
        </div>

        <div>
          <label className="label">最大并发会话数</label>
          <input
            type="number"
            min={1}
            max={16}
            value={maxSessions}
            onChange={(e) => setMaxSessions(e.target.value)}
            className="input"
          />
          <p className="text-xs mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
            结构化 Agent 会话的同时运行上限（默认 3，范围 1–16），超出时新会话会被拒绝
          </p>
        </div>

        <div>
          <label className="label">默认 Agent</label>
          <select value={defaultAgent} onChange={(e) => setDefaultAgent(e.target.value)} className="select">
            <option value="">自动选择 - 使用第一个可用的 Agent</option>
            {agentsList.map((a) => (
              <option key={a.id} value={a.binary_path}>
                {a.display_name}（{a.status === "unavailable" ? "已失效" : "可用"}）
              </option>
            ))}
          </select>
          <p className="text-xs mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
            任务触发会话与 PRD 的 CLI 通道将优先使用此 Agent（MA-09）
          </p>
        </div>

        <div>
          <label className="label">默认工具白名单</label>
          <input
            type="text"
            value={allowedTools}
            onChange={(e) => setAllowedTools(e.target.value)}
            placeholder="WebSearch, WebFetch"
            className="input font-mono text-sm"
          />
          <p className="text-xs mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
            逗号分隔；新会话启动时预授权这些工具，避免调用即被权限策略拒绝。会话运行中被拒的工具也可在消息卡片中放行并沉淀到该会话白名单。
          </p>
        </div>
      </div>

      {/* 项目设置 */}
      <div className="panel">
        <div className="flex items-center gap-2 mb-4">
          <div className="p-1.5 rounded-lg" style={{ background: "var(--color-success-light)" }}>
            <FolderOpen className="w-4 h-4" style={{ color: "var(--color-success)" }} />
          </div>
          <h2 className="panel-title">项目设置</h2>
        </div>
        <div className="space-y-4">
          <div>
            <label className="label">项目名称</label>
            <input type="text" value={projectName} onChange={(e) => setProjectName(e.target.value)} className="input" />
          </div>
          <div>
            <label className="label">项目路径</label>
            <input
              type="text"
              value={projectPath}
              onChange={(e) => setProjectPath(e.target.value)}
              placeholder="E:\projects\my-project"
              className="input"
            />
          </div>
        </div>
      </div>

      {/* Git 设置 */}
      <div className="panel">
        <div className="flex items-center gap-2 mb-4">
          <div className="p-1.5 rounded-lg" style={{ background: "rgba(124, 58, 237, 0.08)" }}>
            <Info className="w-4 h-4" style={{ color: "#7c3aed" }} />
          </div>
          <h2 className="panel-title">Git 设置</h2>
        </div>
        <div className="space-y-4">
          <div>
            <label className="label">自动提交模式</label>
            <select value={autoCommit} onChange={(e) => setAutoCommit(e.target.value)} className="select">
              <option value="auto">自动提交</option>
              <option value="semi-auto">半自动（需确认）</option>
              <option value="manual">手动提交</option>
            </select>
          </div>
          <div>
            <label className="label">Commit 消息模板</label>
            <input
              type="text"
              value={commitTemplate}
              onChange={(e) => setCommitTemplate(e.target.value)}
              className="input font-mono text-sm"
            />
            <p className="text-xs mt-1.5" style={{ color: "var(--color-text-tertiary)" }}>
              可用变量: {"{task_id}"} {"{task_title}"} {"{dispatch_id}"} {"{agent_type}"}
            </p>
          </div>
        </div>
      </div>

      {/* PRD Skill 模板 */}
      <div className="panel">
        <div className="flex items-center gap-2 mb-4">
          <div className="p-1.5 rounded-lg" style={{ background: "var(--color-warning-light)" }}>
            <FileText className="w-4 h-4" style={{ color: "var(--color-warning)" }} />
          </div>
          <h2 className="panel-title">PRD Skill 模板</h2>
        </div>
        <div className="space-y-2">
          {skills.length === 0 ? (
            <div className="empty-state py-6">
              <p className="empty-state-desc">暂无模板</p>
            </div>
          ) : (
            skills.map((skill) => (
              <div
                key={skill.id}
                className="rounded-lg overflow-hidden"
                style={{ border: "1px solid var(--color-border-subtle)" }}
              >
                <div
                  className="flex items-center justify-between p-3 cursor-pointer transition-colors"
                  style={{ transition: "background var(--transition-fast)" }}
                  onClick={() => setExpandedSkill(expandedSkill === skill.id ? null : skill.id)}
                  onMouseEnter={(e) => (e.currentTarget.style.background = "var(--color-primary-subtle)")}
                  onMouseLeave={(e) => (e.currentTarget.style.background = "")}
                >
                  <div className="flex items-center gap-2.5">
                    <span
                      className="px-2 py-0.5 text-xs font-medium rounded"
                      style={{ background: "var(--color-primary-subtle)", color: "var(--color-primary)" }}
                    >
                      {skill.skill_type}
                    </span>
                    <span className="text-sm font-medium" style={{ color: "var(--color-text)" }}>
                      {skill.name}
                    </span>
                    {skill.is_builtin && (
                      <span
                        className="px-1.5 py-0.5 text-[10px] font-medium rounded"
                        style={{ background: "var(--color-bg-subtle)", color: "var(--color-text-tertiary)" }}
                      >
                        内置
                      </span>
                    )}
                  </div>
                  {expandedSkill === skill.id ? (
                    <ChevronUp className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
                  ) : (
                    <ChevronDown className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
                  )}
                </div>
                {expandedSkill === skill.id && (
                  <div className="px-3 pb-3 pt-0" style={{ borderTop: "1px solid var(--color-border-subtle)" }}>
                    <p className="text-sm mt-3 mb-2" style={{ color: "var(--color-text-secondary)" }}>
                      {skill.description}
                    </p>
                    <pre
                      className="text-xs p-3 rounded-lg overflow-auto max-h-40 font-mono"
                      style={{ color: "var(--color-text-tertiary)", background: "var(--color-bg-subtle)" }}
                    >
                      {skill.template}
                    </pre>
                  </div>
                )}
              </div>
            ))
          )}
        </div>
      </div>

      {/* 应用更新 */}
      <div className="rounded-xl border p-4" style={{ borderColor: "var(--color-border)" }}>
        <div className="flex items-center gap-2 mb-3">
          <RefreshCw className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
          <h3 className="text-sm font-semibold" style={{ color: "var(--color-text)" }}>
            应用更新
          </h3>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={handleCheckUpdate}
            disabled={updateChecking}
            className="px-3 py-1.5 text-sm font-medium rounded-lg transition-colors disabled:opacity-50"
            style={{ background: "var(--color-bg-subtle)", color: "var(--color-text)" }}
          >
            {updateChecking ? "检查中..." : "检查更新"}
          </button>
          {updateMessage && (
            <span className="text-sm" style={{ color: "var(--color-text-secondary)" }}>
              {updateMessage}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
