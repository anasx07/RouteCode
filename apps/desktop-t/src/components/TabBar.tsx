import { useEffect, useRef, useState } from "react";
import { Plus, X, MessageSquare, MoreHorizontal } from "lucide-react";

interface TabBarProps {
  sessions: string[];
  activeSession: string;
  onSelectSession: (name: string) => void;
  onNewSession: () => void;
  onDeleteSession: (name: string) => void;
  onRenameSession?: (oldName: string, newName: string) => void;
}

export default function TabBar({
  sessions,
  activeSession,
  onSelectSession,
  onNewSession,
  onDeleteSession,
  onRenameSession,
}: TabBarProps) {
  const stripRef = useRef<HTMLDivElement>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [draftName, setDraftName] = useState<string>("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  useEffect(() => {
    const el = stripRef.current?.querySelector(
      `[data-tab="${CSS.escape(activeSession)}"]`
    );
    if (el && stripRef.current) {
      const strip = stripRef.current;
      const elRect = (el as HTMLElement).offsetLeft;
      const elWidth = (el as HTMLElement).offsetWidth;
      const scrollLeft = strip.scrollLeft;
      const stripWidth = strip.clientWidth;
      if (elRect < scrollLeft || elRect + elWidth > scrollLeft + stripWidth) {
        strip.scrollTo({
          left: elRect - stripWidth / 2 + elWidth / 2,
          behavior: "smooth",
        });
      }
    }
  }, [activeSession, sessions]);

  const beginEdit = (name: string) => {
    if (sessions.length <= 1) return;
    setEditing(name);
    setDraftName(name);
  };

  const commitEdit = () => {
    if (!editing) return;
    const trimmed = draftName.trim();
    if (trimmed && trimmed !== editing && onRenameSession) {
      onRenameSession(editing, trimmed);
    }
    setEditing(null);
    setDraftName("");
  };

  return (
    <div className="flex items-stretch w-full bg-[var(--background)] border-b border-[var(--border)] select-none">
      <div
        ref={stripRef}
        className="flex-1 flex items-stretch overflow-x-auto overflow-y-hidden custom-scrollbar"
        style={{ minHeight: 36 }}
      >
        {sessions.length === 0 && (
          <div className="flex items-center px-4 text-[11px] text-[var(--muted-foreground)] font-mono">
            No sessions. Click + to start.
          </div>
        )}
        {sessions.map(name => {
          const isActive = activeSession === name;
          const isEditing = editing === name;
          return (
            <div
              key={name}
              data-tab={name}
              role="tab"
              aria-selected={isActive}
              onDoubleClick={() => beginEdit(name)}
              className={`group relative flex items-center gap-2 px-3.5 cursor-pointer shrink-0 min-w-[120px] max-w-[240px] border-r border-[var(--border)] transition-colors ${
                isActive
                  ? "bg-[var(--surface)] text-[var(--foreground)]"
                  : "bg-transparent text-[var(--muted-foreground)] hover:bg-[var(--surface)]/40 hover:text-[var(--foreground)]"
              }`}
            >
              {isActive && (
                <span className="absolute left-0 right-0 top-0 h-[2px] bg-gradient-to-r from-violet-500 via-fuchsia-500 to-violet-500" />
              )}
              <MessageSquare
                className={`w-3.5 h-3.5 shrink-0 ${
                  isActive ? "text-violet-400" : "text-[var(--muted-foreground)]"
                }`}
              />
              {isEditing ? (
                <input
                  ref={inputRef}
                  value={draftName}
                  onChange={e => setDraftName(e.target.value)}
                  onBlur={commitEdit}
                  onKeyDown={e => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      commitEdit();
                    } else if (e.key === "Escape") {
                      e.preventDefault();
                      setEditing(null);
                      setDraftName("");
                    }
                  }}
                  className="flex-1 min-w-0 bg-transparent border-b border-violet-500/40 outline-none text-[12px] font-mono text-[var(--foreground)]"
                />
              ) : (
                <button
                  onClick={() => onSelectSession(name)}
                  className="flex-1 min-w-0 text-left text-[12px] font-mono truncate"
                  title={name}
                >
                  {name}
                </button>
              )}
              {sessions.length > 1 && !isEditing && (
                <button
                  onClick={e => {
                    e.stopPropagation();
                    onDeleteSession(name);
                  }}
                  className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-rose-500/20 text-[var(--muted-foreground)] hover:text-rose-400 transition-all shrink-0"
                  title="Close tab"
                  aria-label={`Close ${name}`}
                >
                  <X className="w-3 h-3" />
                </button>
              )}
              {onRenameSession && !isEditing && (
                <button
                  onClick={e => {
                    e.stopPropagation();
                    beginEdit(name);
                  }}
                  className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-[var(--secondary)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] transition-all shrink-0"
                  title="Rename tab (double-click)"
                  aria-label={`Rename ${name}`}
                >
                  <MoreHorizontal className="w-3 h-3" />
                </button>
              )}
            </div>
          );
        })}
      </div>
      <button
        onClick={onNewSession}
        className="flex items-center justify-center px-3 border-l border-[var(--border)] text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--surface)]/60 transition-colors shrink-0"
        title="New tab (Ctrl+N)"
        aria-label="New tab"
      >
        <Plus className="w-3.5 h-3.5" />
      </button>
    </div>
  );
}
