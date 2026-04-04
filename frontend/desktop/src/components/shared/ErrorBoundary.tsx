import React, { type ErrorInfo, type ReactNode } from "react";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
  componentStack: string | null;
  source: "render" | "runtime" | null;
}

/**
 * Prevents renderer failures from collapsing the whole window into a blank view.
 *
 * React error boundaries only catch render/lifecycle failures, so this component
 * also listens for top-level runtime errors and unhandled promise rejections.
 */
export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  public state: ErrorBoundaryState = {
    error: null,
    componentStack: null,
    source: null,
  };

  public static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return {
      error,
      source: "render",
    };
  }

  public componentDidMount(): void {
    window.addEventListener("error", this.handleWindowError);
    window.addEventListener("unhandledrejection", this.handleUnhandledRejection);
  }

  public componentWillUnmount(): void {
    window.removeEventListener("error", this.handleWindowError);
    window.removeEventListener("unhandledrejection", this.handleUnhandledRejection);
  }

  public componentDidCatch(error: Error, info: ErrorInfo): void {
    this.setState({ componentStack: info.componentStack ?? null });
    console.error("[renderer] React error boundary caught an error:", error, info);
  }

  private readonly handleWindowError = (event: ErrorEvent): void => {
    const error = event.error instanceof Error
      ? event.error
      : new Error(event.message || "Unknown renderer error");
    this.setState({
      error,
      componentStack: null,
      source: "runtime",
    });
    console.error("[renderer] Unhandled window error:", error);
  };

  private readonly handleUnhandledRejection = (event: PromiseRejectionEvent): void => {
    const error = event.reason instanceof Error
      ? event.reason
      : new Error(String(event.reason ?? "Unknown unhandled rejection"));
    this.setState({
      error,
      componentStack: null,
      source: "runtime",
    });
    console.error("[renderer] Unhandled promise rejection:", error);
  };

  public render(): ReactNode {
    const { error, componentStack, source } = this.state;
    if (!error) {
      return this.props.children;
    }

    return (
      <div className="flex min-h-screen items-center justify-center bg-surface px-6 py-10 text-fg">
        <div className="flex w-full max-w-3xl flex-col gap-5 rounded-2xl border border-edge bg-panel p-6 shadow-[0_18px_48px_rgba(0,0,0,0.28)]">
          <div className="flex flex-col gap-2">
            <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-fg-subtle">
              Renderer Failure
            </p>
            <h1 className="text-2xl font-semibold text-fg">The desktop UI crashed.</h1>
            <p className="text-sm leading-6 text-fg-muted">
              Maestro caught the renderer failure instead of leaving the window blank. Reload the app,
              and if this keeps happening, copy the details below with the pipeline debug log.
            </p>
          </div>

          <div className="rounded-xl border border-edge bg-input-bg p-4">
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-fg-subtle">
              {source === "render" ? "React render error" : "Runtime error"}
            </p>
            <p className="mt-2 whitespace-pre-wrap break-words font-mono text-xs leading-6 text-fg">
              {error.stack || error.message}
            </p>
            {componentStack && (
              <pre className="mt-4 whitespace-pre-wrap break-words border-t border-edge pt-4 font-mono text-[11px] leading-5 text-fg-muted">
                {componentStack.trim()}
              </pre>
            )}
          </div>

          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => window.location.reload()}
              className="rounded-lg border border-edge bg-elevated px-4 py-2 text-sm font-medium text-fg transition-colors hover:bg-active"
            >
              Reload App
            </button>
          </div>
        </div>
      </div>
    );
  }
}
