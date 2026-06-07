import { ShieldCheck, Zap } from "lucide-react";

export type ApprovalMode = "normal" | "yolo";

interface ModeIndicatorProps {
  mode: ApprovalMode;
  onToggle: () => void;
}

export default function ModeIndicator({ mode, onToggle }: ModeIndicatorProps) {
  const isYolo = mode === "yolo";
  return (
    <button
      type="button"
      onClick={onToggle}
      className={`flex items-center gap-1.5 px-2 py-1 rounded text-[10px] font-mono uppercase tracking-wider border transition-colors ${
        isYolo
          ? "bg-amber-500/[0.08] border-amber-500/30 text-amber-300 hover:bg-amber-500/[0.12]"
          : "bg-emerald-500/[0.06] border-emerald-500/25 text-emerald-300 hover:bg-emerald-500/[0.10]"
      }`}
      title={
        isYolo
          ? "YOLO: tool calls are auto-allowed. Click or press Shift+Tab to switch to Normal (confirm every tool call)."
          : "Normal: every tool call needs approval. Click or press Shift+Tab to switch to YOLO (auto-allow)."
      }
      aria-label={`Approval mode: ${isYolo ? "YOLO" : "Normal"}. Click to toggle.`}
    >
      {isYolo ? (
        <Zap className="w-3 h-3" />
      ) : (
        <ShieldCheck className="w-3 h-3" />
      )}
      <span>{isYolo ? "YOLO" : "Normal"}</span>
    </button>
  );
}
