import { type LucideIcon, MessageSquarePlus, MessageCircle } from "lucide-react";

interface EmptyStateProps {
  icon?: LucideIcon;
  title: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
  compact?: boolean;
}

export default function EmptyState({
  icon: Icon = MessageCircle,
  title,
  description,
  actionLabel,
  onAction,
  compact = false,
}: EmptyStateProps) {
  return (
    <div
      className={`flex flex-col items-center justify-center w-full ${
        compact ? "py-8" : "py-16"
      } px-6 text-center animate-fade-slide`}
      role="status"
    >
      <div
        className={`${
          compact ? "w-10 h-10" : "w-14 h-14"
        } rounded-2xl bg-gradient-to-br from-violet-500/15 to-blue-500/10 border border-white/[0.06] flex items-center justify-center text-violet-300 mb-4`}
      >
        <Icon className={compact ? "w-5 h-5" : "w-6 h-6"} />
      </div>
      <h3 className={`font-extrabold tracking-wide text-[var(--foreground)] ${compact ? "text-sm" : "text-base"}`}>
        {title}
      </h3>
      {description && (
        <p className={`mt-1.5 text-[var(--muted-foreground)] max-w-md leading-relaxed ${compact ? "text-[11px]" : "text-xs"}`}>
          {description}
        </p>
      )}
      {actionLabel && onAction && (
        <button
          onClick={onAction}
          className="mt-5 flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-violet-600 to-fuchsia-600 hover:from-violet-500 hover:to-fuchsia-500 text-white text-xs font-extrabold rounded-lg shadow-lg shadow-violet-600/20 transition-all hover:-translate-y-0.5"
        >
          <MessageSquarePlus className="w-3.5 h-3.5" />
          {actionLabel}
        </button>
      )}
    </div>
  );
}
