import React, { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";
import { CheckCircle2, AlertCircle, Info, XCircle } from "lucide-react";

export type ToastTone = "success" | "error" | "info" | "warning";

export interface Toast {
  id: number;
  tone: ToastTone;
  title: string;
  description?: string;
  durationMs: number;
}

interface ToastContextValue {
  toasts: Toast[];
  push: (toast: Omit<Toast, "id" | "durationMs"> & { durationMs?: number }) => number;
  dismiss: (id: number) => void;
  success: (title: string, description?: string) => number;
  error: (title: string, description?: string) => number;
  info: (title: string, description?: string) => number;
  warning: (title: string, description?: string) => number;
}

const ToastContext = createContext<ToastContextValue | null>(null);

const DEFAULT_DURATION: Record<ToastTone, number> = {
  success: 3000,
  info: 3500,
  warning: 4500,
  error: 6000,
};

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextId = useRef(1);

  const dismiss = useCallback((id: number) => {
    setToasts(prev => prev.filter(t => t.id !== id));
  }, []);

  const push = useCallback<ToastContextValue["push"]>((toast) => {
    const id = nextId.current++;
    const duration = toast.durationMs ?? DEFAULT_DURATION[toast.tone];
    setToasts(prev => [...prev, { id, ...toast, durationMs: duration }]);
    if (duration > 0) {
      window.setTimeout(() => dismiss(id), duration);
    }
    return id;
  }, [dismiss]);

  const value = useMemo<ToastContextValue>(() => {
    const make = (tone: ToastTone) => (title: string, description?: string) =>
      push({ tone, title, description });
    return {
      toasts,
      push,
      dismiss,
      success: make("success"),
      error: make("error"),
      info: make("info"),
      warning: make("warning"),
    };
  }, [toasts, push, dismiss]);

  return <ToastContext.Provider value={value}>{children}</ToastContext.Provider>;
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    throw new Error("useToast must be used inside <ToastProvider>");
  }
  return ctx;
}

const TONE_STYLES: Record<ToastTone, { wrap: string; icon: string; Icon: React.ComponentType<{ className?: string }> }> = {
  success: {
    wrap: "border-emerald-500/30 bg-emerald-500/[0.08]",
    icon: "text-emerald-400",
    Icon: CheckCircle2,
  },
  error: {
    wrap: "border-rose-500/30 bg-rose-500/[0.08]",
    icon: "text-rose-400",
    Icon: XCircle,
  },
  warning: {
    wrap: "border-amber-500/30 bg-amber-500/[0.08]",
    icon: "text-amber-400",
    Icon: AlertCircle,
  },
  info: {
    wrap: "border-[var(--border)] bg-[var(--surface)]",
    icon: "text-[var(--muted-foreground)]",
    Icon: Info,
  },
};

export { TONE_STYLES };
