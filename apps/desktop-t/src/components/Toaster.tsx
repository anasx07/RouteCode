import { useToast, TONE_STYLES } from "../lib/toast";
import { X } from "lucide-react";

export default function Toaster() {
  const { toasts, dismiss } = useToast();
  if (toasts.length === 0) return null;

  return (
    <div
      className="fixed top-4 right-4 z-[100] flex flex-col gap-2 pointer-events-none"
      role="region"
      aria-label="Notifications"
      aria-live="polite"
    >
      {toasts.map(toast => {
        const style = TONE_STYLES[toast.tone];
        const { Icon } = style;
        return (
          <div
            key={toast.id}
            className={`pointer-events-auto flex items-start gap-3 min-w-[280px] max-w-[380px] pl-3 pr-2 py-2.5 rounded-lg border backdrop-blur-xl shadow-2xl shadow-black/40 animate-toast-slide-in ${style.wrap}`}
            role="status"
          >
            <Icon className={`w-4 h-4 mt-0.5 shrink-0 ${style.icon}`} />
            <div className="flex-1 min-w-0 flex flex-col gap-0.5">
              <span className="text-xs font-bold text-[var(--foreground)] tracking-wide truncate">
                {toast.title}
              </span>
              {toast.description && (
                <span className="text-[11px] text-[var(--muted-foreground)] leading-relaxed">
                  {toast.description}
                </span>
              )}
            </div>
            <button
              onClick={() => dismiss(toast.id)}
              className="shrink-0 p-1 rounded text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-white/[0.06] transition-colors"
              aria-label="Dismiss"
            >
              <X className="w-3 h-3" />
            </button>
          </div>
        );
      })}
    </div>
  );
}
