import React, { useState, useRef, useEffect } from "react";
import { 
  ChevronRight, 
  Trash2, 
  Download 
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Import Modular Components
import Sidebar from "./components/Sidebar";
import SettingsModal from "./components/SettingsModal";
import ConfirmationModal from "./components/ConfirmationModal";
import ChatArea from "./components/ChatArea";
import ChatInput from "./components/ChatInput";
import UpdateModal from "./components/UpdateModal";

interface Message {
  id: string;
  sender: "user" | "assistant" | "system-success" | "system-error";
  text: string;
  model?: string;
  thought?: string;
  isStreaming?: boolean;
}

// SDK-compatible message shape
interface SDKMessage {
  role: "system" | "user" | "assistant" | "tool";
  content: string | null;
  reasoning_content?: string | null;
}

export default function App() {
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
  const [isSidebarOpen, setIsSidebarOpen] = useState<boolean>(true);
  const [isGenerating, setIsGenerating] = useState<boolean>(false);

  const [showUpdateModal, setShowUpdateModal] = useState<boolean>(false);
  const [updateInfo, setUpdateInfo] = useState<any>(null);
  const [isInstalling, setIsInstalling] = useState<boolean>(false);
  const [installComplete, setInstallComplete] = useState<boolean>(false);

  const [expandedThoughts, setExpandedThoughts] = useState<Record<string, boolean>>({
    welcome: true
  });

  const [modalOpen, setModalOpen] = useState<boolean>(false);
  const [modalDetails, setModalDetails] = useState<{
    command: string;
    cwd: string;
  }>({ command: "", cwd: "" });

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
          if (cfg.api_keys) {
            setApiKeys(prev => ({
              ...prev,
              ...cfg.api_keys
            }));
          }
          
          // Once config loads, trigger active engine initialization
          invoke("init_engine", { providerName: cfg.provider || "anthropic", modelName: cfg.model || "claude-sonnet-4-5" })
            .catch(err => console.error("Initial init_engine failed:", err));
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
            }
          })
          .catch(err => console.error("Update check failed:", err));
      }, 5000);

      return () => clearTimeout(timer);
    }
  }, [isTauri]);

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
            setModalDetails({
              command: chunk.target || "Bash Sandbox Execution",
              cwd: chunk.message || "d:\\DEV\\Apps\\RouteCode"
            });
            setModalOpen(true);
            break;
          case "done":
            setMessages(prev => {
              const updated = prev.map(m => m.id === streamId ? { ...m, isStreaming: false } : m);
              saveSessionToDisk(activeSession, updated);
              return updated;
            });
            setIsGenerating(false);
            unlisten();
            break;
          case "error":
            setMessages(prev => {
              const updated = prev.map(m => m.id === streamId ? { ...m, text: `Engine Error: ${chunk.content}`, isStreaming: false } : m);
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
          cwd: "d:\\DEV\\Apps\\RouteCode"
        });
        setModalOpen(true);
      }, 1200);
    }

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

  const handleSendMessage = () => {
    const query = inputValue.trim();
    if (!query || isGenerating) return;

    setIsGenerating(true);

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
    const name = prompt("Enter new session name:")?.trim();
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

  const handleDeleteSession = (sessionName: string, e: React.MouseEvent) => {
    e.stopPropagation();
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
        if (next.length > 0) {
          setActiveSession(next[0]);
        }
      }
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
        logo_animation_color: "rainbow"
      };

      invoke("save_config", { config: configObj })
        .then(() => {
          setShowSettings(false);
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
            });
        })
        .catch(err => alert("Failed to save config: " + err));
    } else {
      setShowSettings(false);
      alert("Settings updated (Simulation Mode)");
    }
  };

  const handleAllowTool = () => {
    setModalOpen(false);
    if (isTauri) {
      invoke("respond_confirmation", { allowed: true })
        .then(() => {
          setMessages(prev => [
            ...prev,
            {
              id: `sys-succ-${Date.now()}`,
              sender: "system-success",
              text: "Tool execution approved and completed successfully in sandbox."
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
          text: "Command 'cargo build --workspace' completed with exit status code 0."
        }
      ]);
    }
  };

  const handleDenyTool = () => {
    setModalOpen(false);
    if (isTauri) {
      invoke("respond_confirmation", { allowed: false })
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
        alert("Update installation failed: " + err);
      });
  };

  const handleCloseUpdate = () => {
    setShowUpdateModal(false);
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[#030407] text-[#e2e8f0] font-sans">
      
      {/* 3D Ambient Backdrop Glow Blobs */}
      <div className="absolute top-[-100px] right-[-50px] w-[500px] h-[500px] rounded-full bg-gradient-to-br from-[#8b5cf6] to-[#db2777] opacity-[0.08] filter blur-[130px] pointer-events-none z-0" />
      <div className="absolute bottom-[-100px] left-[200px] w-[450px] h-[450px] rounded-full bg-gradient-to-tr from-[#3b82f6] to-[#6366f1] opacity-[0.06] filter blur-[120px] pointer-events-none z-0" />

      {/* Extracted Sidebar Navigation */}
      <Sidebar
        sessions={sessions}
        activeSession={activeSession}
        onSelectSession={(name) => {
          setActiveSession(name);
          loadActiveSessionMessages(name);
        }}
        onNewSession={handleNewSession}
        onDeleteSession={handleDeleteSession}
        activeProvider={activeProvider}
        activeModel={activeModel}
        isOpen={isSidebarOpen}
        isTauri={isTauri}
        onOpenSettings={() => setShowSettings(true)}
      />

      {/* Main Workspace Frame */}
      <main className="relative z-10 flex-1 flex flex-col h-full overflow-hidden">
        {/* Header Bar */}
        <header className="h-[75px] bg-black/10 border-b border-white/[0.03] backdrop-blur-xl flex items-center justify-between px-8">
          <div className="flex items-center gap-4">
            <button 
              onClick={() => setIsSidebarOpen(!isSidebarOpen)}
              className="p-2 rounded-xl hover:bg-white/5 transition-all cursor-pointer border border-white/[0.02]"
            >
              <ChevronRight className={`w-4 h-4 text-gray-400 transition-all ${isSidebarOpen ? "rotate-180" : ""}`} />
            </button>
            <div className="flex items-center gap-3">
              <div className="relative">
                <div className="w-2 h-2 bg-emerald-500 rounded-full shadow-[0_0_8px_#10b981]" />
                <div className="absolute inset-0 bg-emerald-500 rounded-full animate-ping opacity-45" />
              </div>
              <span className="font-extrabold text-sm text-gray-100 tracking-wide">
                {activeSession}
              </span>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <button 
              onClick={handleClearChat}
              className="flex items-center gap-2 px-4 py-2 bg-white/[0.02] border border-white/[0.03] hover:bg-white/[0.06] text-xs font-bold text-gray-300 rounded-xl transition-all cursor-pointer"
            >
              <Trash2 className="w-3.5 h-3.5 text-gray-400" /> Clear Chat
            </button>
            <button 
              onClick={() => alert("Exporting conversation log as markdown...")}
              className="flex items-center gap-2 px-4 py-2 bg-[#8b5cf6]/10 border border-[#8b5cf6]/20 hover:bg-[#8b5cf6]/15 text-xs font-bold text-[#c084fc] rounded-xl transition-all cursor-pointer"
            >
              <Download className="w-3.5 h-3.5" /> Export Logs
            </button>
          </div>
        </header>

        {/* Extracted Chat Area */}
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
          isGenerating={isGenerating}
          textareaRef={textareaRef}
        />

        {/* Extracted Safe Tool Confirmation Modal */}
        <ConfirmationModal
          isOpen={modalOpen}
          command={modalDetails.command}
          cwd={modalDetails.cwd}
          onAllow={handleAllowTool}
          onDeny={handleDenyTool}
        />

        {/* Extracted Translucent Settings Modal */}
        <SettingsModal
          isOpen={showSettings}
          onClose={() => setShowSettings(false)}
          activeProvider={activeProvider}
          activeModel={activeModel}
          apiKeys={apiKeys}
          onChangeProvider={setActiveProvider}
          onChangeModel={setActiveModel}
          onChangeApiKeys={setApiKeys}
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
      </main>
    </div>
  );
}
