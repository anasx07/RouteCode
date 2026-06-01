import { X, Download, Loader2, CheckCircle2 } from "lucide-react";

interface UpdateInfo {
  version: string;
  current_version: string;
  changelog: string;
  published_at: string;
  is_update_available: boolean;
}

interface UpdateModalProps {
  isOpen: boolean;
  updateInfo: UpdateInfo | null;
  onClose: () => void;
  onInstall: () => void;
  isInstalling: boolean;
  installComplete: boolean;
}

export default function UpdateModal({
  isOpen,
  updateInfo,
  onClose,
  onInstall,
  isInstalling,
  installComplete,
}: UpdateModalProps) {
  if (!isOpen || !updateInfo) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm animate-modal-scale">
      <div className="w-full max-w-md mx-4 bg-[#0a0a14] border border-[#8b5cf6]/30 rounded-2xl shadow-2xl shadow-[#8b5cf6]/10 overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-white/[0.04]">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-xl bg-gradient-to-br from-[#8b5cf6] to-[#db2777] flex items-center justify-center">
              <Download className="w-4 h-4 text-white" />
            </div>
            <div>
              <h2 className="font-bold text-sm text-gray-100">Update Available</h2>
              <p className="text-[10px] text-gray-500 font-mono">
                v{updateInfo.current_version} → v{updateInfo.version}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={isInstalling}
            className="p-1.5 rounded-lg hover:bg-white/[0.04] transition-all cursor-pointer disabled:opacity-30"
          >
            <X className="w-4 h-4 text-gray-500" />
          </button>
        </div>

        <div className="px-6 py-4">
          <h3 className="text-xs font-bold text-gray-400 uppercase tracking-wider mb-2">
            What's New
          </h3>
          <div className="max-h-48 overflow-y-auto pr-2 custom-scrollbar">
            <div className="text-xs text-gray-400 leading-relaxed whitespace-pre-wrap font-mono">
              {updateInfo.changelog || "No release notes available."}
            </div>
          </div>
        </div>

        <div className="px-6 py-4 border-t border-white/[0.04] flex items-center justify-end gap-3">
          {!installComplete && !isInstalling && (
            <>
              <button
                onClick={onClose}
                className="px-5 py-2.5 text-xs font-bold text-gray-400 bg-white/[0.02] border border-white/[0.05] hover:bg-white/[0.05] rounded-xl transition-all cursor-pointer"
              >
                Skip
              </button>
              <button
                onClick={onInstall}
                className="px-5 py-2.5 text-xs font-bold text-white bg-gradient-to-r from-[#8b5cf6] to-[#db2777] hover:from-[#7c3aed] hover:to-[#be185d] rounded-xl transition-all cursor-pointer shadow-lg shadow-[#8b5cf6]/20"
              >
                Download & Install
              </button>
            </>
          )}

          {isInstalling && !installComplete && (
            <div className="flex items-center gap-3 px-5 py-2.5">
              <Loader2 className="w-4 h-4 text-[#8b5cf6] animate-spin" />
              <span className="text-xs font-bold text-gray-300">Downloading update...</span>
            </div>
          )}

          {installComplete && (
            <div className="flex items-center gap-3 px-5 py-2.5">
              <CheckCircle2 className="w-4 h-4 text-emerald-500" />
              <span className="text-xs font-bold text-emerald-400">
                Update installed! Please restart the app.
              </span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
