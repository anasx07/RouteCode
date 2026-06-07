import React, { useState, useEffect, useRef } from "react";
import { MessageSquarePlus } from "lucide-react";

interface PromptModalProps {
  isOpen: boolean;
  title?: string;
  placeholder?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

export default function PromptModal({
  isOpen,
  title = "Enter new session name:",
  placeholder = "my_new_session",
  onConfirm,
  onCancel
}: PromptModalProps) {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isOpen) {
      setValue("");
      // Focus input shortly after opening to ensure it's rendered
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim()) {
      onConfirm(value.trim());
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      onCancel();
    }
  };

  return (
    <div className="absolute inset-0 bg-black/85 backdrop-blur-xl flex items-center justify-center z-50 animate-fade-in">
      <div className="w-[490px] bg-[#0b0c13]/95 border border-white/[0.08] rounded-[24px] p-8 shadow-[0_32px_80px_rgba(0,0,0,0.8)] flex flex-col gap-6 animate-modal-scale">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 rounded-2xl bg-blue-500/10 border border-blue-500/20 text-blue-400 flex items-center justify-center shadow-lg shadow-blue-500/5">
            <MessageSquarePlus className="w-6 h-6" />
          </div>
          <div className="flex flex-col">
            <h3 className="text-base font-extrabold text-white tracking-wide">
              {title}
            </h3>
            <p className="text-[11px] text-gray-400 font-semibold tracking-wide">
              Provide a unique name for the new session.
            </p>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          <input
            ref={inputRef}
            type="text"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={placeholder}
            className="w-full bg-black/45 border border-white/[0.06] rounded-xl px-4 py-3 text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500/50 focus:ring-1 focus:ring-blue-500/50 transition-all"
          />

          <div className="flex items-center justify-end gap-3 pt-2">
            <button
              type="button"
              onClick={onCancel}
              className="px-5 py-2.5 border border-white/[0.04] hover:bg-white/[0.06] hover:text-white text-xs font-bold text-gray-400 rounded-xl transition-all duration-300 cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!value.trim()}
              className="px-6 py-2.5 bg-gradient-to-r from-blue-600 to-indigo-600 hover:from-blue-500 hover:to-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed text-white text-xs font-black rounded-xl shadow-lg shadow-blue-600/15 hover:shadow-blue-600/25 transition-all duration-300 hover:-translate-y-0.5 active:translate-y-0 cursor-pointer"
            >
              OK
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
