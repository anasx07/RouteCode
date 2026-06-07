import {
  MessageSquarePlus,
  Settings,
  Trash2,
  Download,
  Power,
  KeyRound,
  Search,
  RefreshCw,
  Infinity as InfinityIcon,
  Zap,
  type LucideIcon,
} from "lucide-react";
import type { ApprovalMode } from "../components/ModeIndicator";

export interface CommandContext {
  sessions: string[];
  activeSession: string;
  onSelectSession: (name: string) => void;
  onNewSession: () => void;
  onClearChat: () => void;
  onOpenSettings: () => void;
  onExportLogs: () => void;
  onQuit: () => void;
  onCheckUpdate: () => void;
  onToggleQir: () => void;
  onToggleApprovalMode: () => void;
  quickInfiniteRetry: boolean;
  approvalMode: ApprovalMode;
}

export interface Command {
  id: string;
  label: string;
  description?: string;
  icon: LucideIcon;
  shortcut?: string;
  keywords: string[];
  group: "Session" | "App" | "Settings";
  visible: (ctx: CommandContext) => boolean;
  run: (ctx: CommandContext) => void;
}

export function buildCommands(): Command[] {
  return [
    {
      id: "session.new",
      label: "New Session",
      description: "Create a fresh session with a chosen name",
      icon: MessageSquarePlus,
      shortcut: "Ctrl N",
      keywords: ["create", "session", "new", "start"],
      group: "Session",
      visible: () => true,
      run: ctx => ctx.onNewSession(),
    },
    {
      id: "session.clear",
      label: "Clear Chat History",
      description: "Wipe messages from the current session",
      icon: Trash2,
      shortcut: "Ctrl L",
      keywords: ["clear", "reset", "wipe", "history"],
      group: "Session",
      visible: () => true,
      run: ctx => ctx.onClearChat(),
    },
    {
      id: "session.export",
      label: "Export Conversation Log",
      description: "Save the current session as Markdown",
      icon: Download,
      keywords: ["export", "save", "download", "markdown"],
      group: "Session",
      visible: () => true,
      run: ctx => ctx.onExportLogs(),
    },
    {
      id: "session.switch.<id>",
      label: "Switch Session",
      description: "Jump to another saved session",
      icon: Search,
      keywords: ["switch", "open", "session"],
      group: "Session",
      visible: ctx => ctx.sessions.length > 0,
      run: () => {},
    },
    {
      id: "app.settings",
      label: "Open Settings",
      description: "Provider, model, API keys, sandbox rules",
      icon: Settings,
      shortcut: "Ctrl ,",
      keywords: ["settings", "preferences", "config"],
      group: "App",
      visible: () => true,
      run: ctx => ctx.onOpenSettings(),
    },
    {
      id: "app.update",
      label: "Check for Updates",
      description: "Query the RouteCode release feed",
      icon: RefreshCw,
      keywords: ["update", "upgrade", "version"],
      group: "App",
      visible: () => true,
      run: ctx => ctx.onCheckUpdate(),
    },
    {
      id: "app.quit",
      label: "Exit RouteCode",
      description: "Close the application",
      icon: Power,
      shortcut: "Ctrl Q",
      keywords: ["quit", "exit", "close"],
      group: "App",
      visible: () => true,
      run: ctx => ctx.onQuit(),
    },
    {
      id: "settings.api_keys",
      label: "Manage API Keys",
      description: "Jump to the API Keys tab in Settings",
      icon: KeyRound,
      keywords: ["keys", "api", "tokens", "secrets"],
      group: "Settings",
      visible: () => true,
      run: ctx => ctx.onOpenSettings(),
    },
    {
      id: "settings.qir",
      label: "Toggle Quick Infinite Retry",
      description: "Enable or disable the experimental QIR loop",
      icon: InfinityIcon,
      keywords: ["qir", "retry", "loop", "experimental"],
      group: "Settings",
      visible: () => true,
      run: ctx => ctx.onToggleQir(),
    },
    {
      id: "settings.approval_mode",
      label: "Toggle Approval Mode (Normal <-> YOLO)",
      description:
        "YOLO auto-allows every tool call. Normal requires confirmation. The header indicator shows the current mode.",
      icon: Zap,
      shortcut: "Shift Tab",
      keywords: ["yolo", "normal", "approval", "auto", "allow", "mode", "tool", "shift tab"],
      group: "Settings",
      visible: () => true,
      run: ctx => ctx.onToggleApprovalMode(),
    },
  ];
}

export function buildSessionSwitchCommands(sessions: string[], activeSession: string): Command[] {
  return sessions
    .filter(s => s !== activeSession)
    .map<Command>(s => ({
      id: `session.switch.${s}`,
      label: `Switch to "${s}"`,
      description: "Open this saved session",
      icon: Search,
      keywords: ["switch", "open", s.toLowerCase()],
      group: "Session",
      visible: () => true,
      run: ctx => ctx.onSelectSession(s),
    }));
}

export function filterCommands(commands: Command[], query: string): Command[] {
  const q = query.trim().toLowerCase();
  if (!q) return commands;
  return commands.filter(cmd => {
    const haystack = [
      cmd.label.toLowerCase(),
      cmd.description?.toLowerCase() ?? "",
      cmd.keywords.join(" ").toLowerCase(),
      cmd.group.toLowerCase(),
    ].join(" ");
    return haystack.includes(q);
  });
}
