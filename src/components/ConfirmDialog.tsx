import { AlertTriangle, X } from "lucide-react";

interface ConfirmDialogProps {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** 红色主按钮（删除等危险操作） */
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

/** 统一确认弹窗（取代原生 confirm()；危险操作红色主按钮 + 警示图标） */
export default function ConfirmDialog({
  title,
  message,
  confirmLabel = "确定",
  cancelLabel = "取消",
  danger = false,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 420 }}>
        <div className="modal-header">
          <div className="flex items-center gap-2">
            {danger && (
              <span
                className="w-7 h-7 rounded-lg flex items-center justify-center flex-shrink-0"
                style={{ background: "var(--color-danger-light)" }}
              >
                <AlertTriangle className="w-4 h-4" style={{ color: "var(--color-danger)" }} />
              </span>
            )}
            <h3 className="text-base font-semibold" style={{ color: "var(--color-text)" }}>
              {title}
            </h3>
          </div>
          <button
            onClick={onCancel}
            className="p-1 rounded-lg"
            style={{ color: "var(--color-text-tertiary)" }}
            aria-label="关闭"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
        <div className="modal-body">
          <p className="text-sm" style={{ color: "var(--color-text-secondary)", whiteSpace: "pre-line" }}>
            {message}
          </p>
        </div>
        <div className="modal-footer">
          <button onClick={onCancel} className="btn-secondary">
            {cancelLabel}
          </button>
          <button
            onClick={onConfirm}
            className="btn-primary"
            style={danger ? { background: "var(--color-danger)", borderColor: "var(--color-danger)" } : undefined}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
