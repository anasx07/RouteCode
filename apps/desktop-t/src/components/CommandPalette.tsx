import { useEffect, useMemo, useRef, useState } from "react";
import { Search, Command as CommandIcon, CornerDownLeft, ArrowUp, ArrowDown } from "lucide-react";
import {
  buildCommands,
  buildSessionSwitchCommands,
  filterCommands,
  type Command,
  type CommandContext,
} from "../lib/commands";

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  ctx: CommandContext;
}

const MAX_VISIBLE = 8;

export default function CommandPalette({ isOpen, onClose, ctx }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Build the live command list with per-session switchers.
  const allCommands = useMemo(() => {
    const base = buildCommands();
    const sessionSwitchers = buildSessionSwitchCommands(ctx.sessions, ctx.activeSession);
    return [...base, ...sessionSwitchers];
  }, [ctx.sessions, ctx.activeSession]);

  // Apply visibility filter + text query.
  const filtered = useMemo(() => {
    const visible = allCommands.filter(c => c.visible(ctx));
    return filterCommands(visible, query);
  }, [allCommands, query, ctx]);

  // Group commands by group while preserving order.
  const grouped = useMemo(() => {
    const groups: Record<string, Command[]> = {};
    filtered.forEach(cmd => {
      (groups[cmd.group] ??= []).push(cmd);
    });
    return groups;
  }, [filtered]);

  // Flat ordered list used for keyboard navigation.
  const flat = useMemo(() => {
    const order: string[] = ["Session", "App", "Settings"];
    const result: Command[] = [];
    for (const g of order) {
      if (grouped[g]) result.push(...grouped[g]);
    }
    return result;
  }, [grouped]);

  // Reset state when the palette opens.
  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setSelectedIdx(0);
      const t = window.setTimeout(() => inputRef.current?.focus(), 30);
      return () => window.clearTimeout(t);
    }
  }, [isOpen]);

  // Keep selection in range as the list shrinks.
  useEffect(() => {
    if (selectedIdx >= flat.length) setSelectedIdx(Math.max(0, flat.length - 1));
  }, [flat.length, selectedIdx]);

  // Scroll the selected item into view.
  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(`[data-cmd-idx="${selectedIdx}"]`);
    if (el) el.scrollIntoView({ block: "nearest" });
  }, [selectedIdx]);

  if (!isOpen) return null;

  const runCommand = (cmd: Command) => {
    onClose();
    // Defer to next tick so the palette can close before the action mutates state.
    window.setTimeout(() => cmd.run(ctx), 0);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIdx(i => (flat.length === 0 ? 0 : (i + 1) % flat.length));
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIdx(i => (flat.length === 0 ? 0 : (i - 1 + flat.length) % flat.length));
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const cmd = flat[selectedIdx];
      if (cmd) runCommand(cmd);
    }
  };

  // Build an index for each flat command so the keyboard nav matches the rendered list.
  let runningIdx = 0;
  const visible = flat.slice(0, MAX_VISIBLE);

  return (
    <div
      className="fixed inset-0 z-[90] flex items-start justify-center pt-[15vh] bg-black/60 backdrop-blur-md animate-fade-in"
      onMouseDown={e => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="w-[560px] max-w-[90vw] bg-[#0c0d14]/95 border border-white/[0.08] rounded-2xl shadow-[0_24px_80px_rgba(0,0,0,0.7)] overflow-hidden animate-modal-scale"
        role="dialog"
        aria-label="Command palette"
      >
        {/* Search input */}
        <div className="flex items-center gap-3 px-4 py-3 border-b border-white/[0.04]">
          <Search className="w-4 h-4 text-[var(--muted-foreground)] shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={e => {
              setQuery(e.target.value);
              setSelectedIdx(0);
            }}
            onKeyDown={onKeyDown}
            placeholder="Type a command or search sessions..."
            className="flex-1 bg-transparent border-none outline-none text-sm text-[var(--foreground)] placeholder-[#707070]"
            autoComplete="off"
            spellCheck={false}
          />
          <span className="hidden sm:flex items-center gap-1 text-[10px] font-mono text-[var(--muted-foreground)] bg-white/[0.04] px-1.5 py-0.5 rounded">
            <CommandIcon className="w-2.5 h-2.5" /> K
          </span>
        </div>

        {/* Results */}
        <div
          ref={listRef}
          className="max-h-[50vh] overflow-y-auto py-1.5 custom-scrollbar"
        >
          {flat.length === 0 && (
            <div className="px-4 py-8 text-center text-xs text-[var(--muted-foreground)]">
              No commands match "{query}".
            </div>
          )}

          {(["Session", "App", "Settings"] as const).map(group => {
            const cmds = grouped[group] ?? [];
            if (cmds.length === 0) return null;
            return (
              <div key={group} className="flex flex-col">
                <div className="px-4 py-1.5 text-[9px] font-black uppercase tracking-wider text-[var(--muted-foreground)]">
                  {group}
                </div>
                {cmds.map(cmd => {
                  if (cmd && visible.includes(cmd)) {
                    const idx = runningIdx;
                    runningIdx++;
                    const isSelected = idx === selectedIdx;
                    const Icon = cmd.icon;
                    return (
                      <button
                        key={cmd.id}
                        data-cmd-idx={idx}
                        onClick={() => runCommand(cmd)}
                        onMouseEnter={() => setSelectedIdx(idx)}
                        className={`flex items-center gap-3 w-full px-4 py-2 text-left transition-colors ${
                          isSelected
                            ? "bg-violet-500/10 text-[var(--foreground)]"
                            : "text-[var(--muted-foreground)] hover:bg-white/[0.03]"
                        }`}
                      >
                        <Icon className={`w-3.5 h-3.5 shrink-0 ${isSelected ? "text-violet-300" : "text-[var(--muted-foreground)]"}`} />
                        <div className="flex-1 min-w-0 flex flex-col">
                          <span className="text-xs font-bold truncate">{cmd.label}</span>
                          {cmd.description && (
                            <span className="text-[10px] text-[var(--muted-foreground)] truncate">
                              {cmd.description}
                            </span>
                          )}
                        </div>
                        {cmd.shortcut && (
                          <span className="hidden sm:inline-block text-[9px] font-mono text-[var(--muted-foreground)] bg-white/[0.04] px-1.5 py-0.5 rounded">
                            {cmd.shortcut}
                          </span>
                        )}
                      </button>
                    );
                  }
                  // Skip the index bump for items past the visible window.
                  return null;
                })}
              </div>
            );
          })}

          {flat.length > MAX_VISIBLE && (
            <div className="px-4 py-2 text-[10px] text-[var(--muted-foreground)] text-center">
              {flat.length - MAX_VISIBLE} more — refine your search.
            </div>
          )}
        </div>

        {/* Footer hint */}
        <div className="flex items-center justify-between px-4 py-2 border-t border-white/[0.04] bg-black/20 text-[10px] text-[var(--muted-foreground)]">
          <div className="flex items-center gap-3">
            <span className="flex items-center gap-1">
              <ArrowUp className="w-2.5 h-2.5" />
              <ArrowDown className="w-2.5 h-2.5" /> navigate
            </span>
            <span className="flex items-center gap-1">
              <CornerDownLeft className="w-2.5 h-2.5" /> run
            </span>
            <span>esc to close</span>
          </div>
          <span>{flat.length} command{flat.length === 1 ? "" : "s"}</span>
        </div>
      </div>
    </div>
  );
}
