import React from "react";
import { Loader2, FileSearch, Brain } from "lucide-react";

interface AgentStatusProps {
  status: string | null;
}

function classify(status: string): { icon: React.ReactNode; label: string } {
  const s = status.trim();
  const exploring = s.match(/^(?:Exploring|Reading|Scanning)\s+(?:the\s+)?(?:.*?\s+)?(\d+)\s+reads?/i);
  if (exploring) {
    return {
      icon: <FileSearch className="w-3 h-3 text-sky-400 animate-pulse" />,
      label: `Exploring ${exploring[1]} read${exploring[1] === "1" ? "" : "s"}`,
    };
  }
  const thinking = s.match(/^Thinking[:\s]+(.+)/i);
  if (thinking) {
    return {
      icon: <Brain className="w-3 h-3 text-violet-400 animate-pulse" />,
      label: `Thinking ${thinking[1].trim()}`,
    };
  }
  const reading = s.match(/^(?:Reading|Scanning)\s+(.+)/i);
  if (reading) {
    return {
      icon: <FileSearch className="w-3 h-3 text-sky-400 animate-pulse" />,
      label: `Reading ${reading[1].trim()}`,
    };
  }
  return {
    icon: <Loader2 className="w-3 h-3 text-[var(--muted-foreground)] animate-spin" />,
    label: s,
  };
}

export default function AgentStatus({ status }: AgentStatusProps) {
  if (!status) return null;
  const { icon, label } = classify(status);
  return (
    <div
      className="flex items-center gap-2 px-1 py-1 text-[10px] font-mono text-[var(--muted-foreground)]"
      role="status"
      aria-live="polite"
    >
      {icon}
      <span className="truncate">{label}</span>
    </div>
  );
}
