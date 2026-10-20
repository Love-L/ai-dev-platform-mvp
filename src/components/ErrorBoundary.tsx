import { Component, type ReactNode } from "react";
import { AlertTriangle, RefreshCw } from "lucide-react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error("ErrorBoundary caught:", error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="flex items-center justify-center h-full p-6">
          <div className="text-center max-w-md">
            <div
              className="p-3 rounded-full w-fit mx-auto mb-4"
              style={{ background: 'var(--color-danger-light)' }}
            >
              <AlertTriangle className="w-8 h-8" style={{ color: 'var(--color-danger)' }} />
            </div>
            <h2 className="text-lg font-semibold mb-2" style={{ color: 'var(--color-text)' }}>
              页面出现错误
            </h2>
            <p className="text-sm mb-4" style={{ color: 'var(--color-text-tertiary)' }}>
              {this.state.error?.message || "发生了未知错误"}
            </p>
            <button
              onClick={this.handleReset}
              className="btn-primary"
            >
              <RefreshCw className="w-4 h-4" />
              重试
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
