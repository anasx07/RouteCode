import { Component, type ErrorInfo, type ReactNode } from "react";
import { AlertOctagon, RotateCw, Copy, Check } from "lucide-react";

interface Props {
  children: ReactNode;
  fallbackTitle?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
  copied: boolean;
}

export default class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null, copied: false };

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // The desktop-t app does not have a remote error sink; surface in the
    // console so developers can pick it up from DevTools.
    console.error("[ErrorBoundary]", error, info.componentStack);
  }

  private handleReset = () => {
    this.setState({ hasError: false, error: null, copied: false });
  };

  private handleReload = () => {
    if (typeof window !== "undefined") window.location.reload();
  };

  private handleCopy = async () => {
    const text = `${this.state.error?.name ?? "Error"}: ${this.state.error?.message ?? ""}\n${this.state.error?.stack ?? ""}`;
    try {
      await navigator.clipboard.writeText(text);
      this.setState({ copied: true });
      window.setTimeout(() => this.setState({ copied: false }), 1500);
    } catch {
      // Clipboard can be unavailable in non-secure contexts; fall back to
      // selection so the user can still grab the text.
      const ta = document.createElement("textarea");
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      try { document.execCommand("copy"); } catch { /* ignore */ }
      document.body.removeChild(ta);
    }
  };

  render() {
    if (!this.state.hasError) return this.props.children;

    return (
      <div className="flex flex-col items-center justify-center h-full w-full p-8 bg-[var(--background)] text-[var(--foreground)] font-sans animate-fade-in">
        <div className="w-[520px] max-w-full bg-[#0b0c13] border border-rose-500/30 rounded-2xl p-6 shadow-2xl shadow-black/60 flex flex-col gap-5">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-rose-500/10 border border-rose-500/20 flex items-center justify-center text-rose-400">
              <AlertOctagon className="w-5 h-5" />
            </div>
            <div className="flex flex-col">
              <h2 className="text-sm font-extrabold tracking-wide">
                {this.props.fallbackTitle ?? "Something went wrong"}
              </h2>
              <p className="text-[11px] text-[var(--muted-foreground)]">
                RouteCode hit an unexpected error. The rest of the app is unaffected.
              </p>
            </div>
          </div>

          <div className="bg-black/45 border border-white/[0.04] rounded-xl p-3 max-h-[180px] overflow-y-auto custom-scrollbar">
            <code className="text-[11px] font-mono text-rose-300/90 whitespace-pre-wrap break-words">
              {this.state.error?.message ?? "Unknown error"}
            </code>
          </div>

          <div className="flex items-center justify-end gap-2">
            <button
              onClick={this.handleCopy}
              className="flex items-center gap-1.5 px-3 py-2 text-[11px] font-bold text-[var(--muted-foreground)] hover:text-[var(--foreground)] bg-white/[0.02] border border-white/[0.04] hover:bg-white/[0.06] rounded-lg transition-colors"
            >
              {this.state.copied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
              {this.state.copied ? "Copied" : "Copy details"}
            </button>
            <button
              onClick={this.handleReset}
              className="flex items-center gap-1.5 px-3 py-2 text-[11px] font-bold text-[var(--foreground)] bg-white/[0.04] hover:bg-white/[0.10] rounded-lg transition-colors"
            >
              <RotateCw className="w-3 h-3" />
              Try again
            </button>
            <button
              onClick={this.handleReload}
              className="px-4 py-2 text-[11px] font-extrabold text-white bg-gradient-to-r from-rose-500 to-fuchsia-500 hover:from-rose-400 hover:to-fuchsia-400 rounded-lg shadow-lg shadow-rose-500/20 transition-all"
            >
              Reload app
            </button>
          </div>
        </div>
      </div>
    );
  }
}
