import { useEffect, useState } from "react";
import { CheckCircle, XCircle, AlertCircle, Info, X } from "lucide-react";

export type ToastType = "success" | "error" | "warning" | "info";

interface Toast {
  id: string;
  type: ToastType;
  message: string;
}

let toastId = 0;
let addToastFn: ((type: ToastType, message: string) => void) | null = null;

export function showToast(type: ToastType, message: string) {
  if (addToastFn) {
    addToastFn(type, message);
  }
}

export default function ToastContainer() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  useEffect(() => {
    addToastFn = (type, message) => {
      const id = String(++toastId);
      setToasts((prev) => [...prev, { id, type, message }]);

      // 自动移除
      setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== id));
      }, 3000);
    };

    return () => {
      addToastFn = null;
    };
  }, []);

  function removeToast(id: string) {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }

  function getIcon(type: ToastType) {
    switch (type) {
      case "success":
        return <CheckCircle className="w-5 h-5" style={{ color: "var(--color-success)" }} />;
      case "error":
        return <XCircle className="w-5 h-5" style={{ color: "var(--color-danger)" }} />;
      case "warning":
        return <AlertCircle className="w-5 h-5" style={{ color: "var(--color-warning)" }} />;
      case "info":
        return <Info className="w-5 h-5" style={{ color: "var(--color-primary)" }} />;
    }
  }

  function getBgColor(type: ToastType) {
    switch (type) {
      case "success":
        return { background: "var(--color-success-light)", borderColor: "var(--color-success)" };
      case "error":
        return { background: "var(--color-danger-light)", borderColor: "var(--color-danger)" };
      case "warning":
        return { background: "var(--color-warning-light)", borderColor: "var(--color-warning)" };
      case "info":
        return { background: "var(--color-primary-subtle)", borderColor: "var(--color-primary)" };
    }
  }

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 space-y-2">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className="flex items-center gap-3 px-4 py-3 rounded-lg border shadow-lg animate-slide-in"
          style={getBgColor(toast.type)}
        >
          {getIcon(toast.type)}
          <span className="text-sm" style={{ color: "var(--color-text)" }}>{toast.message}</span>
          <button
            onClick={() => removeToast(toast.id)}
            className="ml-2 p-0.5 rounded"
            style={{ transition: "background var(--transition-fast)" }}
            onMouseEnter={(e) => e.currentTarget.style.background = "rgba(0,0,0,0.08)"}
            onMouseLeave={(e) => e.currentTarget.style.background = ""}
          >
            <X className="w-4 h-4" style={{ color: "var(--color-text-tertiary)" }} />
          </button>
        </div>
      ))}
    </div>
  );
}
