import { ShieldAlert, Check, X, Repeat, FolderOpen } from "lucide-react";

interface ConfirmationModalProps {
  isOpen: boolean;
  command: string;
  cwd: string;
  toolName?: string;
  onAllow: (scope: "once" | "session" | "workspace") => void;
  onDeny: () => void;
}

export default function ConfirmationModal({
  isOpen,
  command,
  cwd,
  toolName,
  onAllow,
  onDeny,
}: ConfirmationModalProps) {
  if (!isOpen) return null;

  return (
    <div className="absolute inset-0 bg-black/85 backdrop-blur-xl flex items-center justify-center z-50 animate-fade-in">
      <div className="w-[520px] bg-[#0b0c13]/95 border border-white/[0.08] rounded-[24px] p-8 shadow-[0_32px_80px_rgba(0,0,0,0.8)] flex flex-col gap-6 animate-modal-scale">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-2xl bg-amber-500/10 border border-amber-500/20 text-amber-500 flex items-center justify-center shadow-lg shadow-amber-500/5">
            <ShieldAlert className="w-6 h-6 animate-bounce" style={{ animationDuration: '2s' }} />
          </div>
          <div className="flex flex-col">
            <h3 className="text-base font-extrabold text-white tracking-wide">
              Workspace Permission
            </h3>
            <p className="text-[11px] text-gray-400 font-semibold tracking-wide">
              An agent requires authorization to execute a sandbox tool.
            </p>
          </div>
        </div>

        <div className="bg-black/45 border border-white/[0.03] rounded-xl p-5 font-mono text-[11.5px] text-[#cbd5e1] leading-relaxed flex flex-col gap-2 max-h-[160px] overflow-y-auto">
          <div className="flex gap-2">
            <span className="text-gray-500 shrink-0">Tool:</span>
            <span className="text-sky-300 font-bold">{toolName ?? "bash"}</span>
          </div>
          <div className="flex gap-2">
            <span className="text-gray-500 shrink-0">Operation:</span>
            <span className="text-white font-bold select-all break-all">{command}</span>
          </div>
          <div className="flex gap-3">
            <span className="text-gray-500 shrink-0">Directory:</span>
            <span className="text-violet-300 select-all break-all">{cwd}</span>
          </div>
          <div className="flex gap-3">
            <span className="text-gray-500 shrink-0">Boundary:</span>
            <span className="text-emerald-400 font-extrabold">SANDBOXED (RouteCode Secured)</span>
          </div>
        </div>

        <div className="flex flex-col gap-2">
          <div className="text-[10px] font-mono uppercase tracking-wide text-gray-500 px-1">
            Choose scope
          </div>
          <div className="grid grid-cols-3 gap-2">
            <button
              onClick={() => onAllow("once")}
              className="group flex flex-col items-start gap-1.5 px-3 py-3 border border-white/[0.06] hover:border-violet-500/40 hover:bg-violet-500/[0.06] rounded-xl text-left transition-all duration-200 cursor-pointer"
            >
              <span className="flex items-center gap-1.5 text-[11px] font-bold text-white">
                <Check className="w-3.5 h-3.5 text-violet-400" />
                Allow once
              </span>
              <span className="text-[10px] text-gray-500 leading-snug">
                Run this single command
              </span>
            </button>
            <button
              onClick={() => onAllow("session")}
              className="group flex flex-col items-start gap-1.5 px-3 py-3 border border-white/[0.06] hover:border-emerald-500/40 hover:bg-emerald-500/[0.06] rounded-xl text-left transition-all duration-200 cursor-pointer"
            >
              <span className="flex items-center gap-1.5 text-[11px] font-bold text-white">
                <Repeat className="w-3.5 h-3.5 text-emerald-400" />
                Allow session
              </span>
              <span className="text-[10px] text-gray-500 leading-snug">
                All commands in this tab
              </span>
            </button>
            <button
              onClick={() => onAllow("workspace")}
              className="group flex flex-col items-start gap-1.5 px-3 py-3 border border-white/[0.06] hover:border-amber-500/40 hover:bg-amber-500/[0.06] rounded-xl text-left transition-all duration-200 cursor-pointer"
            >
              <span className="flex items-center gap-1.5 text-[11px] font-bold text-white">
                <FolderOpen className="w-3.5 h-3.5 text-amber-400" />
                Allow workspace
              </span>
              <span className="text-[10px] text-gray-500 leading-snug">
                Every session in this folder
              </span>
            </button>
          </div>
        </div>

        <div className="flex items-center justify-end pt-1">
          <button
            onClick={onDeny}
            className="flex items-center gap-1.5 px-5 py-2.5 border border-white/[0.04] hover:bg-rose-500/[0.08] hover:text-rose-300 hover:border-rose-500/30 text-xs font-bold text-gray-400 rounded-xl transition-all duration-300 cursor-pointer"
          >
            <X className="w-3.5 h-3.5" />
            Deny
          </button>
        </div>
      </div>
    </div>
  );
}
