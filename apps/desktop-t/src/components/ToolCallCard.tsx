import React, { useState } from "react";
import {
  Terminal,
  CheckCircle2,
  XCircle,
  Loader2,
  Ban,
  ChevronDown,
  ChevronRight,
  FileEdit,
  FileSearch,
  FileText,
  Wrench,
  Clock,
} from "lucide-react";
import { HighlightedSpan } from "../lib/textHighlight";
import type { ToolEvent } from "../App";

interface ToolCallCardProps {
  event: ToolEvent;
}

const ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  bash: Terminal,
  read: FileText,
  edit: FileEdit,
  write: FileEdit,
  glob: FileSearch,
  grep: FileSearch,
  ls: FileSearch,
  default: Wrench,
};

const STATUS_STYLES: Record<
  ToolEvent["status"],
  { border: string; bg: string; pill: string; icon: React.ComponentType<{ className?: string }>; label: string }
> = {
  running: {
    border: "border-amber-500/30",
    bg: "bg-amber-500/[0.04]",
    pill: "bg-amber-500/[0.10] text-amber-300 border-amber-500/30",
    icon: Loader2,
    label: "running",
  },
  success: {
    border: "border-emerald-500/25",
    bg: "bg-emerald-500/[0.04]",
    pill: "bg-emerald-500/[0.10] text-emerald-300 border-emerald-500/25",
    icon: CheckCircle2,
    label: "ok",
  },
  error: {
    border: "border-rose-500/30",
    bg: "bg-rose-500/[0.04]",
    pill: "bg-rose-500/[0.10] text-rose-300 border-rose-500/30",
    icon: XCircle,
    label: "error",
  },
  denied: {
    border: "border-zinc-500/30",
    bg: "bg-zinc-500/[0.04]",
    pill: "bg-zinc-500/[0.10] text-zinc-300 border-zinc-500/30",
    icon: Ban,
    label: "denied",
  },
  pending: {
    border: "border-zinc-500/30",
    bg: "bg-zinc-500/[0.04]",
    pill: "bg-zinc-500/[0.10] text-zinc-300 border-zinc-500/30",
    icon: Clock,
    label: "pending",
  },
};

function formatArgs(name: string, args: string): React.ReactNode {
  // Try to render the most common tool shapes with a typed header; fall back
  // to the raw JSON pretty-printed.
  let parsed: Record<string, unknown> | null = null;
  try {
    const p = JSON.parse(args);
    if (p && typeof p === "object" && !Array.isArray(p)) parsed = p as Record<string, unknown>;
  } catch {
    return (
      <pre className="text-[12px] font-mono text-[var(--foreground)] whitespace-pre-wrap break-all">
        {args}
      </pre>
    );
  }
  if (!parsed) return null;

  // Bash
  if (name === "bash") {
    const cmd = typeof parsed.command === "string" ? parsed.command : "";
    const cwd = typeof parsed.cwd === "string" ? parsed.cwd : "";
    return (
      <div className="flex flex-col gap-1.5">
        {cmd && (
          <code className="block text-[12.5px] font-mono text-[var(--foreground)] whitespace-pre-wrap break-words">
            <HighlightedSpan>{cmd}</HighlightedSpan>
          </code>
        )}
        {cwd && (
          <div className="text-[10.5px] font-mono text-[var(--muted-foreground)]">
            cwd: <HighlightedSpan>{cwd}</HighlightedSpan>
          </div>
        )}
      </div>
    );
  }

  // File ops (read/edit/write)
  if (name === "read" || name === "edit" || name === "write") {
    const path = typeof parsed.path === "string" ? parsed.path : "";
    if (!path) return null;
    return (
      <code className="block text-[12.5px] font-mono text-[var(--foreground)] whitespace-pre-wrap break-all">
        <HighlightedSpan>{path}</HighlightedSpan>
      </code>
    );
  }

  // Glob/grep/ls
  if (name === "glob" || name === "grep" || name === "ls") {
    const pat = (parsed.pattern ?? parsed.path ?? parsed.query ?? "") as string;
    const dir = (parsed.path ?? parsed.dir ?? parsed.cwd ?? "") as string;
    return (
      <div className="flex flex-col gap-1 text-[12.5px] font-mono">
        {pat !== "" && (
          <div>
            <span className="text-[var(--muted-foreground)]">{name}:</span>{" "}
            <HighlightedSpan>{String(pat)}</HighlightedSpan>
          </div>
        )}
        {dir && (
          <div className="text-[10.5px] text-[var(--muted-foreground)]">
            in <HighlightedSpan>{dir}</HighlightedSpan>
          </div>
        )}
      </div>
    );
  }

  // Generic
  return (
    <pre className="text-[12px] font-mono text-[var(--foreground)] whitespace-pre-wrap break-all">
      {JSON.stringify(parsed, null, 2)}
    </pre>
  );
}

