import { useState, useRef, useEffect } from "react";
import {
  Trash2,
  Download,
  Coins,
  Infinity as InfinityIcon,
  Command as CommandIcon,
  Search,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Import Modular Components
import TabBar from "./components/TabBar";
import SettingsModal from "./components/SettingsModal";
import ConfirmationModal from "./components/ConfirmationModal";
import ChatArea from "./components/ChatArea";
import ChatInput from "./components/ChatInput";
import UpdateModal from "./components/UpdateModal";
import PromptModal from "./components/PromptModal";
import Toaster from "./components/Toaster";
import CommandPalette from "./components/CommandPalette";
import ErrorBoundary from "./components/ErrorBoundary";
import { ToastProvider, useToast } from "./lib/toast";
import type { CommandContext } from "./lib/commands";
import { fetchProviderModels, type FetchResult } from "./lib/models";
import ModeIndicator, { type ApprovalMode } from "./components/ModeIndicator";

export type ToolEventStatus = "pending" | "running" | "success" | "error" | "denied";

export interface ToolEvent {
  id: string;
  name: string;
  args: string;
  status: ToolEventStatus;
  resultRaw?: string;
  resultContent?: string;
  resultError?: string;
  resultDiff?: string;
  resultSuccess?: boolean;
  startedAt: number;
  finishedAt?: number;
}

interface Message {
  id: string;
  sender: "user" | "assistant" | "system-success" | "system-error";
  text: string;
  model?: string;
  thought?: string;
  isStreaming?: boolean;
  qirStatus?: string;
  toolEvents?: ToolEvent[];
}

// SDK-compatible message shape
interface SDKMessage {
  role: "system" | "user" | "assistant" | "tool";
  content: string | null;
  reasoning_content?: string | null;
}

const PROVIDER_OPTIONS = [
  { id: "anthropic", label: "Anthropic" },
  { id: "openai", label: "OpenAI" },
  { id: "openrouter", label: "OpenRouter" },
  { id: "deepseek", label: "DeepSeek" },
  { id: "google", label: "Google" },
  { id: "nvidia", label: "NVIDIA NIM" },
  { id: "cloudflare-workers", label: "Cloudflare Workers" },
];

export default function App() {
  return (
    <ErrorBoundary>
      <ToastProvider>
        <AppInner />
        <Toaster />
      </ToastProvider>
    </ErrorBoundary>
  );
}

function AppInner() {
  // Session States
  const [sessions, setSessions] = useState<string[]>([]);
  const [activeSession, setActiveSession] = useState<string>("default_session");
  const [messages, setMessages] = useState<Message[]>([
    {
      id: "welcome",
      sender: "assistant",
      text: "Hi! I'm RouteCode, your AI agent pair programmer. I have complete secure access to your workspace tools and can assist you with building high-performance systems.\n\nHow can I help you today?",
      thought: "Analyzing routecode-sdk capabilities...\nWorkspace initialized.\nReady to assist user with codebase modifications, analysis, or secure command executions."
    }
  ]);
  
  // Settings Panel States
  const [showSettings, setShowSettings] = useState<boolean>(false);
  
  // Persistent SDK Config Values
  const [activeProvider, setActiveProvider] = useState<string>("anthropic");
  const [activeModel, setActiveModel] = useState<string>("claude-sonnet-4-5");
  const [quickInfiniteRetry, setQuickInfiniteRetry] = useState<boolean>(false);
  const [approvalMode, setApprovalMode] = useState<ApprovalMode>("normal");
  const [apiKeys, setApiKeys] = useState<Record<string, string>>({
    anthropic: "",
    openai: "",
    openrouter: "",
    deepseek: "",
    google: "",
    nvidia: "",
    "cloudflare-workers": ""
  });
  
  // UI & UX States
  const [inputValue, setInputValue] = useState<string>("");
  const [isGenerating, setIsGenerating] = useState<boolean>(false);
  const [qirRetryStatus, setQirRetryStatus] = useState<string | null>(null);
  const [agentStatus, setAgentStatus] = useState<string | null>(null);
  const [sessionStats, setSessionStats] = useState<{ totalTokens: number; totalCost: number; qirAttempts: number }>({
    totalTokens: 0,
    totalCost: 0,
    qirAttempts: 0,
  });

  const [showUpdateModal, setShowUpdateModal] = useState<boolean>(false);
  const [updateInfo, setUpdateInfo] = useState<any>(null);
  const [isInstalling, setIsInstalling] = useState<boolean>(false);
  const [installComplete, setInstallComplete] = useState<boolean>(false);

  const [promptModalOpen, setPromptModalOpen] = useState<boolean>(false);

  // Per-provider model cache. Populated by fetching from the provider's
  // `/models` endpoint (same path the CLI uses) when the user opens the
  // model menu or saves a new API key. `null` means "not yet fetched".
  const [providerModels, setProviderModels] = useState<Record<string, FetchResult>>({});
  const [fetchingProviderFor, setFetchingProviderFor] = useState<string | null>(null);

  // Global shell state
  const [commandPaletteOpen, setCommandPaletteOpen] = useState<boolean>(false);
  const toast = useToast();

  const [expandedThoughts, setExpandedThoughts] = useState<Record<string, boolean>>({
    welcome: true
  });

  const [modalOpen, setModalOpen] = useState<boolean>(false);
  const [modalDetails, setModalDetails] = useState<{
    command: string;
    cwd: string;
    toolName: string;
  }>({ command: "", cwd: "", toolName: "bash" });

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const isTauri = typeof window !== "undefined" && (window as any).__TAURI_INTERNALS__ !== undefined;

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${textareaRef.current.scrollHeight}px`;
    }
  }, [inputValue]);

  // Initial Load of Saved Config and Sessions
  useEffect(() => {
    if (isTauri) {
      // Load configuration from persistent routecode storage
      invoke("get_config")
        .then((cfg: any) => {
          console.log("Loaded persistent config:", cfg);
          if (cfg.provider) setActiveProvider(cfg.provider);
          if (cfg.model) setActiveModel(cfg.model);
          // Read the retry policy. Accept both the new tagged shape
          // (`retry_policy: {strategy: "qir"}`) and the legacy bool
          // (`quick_infinite_retry: true`) for backward compat.
          const policy = cfg.retry_policy;
          if (policy && typeof policy === "object" && "strategy" in policy) {
            setQuickInfiniteRetry(policy.strategy === "qir");
          } else if (typeof cfg.quick_infinite_retry === "boolean") {
            setQuickInfiniteRetry(cfg.quick_infinite_retry);
          }
          if (cfg.api_keys) {
            setApiKeys(prev => ({
              ...prev,
              ...cfg.api_keys
            }));
          }
          if (cfg.approval_mode) {
            const am = cfg.approval_mode;
            if (am && typeof am === "object" && "strategy" in am) {
              setApprovalMode(am.strategy === "yolo" ? "yolo" : "normal");
            }
          }

          // Once config loads, trigger active engine initialization
          invoke("init_engine", { providerName: cfg.provider || "anthropic", modelName: cfg.model || "claude-sonnet-4-5" })
            .catch(err => console.error("Initial init_engine failed:", err));

          // Auto-fetch models for the active provider. Uses the same flow as
          // the CLI's `/model` command. Result populates the model menu
          // dropdown so the user can pick a real model without typing the id.
          const activeProv = cfg.provider || "anthropic";
          const activeKey = (cfg.api_keys && cfg.api_keys[activeProv]) || "";
          handleFetchProviderModels(activeProv, activeKey);
        })
        .catch(err => console.error("Failed to load config:", err));

      // Load session names list from workspace sessions folder
      refreshSessionsList();
    } else {
      // Mock sessions for Web preview
      setSessions(["default_session", "code_refactor_sandbox", "dependency_hardening"]);
    }
  }, [isTauri]);

  useEffect(() => {
    if (isTauri) {
      const timer = setTimeout(() => {
        invoke("check_update")
          .then((result: any) => {
            const info = typeof result === "string" ? JSON.parse(result) : result;
            if (info && info.is_update_available) {
              setUpdateInfo(info);
              setShowUpdateModal(true);
              toast.info("Update available", `Version ${info.version}`);
            }
          })
          .catch(err => console.error("Update check failed:", err));
      }, 5000);

      return () => clearTimeout(timer);
    }
  }, [isTauri, toast]);

  // Global keyboard shortcuts. Only Ctrl/Cmd-modified shortcuts to avoid
  // hijacking normal typing in the chat input. Handlers are read through
  // refs so we can reference them in the listener without depending on
  // their (re-created) identity every render.
  const handlerRefs = useRef({
    newSession: () => {},
    clearChat: () => {},
  });
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const isEditable =
        !!target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable);
      const mod = e.ctrlKey || e.metaKey;

      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setCommandPaletteOpen(p => !p);
        return;
      }
      if (mod && e.key.toLowerCase() === "n" && !isEditable) {
        e.preventDefault();
        handlerRefs.current.newSession();
        return;
      }
      if (mod && e.key.toLowerCase() === "l" && !isEditable) {
        e.preventDefault();
        handlerRefs.current.clearChat();
        return;
      }
      if (mod && e.key === ",") {
        e.preventDefault();
        setShowSettings(true);
        return;
      }
      if (e.key === "Tab" && e.shiftKey) {
        e.preventDefault();
        handleToggleApprovalMode();
        return;
      }
      if (e.key === "Escape" && commandPaletteOpen) {
        setCommandPaletteOpen(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [commandPaletteOpen]);

  const refreshSessionsList = (selectSessionName?: string) => {
    if (isTauri) {
      invoke("list_saved_sessions")
        .then((list: any) => {
          const sessionsList = list || [];
          setSessions(sessionsList);
          if (sessionsList.length > 0) {
            const nextActive = selectSessionName || activeSession;
            if (sessionsList.includes(nextActive)) {
              setActiveSession(nextActive);
              loadActiveSessionMessages(nextActive);
            } else {
              setActiveSession(sessionsList[0]);
              loadActiveSessionMessages(sessionsList[0]);
            }
          } else {
            // Create a default session if folder is empty
            invoke("save_saved_session", {
              name: "default_session",
              messages: [
                {
                  role: "assistant",
                  content: "Welcome to RouteCode! Active native SDK workspace ready.",
                  reasoning_content: "Orchestrator online."
                }
              ],
              model: activeModel
            }).then(() => {
              setSessions(["default_session"]);
              setActiveSession("default_session");
              loadActiveSessionMessages("default_session");
            });
          }
        })
        .catch(err => console.error("Failed to list sessions:", err));
    }
  };

  // Load a session's messages from disk
  const loadActiveSessionMessages = (sessionName: string) => {
    if (isTauri) {
      // Re-apply any persisted per-session permission flags (e.g. a prior
      // "Allow for this session" choice) to the live orchestrator atomics.
      invoke("set_session_permissions", { name: sessionName })
        .catch(err => console.warn("set_session_permissions failed:", err));
      invoke("load_saved_session", { name: sessionName })
        .then((session: any) => {
          console.log("Loaded messages from disk for session:", sessionName);
          const mapped = session.messages.map((m: any, idx: number) => ({
            id: `hist-${idx}-${Date.now()}`,
            sender: m.role === "user" ? "user" : m.role === "system" ? "system-success" : "assistant",
            text: m.content || "",
            thought: m.reasoning_content || undefined
          }));
          setMessages(mapped);
          if (session.model) {
            setActiveModel(session.model);
          }
        })
        .catch(err => console.error("Failed to load active session:", err));
    }
  };

  const handleToggleThought = (msgId: string) => {
    setExpandedThoughts(prev => ({
      ...prev,
      [msgId]: !prev[msgId]
    }));
  };

  // Real SDK Streaming logic using Tauri Event listeners
  const runNativeAgentFlow = async (sdkHistory: SDKMessage[], streamId: string) => {
    try {
      let accumulatedText = "";
      let accumulatedThought = "";

      const unlisten = await listen("agent-chunk", (event) => {
        const chunk = event.payload as any;
        console.log("StreamChunk Event:", chunk);

        switch (chunk.type) {
          case "text":
            accumulatedText += chunk.content;
            setMessages(prev => 
              prev.map(m => m.id === streamId ? { ...m, text: accumulatedText } : m)
            );
            break;
          case "thought":
            accumulatedThought += chunk.content;
            setMessages(prev => 
              prev.map(m => m.id === streamId ? { ...m, thought: accumulatedThought } : m)
            );
            break;
          case "request_confirmation":
            if (approvalMode === "yolo") {
              // Auto-allow without showing the modal. The user can still see
              // what ran via the ToolCallCard that follows the result chunk.
              invoke("respond_confirmation", {
                response: "allow_once",
                sessionName: activeSession,
              })
                .then(() => {
                  toast.info(
                    "Auto-allowed (YOLO)",
                    chunk.target || "tool execution"
                  );
                })
                .catch(err => console.error("YOLO auto-allow failed:", err));
            } else {
              setModalDetails({
                command: chunk.target || "Bash Sandbox Execution",
                cwd: chunk.message || "d:\\DEV\\Apps\\RouteCode",
                toolName: chunk.tool_name || "bash",
              });
              setModalOpen(true);
            }
            break;
          case "tool_call": {
            const tc = chunk.tool_call;
            if (!tc || !tc.id) break;
            const newEvent: ToolEvent = {
              id: tc.id,
              name: tc.function?.name ?? "tool",
              args: tc.function?.arguments ?? "{}",
              status: "running",
              startedAt: Date.now(),
            };
            setMessages(prev =>
              prev.map(m => {
                if (m.id !== streamId) return m;
                const events = m.toolEvents ? [...m.toolEvents] : [];
                const existingIdx = events.findIndex(e => e.id === newEvent.id);
                if (existingIdx >= 0) {
                  events[existingIdx] = { ...events[existingIdx], ...newEvent };
                } else {
                  events.push(newEvent);
                }
                return { ...m, toolEvents: events };
              })
            );
            break;
          }
          case "tool_result": {
            const toolCallId: string = chunk.tool_call_id;
            const resultContent: string = chunk.content ?? "";
            let parsed: {
              success?: boolean;
              content?: string;
              error?: string;
              diff?: string;
            } = {};
            try {
              parsed = JSON.parse(resultContent);
            } catch {
              parsed = { content: resultContent };
            }
            const success =
              parsed.success === true ||
              (parsed.error === undefined && parsed.success !== false);
            setMessages(prev =>
              prev.map(m => {
                if (m.id !== streamId) return m;
                const events = m.toolEvents ? [...m.toolEvents] : [];
                const idx = events.findIndex(e => e.id === toolCallId);
                const status: ToolEventStatus = success ? "success" : "error";
                if (idx >= 0) {
                  events[idx] = {
                    ...events[idx],
                    status,
                    resultRaw: resultContent,
                    resultContent: parsed.content,
                    resultError: parsed.error,
                    resultDiff: parsed.diff,
                    resultSuccess: success,
                    finishedAt: Date.now(),
                  };
                } else {
                  // Result arrived before/without a tool_call chunk (rare).
                  events.push({
                    id: toolCallId,
                    name: chunk.name ?? "tool",
                    args: "{}",
                    status,
                    resultRaw: resultContent,
                    resultContent: parsed.content,
                    resultError: parsed.error,
                    resultDiff: parsed.diff,
                    resultSuccess: success,
                    startedAt: Date.now(),
                    finishedAt: Date.now(),
                  });
                }
                return { ...m, toolEvents: events };
              })
            );
            break;
          }
          case "status":
            setQirRetryStatus(chunk.content);
            setMessages(prev =>
              prev.map(m => m.id === streamId ? { ...m, qirStatus: chunk.content } : m)
            );
            break;
          case "agent_status":
            setAgentStatus(chunk.content || null);
            break;
          case "session_stats":
            setSessionStats({
              totalTokens: chunk.total_tokens ?? 0,
              totalCost: chunk.total_cost ?? 0,
              qirAttempts: chunk.qir_attempts ?? 0,
            });
            break;
          case "done":
            setQirRetryStatus(null);
            setAgentStatus(null);
            setMessages(prev => {
              const updated = prev.map(m => m.id === streamId ? { ...m, isStreaming: false, qirStatus: undefined } : m);
              saveSessionToDisk(activeSession, updated);
              return updated;
            });
            setIsGenerating(false);
            unlisten();
            break;
          case "error":
            setQirRetryStatus(null);
            setAgentStatus(null);
            setMessages(prev => {
              const updated = prev.map(m => m.id === streamId ? { ...m, text: `Engine Error: ${chunk.content}`, isStreaming: false, qirStatus: undefined } : m);
              saveSessionToDisk(activeSession, updated);
              return updated;
            });
            setIsGenerating(false);
            unlisten();
            break;
          default:
            break;
        }
      });

      await invoke("send_message", { history: sdkHistory, model: activeModel });

    } catch (err) {
      console.error("Native call failed, starting simulation fallback:", err);
      runSimulatedAgentFlow(inputValue, streamId);
    }
  };

  const runSimulatedAgentFlow = (userQuery: string, streamId: string) => {
    const isCommand = userQuery.toLowerCase().includes("run") || userQuery.toLowerCase().includes("write") || userQuery.toLowerCase().includes("/");

    if (isCommand) {
      setTimeout(() => {
        setModalDetails({
          command: "cargo build --workspace",
          cwd: "d:\\DEV\\Apps\\RouteCode",
          toolName: "bash",
        });
        setModalOpen(true);
      }, 1200);
    }

    setAgentStatus("Thinking");
    setTimeout(() => setAgentStatus("Reading 1 read"), 350);
    setTimeout(() => setAgentStatus(`Exploring 3 reads`), 900);
    setTimeout(() => setAgentStatus("Thinking Fix plan:"), 1500);

    const replyText = `[Web Demo Fallback] I have received your request regarding: "${userQuery}". Since the workspace is locked securely, all code analysis, AST parsing, and modifications are verified relative to the root boundaries. How would you like me to proceed?`;
    let currentText = "";
    let index = 0;

    const interval = setInterval(() => {
      if (index < replyText.length) {
        currentText += replyText[index];
        setMessages(prev =>
          prev.map(m => m.id === streamId ? { ...m, text: currentText } : m)
        );
        index++;
      } else {
        clearInterval(interval);
        setAgentStatus(null);
        setMessages(prev => {
          const updated = prev.map(m => m.id === streamId ? { ...m, isStreaming: false } : m);
          setIsGenerating(false);
          return updated;
        });
        setTimeout(() => {
          setExpandedThoughts(prev => ({ ...prev, [streamId]: false }));
        }, 500);
      }
    }, 12);
  };

  // Helper to persist session chat histories directly to files
  const saveSessionToDisk = (sessionName: string, messagesList: Message[]) => {
    if (isTauri) {
      const sdkHistory: SDKMessage[] = messagesList
        .filter(m => m.sender === "user" || m.sender === "assistant")
        .map(m => ({
          role: m.sender === "user" ? "user" : "assistant",
          content: m.text,
          reasoning_content: m.thought || null
        }));

      invoke("save_saved_session", { name: sessionName, messages: sdkHistory, model: activeModel })
        .then(() => console.log("Session saved securely to workspace config folder"))
        .catch(err => console.error("Failed to auto-save session:", err));
    }
  };

  const handleStopGeneration = () => {
    if (isTauri) {
      invoke("cancel_message")
        .then(() => {
          setMessages(prev => [
            ...prev,
            {
              id: `sys-succ-${Date.now()}`,
              sender: "system-success",
              text: "Agent run cancelled by user."
            }
          ]);
        })
        .catch(err => {
          console.error("Failed to cancel run:", err);
          setMessages(prev => [
            ...prev,
            {
              id: `sys-err-${Date.now()}`,
              sender: "system-error",
              text: "Failed to cancel agent run."
            }
          ]);
        });
    } else {
      setIsGenerating(false);
    }
  };

  const handleSendMessage = () => {
    const query = inputValue.trim();
    if (!query || isGenerating) return;

    setIsGenerating(true);
    setQirRetryStatus(null);
    setAgentStatus("Thinking");

    const userMsgId = `user-${Date.now()}`;
    const updatedMessages: Message[] = [
      ...messages,
      {
        id: userMsgId,
        sender: "user",
        text: query
      }
    ];
    setMessages(updatedMessages);
    saveSessionToDisk(activeSession, updatedMessages);

    setInputValue("");
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }

    const streamId = `msg-${Date.now()}`;
    setMessages(prev => [
      ...prev,
      {
        id: streamId,
        sender: "assistant",
        text: "...",
        model: activeModel,
        thought: `Routing context stream to SDK engine...\nModel target: ${activeModel}\nResolving active tools: FileRead, FileWrite, FileEdit, Bash, Navigation...`,
        isStreaming: true
      }
    ]);

    setExpandedThoughts(prev => ({
      ...prev,
      [streamId]: true
    }));

    if (isTauri) {
      const sdkHistory: SDKMessage[] = updatedMessages
        .filter(m => m.sender === "user" || m.sender === "assistant")
        .map(m => ({
          role: m.sender === "user" ? "user" : "assistant",
          content: m.text,
          reasoning_content: m.thought || null
        }));

      runNativeAgentFlow(sdkHistory, streamId);
    } else {
      runSimulatedAgentFlow(query, streamId);
    }
  };

  // CRUD Operations on Saved Sessions
  const handleNewSession = () => {
    setPromptModalOpen(true);
  };

  const handleCreateSession = (name: string) => {
    setPromptModalOpen(false);
    if (name) {
      const sanitized = name.replace(/[^a-zA-Z0-9_-]/g, "");
      if (sanitized) {
        if (isTauri) {
          invoke("save_saved_session", {
            name: sanitized,
            messages: [
              {
                role: "assistant",
                content: `Workspace Session '${sanitized}' initialized.`,
                reasoning_content: "Orchestrator ready."
              }
            ],
            model: activeModel
          }).then(() => {
            refreshSessionsList(sanitized);
          });
        } else {
          setSessions(prev => [...prev, sanitized]);
          setActiveSession(sanitized);
          setMessages([
            {
              id: `welcome-${Date.now()}`,
              sender: "assistant",
              text: `Mock Session '${sanitized}' initialized.`,
              thought: "Ready."
            }
          ]);
        }
      }
    }
  };

  const handleDeleteSession = (sessionName: string) => {
    if (confirm(`Are you sure you want to delete session '${sessionName}'?`)) {
      if (isTauri) {
        invoke("delete_session", { name: sessionName })
          .then(() => {
            refreshSessionsList();
          })
          .catch(err => console.error("Failed to delete session:", err));
      } else {
        const next = sessions.filter(s => s !== sessionName);
        setSessions(next);
        if (activeSession === sessionName && next.length > 0) {
          setActiveSession(next[0]);
        }
      }
    }
  };

  const handleRenameSession = (oldName: string, newName: string) => {
    if (oldName === newName) return;
    const sanitized = newName.replace(/[^a-zA-Z0-9_-]/g, "");
    if (!sanitized) {
      toast.error("Rename failed", "Name must contain letters, numbers, _ or -");
      return;
    }
    if (isTauri) {
      invoke("load_saved_session", { name: oldName })
        .then((session: any) => {
          const messages = session?.messages ?? [];
          const model = session?.model ?? activeModel;
          invoke("save_saved_session", { name: sanitized, messages, model })
            .then(() => {
              invoke("delete_session", { name: oldName })
                .then(() => {
                  refreshSessionsList(sanitized);
                  if (activeSession === oldName) {
                    setActiveSession(sanitized);
                    setSessionStats({ totalTokens: 0, totalCost: 0, qirAttempts: 0 });
                    loadActiveSessionMessages(sanitized);
                  }
                  toast.success("Session renamed", `${oldName} -> ${sanitized}`);
                })
                .catch(err => console.error("Rename: failed to delete old session:", err));
            })
            .catch(err => console.error("Rename: failed to create new session:", err));
        })
        .catch(err => console.error("Rename: failed to load old session:", err));
    } else {
      setSessions(prev => prev.map(s => (s === oldName ? sanitized : s)));
      if (activeSession === oldName) setActiveSession(sanitized);
      toast.success("Session renamed", `${oldName} -> ${sanitized}`);
    }
  };

  const handleClearChat = () => {
    const clearedMessages: Message[] = [
      {
        id: "cleared",
        sender: "assistant",
        text: "Workspace conversation history cleared. Ask me anything!"
      }
    ];
    setMessages(clearedMessages);
    saveSessionToDisk(activeSession, clearedMessages);
  };

  // Keep the keyboard shortcut handler in sync with the latest handler
  // identities. Storing them in a ref avoids re-binding the global listener
  // on every state change.
  useEffect(() => {
    handlerRefs.current.newSession = handleNewSession;
    handlerRefs.current.clearChat = handleClearChat;
  });

  // Persists configuration modifications directly to RouteCode config.json
  const handleSaveSettings = () => {
    if (isTauri) {
      const configObj = {
        model: activeModel,
        provider: activeProvider,
        theme: "default",
        api_keys: apiKeys,
        allowlist: [],
        last_update_check: 0.0,
        favorites: [],
        recent_models: [],
        thinking_level: "default",
        logo_animation: "always",
        logo_animation_color: "rainbow",
        retry_policy: { strategy: quickInfiniteRetry ? "qir" : "disabled" },
        approval_mode: { strategy: approvalMode },
      };

      invoke("save_config", { config: configObj })
        .then(() => {
          setShowSettings(false);
          toast.success("Settings saved", `Engine reloading with ${activeProvider} / ${activeModel}`);
          // Re-initialize active SDK orchestrator with the new configuration
          invoke("init_engine", { providerName: activeProvider, modelName: activeModel })
            .then(() => {
              setMessages(prev => [
                ...prev,
                {
                  id: `sys-config-${Date.now()}`,
                  sender: "system-success",
                  text: `SDK Engine updated and re-loaded: Switched to ${activeModel} on ${activeProvider}`
                }
              ]);
              // Re-fetch models for the active provider now that the engine is
              // re-initialized. This is the same path the CLI uses when you
              // run `/model` after saving a key.
              handleFetchProviderModels(activeProvider, apiKeys[activeProvider] ?? "");
            });
        })
        .catch(err => toast.error("Failed to save config", String(err)));
    } else {
      setShowSettings(false);
      toast.info("Settings updated", "Simulation mode");
    }
  };

  // Fetch the list of available models for a given provider using the
  // configured API key. Mirrors the CLI's `handle_command("/model")` flow:
  // resolve the provider trait and call `list_models()`. Results are cached
  // in `providerModels` keyed by provider id.
  const handleFetchProviderModels = async (providerId: string, apiKey: string) => {
    if (!providerId) return;
    if (fetchingProviderFor === providerId) return;
    setFetchingProviderFor(providerId);
    try {
      const result = await fetchProviderModels(providerId, apiKey);
      setProviderModels(prev => ({ ...prev, [providerId]: result }));
      if (result.source === "live") {
        toast.success(`Models loaded`, `${result.models.length} from ${providerId}`);
      } else if (result.error) {
        toast.warning(`Using fallback models`, result.error);
      }
    } finally {
      setFetchingProviderFor(null);
    }
  };

  // Toggle the approval mode (Normal <-> YOLO) and persist it. YOLO means
  // every tool call from the agent is auto-allowed without showing the
  // confirmation modal. Plan-mode-style reads are still available via the
  // `/plan` sub-agent flow that the LLM can invoke itself.
  const handleToggleApprovalMode = () => {
    setApprovalMode(prev => {
      const next: ApprovalMode = prev === "yolo" ? "normal" : "yolo";
      if (isTauri) {
        invoke("get_config")
          .then((cfg: any) => {
            const configObj = {
              ...(cfg || {}),
              model: cfg?.model ?? activeModel,
              provider: cfg?.provider ?? activeProvider,
              approval_mode: { strategy: next },
            };
            return invoke("save_config", { config: configObj });
          })
          .then(() => {
            toast.info(
              next === "yolo" ? "YOLO mode ON" : "Normal mode ON",
              next === "yolo"
                ? "All tool calls will be auto-allowed. Use with caution."
                : "Every tool call needs your approval before execution."
            );
          })
          .catch(err => {
            toast.error("Failed to persist mode", String(err));
            setApprovalMode(prev);
          });
      } else {
        toast.info(
          next === "yolo" ? "YOLO mode ON" : "Normal mode ON",
          "Simulation mode - preference is not persisted."
        );
      }
      return next;
    });
  };

  // Build a Markdown export of the current session and trigger a browser download.
  const handleExportLogs = () => {
    try {
      const md = messages
        .filter(m => m.sender === "user" || m.sender === "assistant")
        .map(m => {
          const who = m.sender === "user" ? "User" : "Assistant";
          const thought = m.thought
            ? `\n\n<details><summary>Reasoning</summary>\n\n\`\`\`\n${m.thought}\n\`\`\`\n\n</details>\n`
            : "";
          return `## ${who}\n\n${m.text || ""}${thought}`;
        })
        .join("\n\n---\n\n");
      const header =
        `# RouteCode Session: ${activeSession}\n\n` +
        `**Model:** ${activeProvider} / ${activeModel}  \n` +
        `**Exported:** ${new Date().toISOString()}\n\n---\n\n`;
      const blob = new Blob([header + md], { type: "text/markdown" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${activeSession}.md`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
      toast.success("Session exported", `Saved as ${activeSession}.md`);
    } catch (err) {
      toast.error("Export failed", String(err));
    }
  };

  // Manually re-trigger the update check from the command palette.
  const handleCheckUpdate = () => {
    if (!isTauri) {
      toast.info("Update check", "Web preview — skipping remote call");
      return;
    }
    toast.info("Checking for updates", "Querying release feed...");
    invoke("check_update")
      .then((result: any) => {
        const info = typeof result === "string" ? JSON.parse(result) : result;
        if (info && info.is_update_available) {
          setUpdateInfo(info);
          setShowUpdateModal(true);
          toast.info("Update available", `Version ${info.version}`);
        } else {
          toast.success("Up to date", "You're on the latest RouteCode build");
        }
      })
      .catch(err => toast.error("Update check failed", String(err)));
  };

  const handleToggleQir = () => {
    setQuickInfiniteRetry(prev => {
      const next = !prev;
      toast.info(
        next ? "QIR enabled" : "QIR disabled",
        next
          ? "Failed requests will be retried indefinitely (experimental)"
          : "Failed requests will be surfaced immediately"
      );
      return next;
    });
  };

  const handleQuit = () => {
    toast.info("Goodbye", "Closing RouteCode...");
    window.setTimeout(() => {
      if (typeof window !== "undefined") window.close();
    }, 250);
  };

  const handleAllowTool = (scope: "once" | "session" | "workspace") => {
    setModalOpen(false);
    const response =
      scope === "once" ? "allow_once" :
      scope === "session" ? "allow_session" :
      "allow_workspace";
    const scopeLabel =
      scope === "once" ? "once" :
      scope === "session" ? "this session" :
      "this workspace";
    if (isTauri) {
      invoke("respond_confirmation", { response, sessionName: activeSession })
        .then(() => {
          setMessages(prev => [
            ...prev,
            {
              id: `sys-succ-${Date.now()}`,
              sender: "system-success",
              text: `Tool execution approved (${scopeLabel}) and completed successfully in sandbox.`
            }
          ]);
        })
        .catch(err => console.error("Allow error:", err));
    } else {
      setMessages(prev => [
        ...prev,
        {
          id: `sys-succ-${Date.now()}`,
          sender: "system-success",
          text: `Command 'cargo build --workspace' completed with exit status code 0.`
        }
      ]);
    }
  };

  const handleDenyTool = () => {
    setModalOpen(false);
    if (isTauri) {
      invoke("respond_confirmation", { response: "deny", sessionName: activeSession })
        .then(() => {
          setMessages(prev => [
            ...prev,
            {
              id: `sys-err-${Date.now()}`,
              sender: "system-error",
              text: "Tool permission denied by developer. Action aborted."
            }
          ]);
        })
        .catch(err => console.error("Deny error:", err));
    } else {
      setMessages(prev => [
        ...prev,
        {
          id: `sys-err-${Date.now()}`,
          sender: "system-error",
          text: "Action aborted by user. Tool execution permission rejected."
        }
      ]);
    }
  };

  const handleInstallUpdate = () => {
    setIsInstalling(true);
    invoke("install_update")
      .then(() => {
        setIsInstalling(false);
        setInstallComplete(true);
      })
      .catch(err => {
        console.error("Install error:", err);
        setIsInstalling(false);
        toast.error("Update installation failed", String(err));
      });
  };

  const handleCloseUpdate = () => {
    setShowUpdateModal(false);
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[var(--background)] text-[var(--foreground)] font-sans">
      {/* Tab strip replaces the sidebar. Sessions live as browser-style tabs at the top. */}
      <main className="relative z-10 flex-1 flex flex-col h-full overflow-hidden bg-[var(--background)]">
        <TabBar
          sessions={sessions}
          activeSession={activeSession}
          onSelectSession={(name) => {
            if (name === activeSession) return;
            setActiveSession(name);
            setSessionStats({ totalTokens: 0, totalCost: 0, qirAttempts: 0 });
            setAgentStatus(null);
            loadActiveSessionMessages(name);
          }}
          onNewSession={handleNewSession}
          onDeleteSession={handleDeleteSession}
          onRenameSession={handleRenameSession}
        />

        {/* Slim header bar showing the active session + utility actions */}
        <header className="h-[40px] bg-[var(--background)] border-b border-[var(--border)] flex items-center justify-between px-5">
          <div className="flex items-center gap-3 min-w-0">
            <span className="font-semibold text-[12px] text-[var(--foreground)] tracking-wide truncate">
              {activeSession}
            </span>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={() => setCommandPaletteOpen(true)}
              className="hidden md:flex items-center gap-2 px-2.5 py-1.5 bg-white/[0.03] hover:bg-white/[0.06] border border-[var(--border)] text-[10px] font-mono text-[var(--muted-foreground)] rounded-md transition-colors"
              title="Open command palette (Ctrl+K)"
              aria-label="Open command palette"
            >
              <Search className="w-3 h-3" />
              <span>Quick actions</span>
              <span className="flex items-center gap-0.5 ml-1 px-1 py-0.5 rounded bg-black/40 text-[9px]">
                <CommandIcon className="w-2.5 h-2.5" /> K
              </span>
            </button>
            <UsageBadge stats={sessionStats} />
            <ModeIndicator mode={approvalMode} onToggle={handleToggleApprovalMode} />
            <button
              onClick={handleClearChat}
              className="flex items-center gap-2 px-3 py-1.5 bg-transparent hover:bg-[var(--secondary)] text-xs font-medium text-[#a0a0a0] rounded-md transition-all cursor-pointer"
              title="Clear current session history (Ctrl+L)"
            >
              <Trash2 className="w-3.5 h-3.5 text-[#a0a0a0]" /> Clear Chat
            </button>
            <button
              onClick={handleExportLogs}
              className="flex items-center gap-2 px-3 py-1.5 bg-transparent hover:bg-[var(--secondary)] text-xs font-medium text-[#a0a0a0] rounded-md transition-all cursor-pointer"
              title="Export current session as Markdown"
            >
              <Download className="w-3.5 h-3.5" /> Export Logs
            </button>
          </div>
        </header>

        {/* Chat Area */}
        <ChatArea
          messages={messages}
          expandedThoughts={expandedThoughts}
          onToggleThought={handleToggleThought}
          messagesEndRef={messagesEndRef}
        />

        {/* Extracted Chat Input Bar */}
        <ChatInput
          value={inputValue}
          onChange={setInputValue}
          onSubmit={handleSendMessage}
          onStop={handleStopGeneration}
          isGenerating={isGenerating}
          qirStatus={qirRetryStatus}
          agentStatus={agentStatus}
          textareaRef={textareaRef}
          activeProvider={activeProvider}
          activeModel={activeModel}
          onChangeModel={setActiveModel}
          providers={PROVIDER_OPTIONS}
          providerModels={providerModels}
          fetchingProviderFor={fetchingProviderFor}
          onFetchProviderModels={handleFetchProviderModels}
          apiKeys={apiKeys}
        />

        {/* Extracted Safe Tool Confirmation Modal */}
        <ConfirmationModal
          isOpen={modalOpen}
          command={modalDetails.command}
          cwd={modalDetails.cwd}
          toolName={modalDetails.toolName}
          onAllow={handleAllowTool}
          onDeny={handleDenyTool}
        />

        {/* Extracted Prompt Modal */}
        <PromptModal
          isOpen={promptModalOpen}
          onConfirm={handleCreateSession}
          onCancel={() => setPromptModalOpen(false)}
        />

        {/* Extracted Translucent Settings Modal */}
        <SettingsModal
          isOpen={showSettings}
          onClose={() => setShowSettings(false)}
          activeProvider={activeProvider}
          activeModel={activeModel}
          apiKeys={apiKeys}
          quickInfiniteRetry={quickInfiniteRetry}
          onChangeProvider={setActiveProvider}
          onChangeModel={setActiveModel}
          onChangeApiKeys={setApiKeys}
          onChangeQuickInfiniteRetry={setQuickInfiniteRetry}
          onSave={handleSaveSettings}
        />

        <UpdateModal
          isOpen={showUpdateModal}
          updateInfo={updateInfo}
          onClose={handleCloseUpdate}
          onInstall={handleInstallUpdate}
          isInstalling={isInstalling}
          installComplete={installComplete}
        />

        {/* Global command palette (Ctrl+K) */}
        <CommandPalette
          isOpen={commandPaletteOpen}
          onClose={() => setCommandPaletteOpen(false)}
          ctx={buildCommandContext()}
        />
      </main>
    </div>
  );

  // Build the command context inside the component so handlers are in scope
  // and stable across renders.
  function buildCommandContext(): CommandContext {
    return {
      sessions,
      activeSession,
      onSelectSession: (name: string) => {
        setActiveSession(name);
        setSessionStats({ totalTokens: 0, totalCost: 0, qirAttempts: 0 });
        loadActiveSessionMessages(name);
        toast.success("Session switched", name);
      },
      onNewSession: handleNewSession,
      onClearChat: handleClearChat,
      onOpenSettings: () => setShowSettings(true),
      onExportLogs: handleExportLogs,
      onQuit: handleQuit,
      onCheckUpdate: handleCheckUpdate,
      onToggleQir: handleToggleQir,
      onToggleApprovalMode: handleToggleApprovalMode,
      quickInfiniteRetry,
      approvalMode,
    };
  }
}

function UsageBadge({ stats }: { stats: { totalTokens: number; totalCost: number; qirAttempts: number } }) {
  const formatTokens = (n: number) => n >= 1000 ? `${(n / 1000).toFixed(1)}k` : `${n}`;
  const formatCost = (n: number) => `$${n.toFixed(4)}`;
  const hasQir = stats.qirAttempts > 0;
  return (
    <div
      className={`flex items-center gap-2 px-2.5 py-1 rounded-md border text-[10px] font-mono tracking-wide ${
        hasQir
          ? "bg-amber-500/[0.06] border-amber-500/20 text-amber-300"
          : "bg-[var(--secondary)] border-[var(--border)] text-[var(--muted-foreground)]"
      }`}
      title="Cumulative session cost: tokens, USD, and QIR retry count. Each retry is a billable request."
    >
      <Coins className={`w-3 h-3 ${hasQir ? "text-amber-400" : "text-[var(--muted-foreground)]"}`} />
      <span>{formatTokens(stats.totalTokens)} tok</span>
      <span className="opacity-50">·</span>
      <span>{formatCost(stats.totalCost)}</span>
      {hasQir && (
        <>
          <span className="opacity-50">·</span>
          <InfinityIcon className="w-3 h-3 text-amber-400" />
          <span className="text-amber-300 font-extrabold">{stats.qirAttempts} retr{stats.qirAttempts === 1 ? "y" : "ies"}</span>
        </>
      )}
    </div>
  );
}
