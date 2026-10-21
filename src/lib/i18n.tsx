/**
 * 极简渐进式 i18n（零运行时依赖）。
 *
 * 策略：**中文即源文案**——组件里直接写中文原文作为 fallback，
 * `t(key, zhFallback)` 优先查当前语言字典，查不到回退中文。
 * 英文字典按模块逐步补充（先覆盖侧边栏 / 设置页等核心 chrome），
 * 未翻译的文案自动显示中文，不会出现空白或 key 裸露。
 *
 * 语言选择持久化在 localStorage（`ace.locale`）。
 */
import { createContext, useCallback, useContext, useState, type ReactNode } from "react";

export type Locale = "zh-CN" | "en-US";

export const LOCALES: { value: Locale; label: string }[] = [
  { value: "zh-CN", label: "中文" },
  { value: "en-US", label: "English" },
];

/** 英文字典：key 与组件中的 t() 调用一一对应 */
const en: Record<string, string> = {
  // 侧边栏导航
  "nav.workspace": "Workspace",
  "nav.data": "Data",
  "nav.dashboard": "Dashboard",
  "nav.projects": "Projects",
  "nav.requirements": "Requirements",
  "nav.board": "Task Board",
  "nav.agents": "Agents",
  "nav.sessions": "Sessions",
  "nav.skills": "Skills",
  "nav.commits": "Commits",
  "nav.statistics": "Statistics",
  "nav.cost": "Token Usage",
  "nav.auditLog": "Audit Log",
  "nav.importExport": "Import / Export",
  "nav.systemMonitor": "System Monitor",
  "nav.settings": "Settings",
  "status.ready": "Ready",

  // 设置页
  "settings.title": "Settings",
  "settings.interface": "Interface",
  "settings.language": "Language",
  "settings.languageHint": "Untranslated text falls back to Chinese",
  "settings.save": "Save",
  "settings.saving": "Saving…",
  "settings.apiKey": "Claude API Key",
  "settings.project": "Project",
  "settings.git": "Git",
  "settings.session": "Session",
  "settings.prd": "PRD Generation",
  "settings.skills": "PRD Skills",
  "settings.appUpdate": "App Update",
  "settings.checkUpdate": "Check for Updates",
  "settings.checking": "Checking…",
  "settings.maxSessions": "Max Concurrent Sessions",
  "settings.maxSessionsHint":
    "Concurrent structured agent session limit (default 3, range 1–16); new sessions are rejected beyond the limit",
  "settings.sessionMode": "Session Mode",
  "settings.autoMode": "Auto — create sessions on task status change",
  "settings.manualMode": "Manual — create sessions explicitly",
};

const dictionaries: Record<Locale, Record<string, string>> = {
  "zh-CN": {},
  "en-US": en,
};

interface I18nContextValue {
  locale: Locale;
  setLocale: (l: Locale) => void;
  /** 取当前语言文案；字典缺失时回退中文原文 */
  t: (key: string, zhFallback: string) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

function loadLocale(): Locale {
  try {
    const saved = localStorage.getItem("ace.locale");
    if (saved === "zh-CN" || saved === "en-US") return saved;
  } catch {
    // localStorage 不可用（测试环境等）时用默认
  }
  return "zh-CN";
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(loadLocale);

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    try {
      localStorage.setItem("ace.locale", l);
    } catch {
      // 忽略持久化失败
    }
  }, []);

  const t = useCallback((key: string, zhFallback: string) => dictionaries[locale]?.[key] ?? zhFallback, [locale]);

  return <I18nContext.Provider value={{ locale, setLocale, t }}>{children}</I18nContext.Provider>;
}

/** 无 Provider 时的降级上下文：默认中文、切换为空操作，保证孤立渲染不崩溃 */
const fallbackContext: I18nContextValue = {
  locale: "zh-CN",
  setLocale: () => {},
  t: (_key, zhFallback) => zhFallback,
};

export function useI18n(): I18nContextValue {
  return useContext(I18nContext) ?? fallbackContext;
}