function DiffPreview({ diff }: { diff: string }) {
  const lines = diff.split(/\r?\n/);
  return (
    <div className="overflow-x-auto rounded border border-[var(--border)] bg-[#0d0a0a] font-mono text-[11.5px] leading-snug">
      {lines.map((line, i) => {
        const trimmed = line.length > 240 ? line.slice(0, 240) + "…" : line;
        let cls = "text-[var(--muted-foreground)]";
        if (line.startsWith("+") && !line.startsWith("+++")) cls = "text-emerald-300 bg-emerald-500/[0.06]";
        else if (line.startsWith("-") && !line.startsWith("---")) cls = "text-rose-300 bg-rose-500/[0.06]";
        else if (line.startsWith("@@")) cls = "text-sky-300";
        else if (line.startsWith("diff ") || line.startsWith("index ")) cls = "text-[var(--muted-foreground)]";
        return (
          <div key={i} className={`px-3 whitespace-pre ${cls}`}>
            {trimmed || "\u00a0"}
          </div>
        );
      })}
    </div>
  );
}

function ResultBody({ event }: { event: ToolEvent }) {
  if (event.status === "denied") {
    return (
      <div className="text-[12px] font-mono text-[var(--muted-foreground)] italic">
        User denied this tool call.
      </div>
    );
  }
  if (event.status === "pending" || event.status === "running") {
    return (
      <div className="flex items-center gap-2 text-[11.5px] font-mono text-[var(--muted-foreground)]">
        <Loader2 className="w-3 h-3 animate-spin" /> awaiting response…
      </div>
    );
  }
  if (event.resultError) {
    return (
      <pre className="overflow-x-auto rounded border border-rose-500/25 bg-rose-500/[0.04] px-3 py-2 text-[12px] font-mono text-rose-300 whitespace-pre-wrap break-words">
        {event.resultError.length > 800
          ? event.resultError.slice(0, 800) + "…"
          : event.resultError}
      </pre>
    );
  }
  if (event.resultDiff && event.resultDiff.trim()) {
    return <DiffPreview diff={event.resultDiff} />;
  }
  if (event.resultContent) {
    const content = event.resultContent;
    const truncated = content.length > 600;
    return (
      <pre className="overflow-x-auto rounded border border-[var(--border)] bg-[#0d0a0a] px-3 py-2 text-[12px] font-mono text-[var(--foreground)] whitespace-pre-wrap break-words">
        {truncated ? content.slice(0, 600) + "…" : content}
      </pre>
    );
  }
  return (
    <div className="text-[11.5px] font-mono text-[var(--muted-foreground)] italic">
      (no output)
    </div>
  );
}

export default function ToolCallCard({ event }: ToolCallCardProps) {
  const [open, setOpen] = useState(false);
  const Icon = ICONS[event.name] ?? ICONS.default;
  const style = STATUS_STYLES[event.status];
  const StatusIcon = style.icon;
  const isSpinning = event.status === "running";
  const hasBody =
    event.status === "denied" ||
    event.status === "pending" ||
    event.resultError ||
    event.resultDiff ||
    event.resultContent;

  const elapsed = (() => {
    if (!event.finishedAt) return null;
    const ms = event.finishedAt - event.startedAt;
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  })();

  return (
    <div
      className={`w-full max-w-[85%] self-start rounded-md border ${style.border} ${style.bg} animate-fade-slide`}
    >
      <button
        type="button"
        onClick={() => setOpen(o => !o)}
        className="flex w-full items-center gap-2.5 px-3 py-2 text-left hover:bg-white/[0.02] transition-colors"
      >
        {open ? (
          <ChevronDown className="w-3.5 h-3.5 text-[var(--muted-foreground)] shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-[var(--muted-foreground)] shrink-0" />
        )}
        <Icon className="w-3.5 h-3.5 text-[var(--muted-foreground)] shrink-0" />
        <span className="font-mono text-[12.5px] text-[var(--foreground)] shrink-0">
          {event.name}
        </span>
        <span
          className={`inline-flex items-center gap-1 rounded border px-1.5 py-0.5 text-[10px] font-mono uppercase tracking-wide ${style.pill}`}
        >
          <StatusIcon className={`w-2.5 h-2.5 ${isSpinning ? "animate-spin" : ""}`} />
          {style.label}
        </span>
        {elapsed && (
          <span className="text-[10px] font-mono text-[var(--muted-foreground)] ml-auto">
            {elapsed}
          </span>
        )}
      </button>

      {open && (
        <div className="border-t border-[var(--border)] px-3 py-2.5 flex flex-col gap-2.5">
          <div className="flex flex-col gap-1">
            <div className="text-[10px] font-mono uppercase tracking-wide text-[var(--muted-foreground)]">
              args
            </div>
            {formatArgs(event.name, event.args)}
          </div>
          {hasBody && (
            <div className="flex flex-col gap-1">
              <div className="text-[10px] font-mono uppercase tracking-wide text-[var(--muted-foreground)]">
                {event.status === "denied" ? "note" : "result"}
              </div>
              <ResultBody event={event} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
