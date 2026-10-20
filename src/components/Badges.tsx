// 共享Badge组件库 - 统一所有页面的状态、优先级、类型标签样式

interface BadgeProps {
  children: React.ReactNode;
  variant?: "default" | "success" | "warning" | "danger" | "info" | "purple";
  size?: "sm" | "md";
  dot?: boolean;
}

const variantStyles: Record<string, string> = {
  default: "bg-gray-100 text-gray-600",
  success: "bg-emerald-50 text-emerald-700 ring-1 ring-emerald-600/10",
  warning: "bg-amber-50 text-amber-700 ring-1 ring-amber-600/10",
  danger: "bg-red-50 text-red-700 ring-1 ring-red-600/10",
  info: "bg-blue-50 text-blue-700 ring-1 ring-blue-600/10",
  purple: "bg-violet-50 text-violet-700 ring-1 ring-violet-600/10",
};

const dotColors: Record<string, string> = {
  default: "bg-gray-400",
  success: "bg-emerald-500",
  warning: "bg-amber-500",
  danger: "bg-red-500",
  info: "bg-blue-500",
  purple: "bg-violet-500",
};

const sizeStyles = {
  sm: "px-1.5 py-0.5 text-[10px]",
  md: "px-2 py-0.5 text-xs",
};

export function Badge({ children, variant = "default", size = "md", dot }: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center gap-1 font-medium rounded-md ${variantStyles[variant]} ${sizeStyles[size]}`}
    >
      {dot && <span className={`w-1.5 h-1.5 rounded-full ${dotColors[variant]}`} />}
      {children}
    </span>
  );
}

// 状态标签 - 需求/任务状态
const statusConfig: Record<string, { variant: BadgeProps["variant"]; label: string }> = {
  draft: { variant: "default", label: "草稿" },
  reviewing: { variant: "warning", label: "评审中" },
  approved: { variant: "success", label: "已批准" },
  rejected: { variant: "danger", label: "已拒绝" },
  frozen: { variant: "info", label: "已冻结" },
  in_development: { variant: "purple", label: "开发中" },
  testing: { variant: "info", label: "测试中" },
  completed: { variant: "success", label: "已完成" },
  running: { variant: "info", label: "运行中" },
  failed: { variant: "danger", label: "失败" },
  available: { variant: "success", label: "可用" },
  unavailable: { variant: "default", label: "不可用" },
};

export function StatusBadge({ status }: { status: string }) {
  const config = statusConfig[status] || statusConfig.draft;
  return <Badge variant={config.variant}>{config.label}</Badge>;
}

// 优先级标签
const priorityConfig: Record<string, { variant: BadgeProps["variant"]; label: string }> = {
  P0: { variant: "danger", label: "P0 紧急" },
  P1: { variant: "warning", label: "P1 高" },
  P2: { variant: "info", label: "P2 中" },
  P3: { variant: "default", label: "P3 低" },
};

export function PriorityBadge({ priority }: { priority: string }) {
  const config = priorityConfig[priority] || priorityConfig.P1;
  return <Badge variant={config.variant}>{config.label}</Badge>;
}

// 类型标签 - 任务类型
const taskTypeConfig: Record<string, { variant: BadgeProps["variant"]; label: string }> = {
  requirement: { variant: "info", label: "需求" },
  development: { variant: "success", label: "开发" },
  testing: { variant: "purple", label: "测试" },
  bug_fix: { variant: "danger", label: "Bug修复" },
  review: { variant: "warning", label: "评审" },
};

export function TaskTypeBadge({ type }: { type: string }) {
  const config = taskTypeConfig[type] || taskTypeConfig.development;
  return (
    <Badge variant={config.variant} size="sm">
      {config.label}
    </Badge>
  );
}

// 类型标签 - 需求类型
const reqTypeConfig: Record<string, { variant: BadgeProps["variant"]; label: string }> = {
  feature: { variant: "info", label: "功能" },
  bug: { variant: "danger", label: "Bug" },
  optimization: { variant: "success", label: "优化" },
  tech_debt: { variant: "purple", label: "技术债" },
};

export function ReqTypeBadge({ type }: { type: string }) {
  const config = reqTypeConfig[type] || reqTypeConfig.feature;
  return <Badge variant={config.variant}>{config.label}</Badge>;
}
