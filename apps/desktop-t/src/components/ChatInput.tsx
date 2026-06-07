import { useState, useRef, useEffect, useMemo } from "react";
import {
  Send,
  Square,
  ChevronDown,
  Cpu,
  Shield,
  Loader2,
  Infinity as InfinityIcon,
  RefreshCw,
  Cloud,
} from "lucide-react";
import AgentStatus from "./AgentStatus";
import { DEFAULT_MODELS, type FetchResult } from "../lib/models";

interface ModelOption {
  id: string;
  label: string;
}

interface ChatInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: (e?: React.FormEvent) => void;
  onStop: () => void;
  isGenerating: boolean;
  qirStatus?: string | null;
  agentStatus?: string | null;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  activeProvider: string;
  activeModel: string;
  onChangeModel: (model: string) => void;
  providers: ModelOption[];
  providerModels?: Record<string, FetchResult>;
  fetchingProviderFor?: string | null;
  onFetchProviderModels?: (providerId: string, apiKey: string) => Promise<void> | void;
  apiKeys?: Record<string, string>;
}

export default function ChatInput({
  value,
  onChange,
  onSubmit,
  onStop,
  isGenerating,
  qirStatus,
  agentStatus,
  textareaRef,
  activeProvider,
  activeModel,
  onChangeModel,
  providers,
  providerModels,
  fetchingProviderFor,
  onFetchProviderModels,
  apiKeys,
}: ChatInputProps) {
  const [modelMenuOpen, setModelMenuOpen] = useState(false);
  const [modelDraft, setModelDraft] = useState(activeModel);
  const modelMenuRef = useRef<HTMLDivElement>(null);
  const modelInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setModelDraft(activeModel);
  }, [activeModel]);

  useEffect(() => {
    if (!modelMenuOpen) return;
    const handler = (e: MouseEvent) => {
      if (modelMenuRef.current && !modelMenuRef.current.contains(e.target as Node)) {
        setModelMenuOpen(false);
      }
    };
    window.addEventListener("mousedown", handler);
    return () => window.removeEventListener("mousedown", handler);
  }, [modelMenuOpen]);

  // When the menu opens, kick off a fetch for the active provider if we
  // don't have a live result yet. Mirrors the CLI: open the model menu,
  // resolve the provider, call list_models().
  useEffect(() => {
    if (!modelMenuOpen) return;
    if (!onFetchProviderModels) return;
    if (!activeProvider) return;
    const cached = providerModels?.[activeProvider];
    if (cached && cached.source === "live") return;
    const key = apiKeys?.[activeProvider] ?? "";
    onFetchProviderModels(activeProvider, key);
  }, [modelMenuOpen, activeProvider, providerModels, apiKeys, onFetchProviderModels]);

  const isFetching = fetchingProviderFor === activeProvider;
  const cachedResult = providerModels?.[activeProvider];
  const modelList = useMemo(() => {
    if (cachedResult && cachedResult.models.length > 0) return cachedResult.models;
    return DEFAULT_MODELS[activeProvider] ?? [];
  }, [cachedResult, activeProvider]);
  const liveCount =
    cachedResult && cachedResult.source === "live" ? cachedResult.models.length : 0;

  const currentProvider = providers.find(p => p.id === activeProvider);
  const providerLabel = currentProvider?.label ?? activeProvider;

  return (
    <div className="px-6 py-3 bg-[var(--background)] border-t border-[var(--border)]">
      <div className="max-w-4xl mx-auto flex flex-col gap-1.5">
        {agentStatus && <AgentStatus status={agentStatus} />}
        {qirStatus && <QirStatusBar status={qirStatus} />}

        <form
          onSubmit={(e) => {
            e.preventDefault();
            onSubmit();
          }}
        >
          <div className="bg-[var(--surface)] border border-[var(--border)] hover:border-[#3e3e3e] focus-within:border-[#505050] rounded-xl p-3 flex flex-col gap-3 transition-colors duration-200">
            <textarea
              ref={textareaRef}
              value={value}
              onChange={e => onChange(e.target.value)}
              onKeyDown={e => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  onSubmit();
                }
              }}
              rows={1}
              placeholder="Ask anything, / for commands, @ for context..."
              className="w-full min-h-[40px] max-h-[200px] bg-transparent border-none outline-none resize-none text-[13px] text-[var(--foreground)] placeholder-[#707070] px-1 py-1"
            />

            <div className="flex items-center justify-between mt-1 px-1">
              <div className="flex items-center gap-2">
                <div ref={modelMenuRef} className="relative">
                  <button
                    type="button"
                    onClick={() => {
                      setModelMenuOpen(p => !p);
                      if (!modelMenuOpen) {
                        setTimeout(() => modelInputRef.current?.focus(), 0);
                      }
                    }}
                    className="flex items-center gap-1.5 px-2 py-1 rounded text-[10px] font-medium text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] transition-colors"
                    title="Switch model"
                    aria-label="Switch model"
                    aria-expanded={modelMenuOpen}
                  >
                    <Cpu className="w-3.5 h-3.5" />
                    <span className="font-mono truncate max-w-[180px]">
                      {providerLabel} · {activeModel}
                    </span>
                    <ChevronDown className={`w-3 h-3 transition-transform ${modelMenuOpen ? "rotate-180" : ""}`} />
                  </button>

                  {modelMenuOpen && (
                    <div className="absolute bottom-full left-0 mb-2 w-80 bg-[#0d0d10] border border-[var(--border)] rounded-lg shadow-xl shadow-black/40 z-30 animate-fade-in">
                      <div className="px-3 py-2 border-b border-[var(--border)] flex items-center justify-between">
                        <div>
                          <div className="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)] font-bold mb-0.5">
                            Provider
                          </div>
                          <div className="text-[11px] font-mono text-[var(--foreground)]">
                            {providerLabel}
                          </div>
                        </div>
                        <div className="flex items-center gap-1">
                          {cachedResult && (
                            <span
                              className={`flex items-center gap-1 px-1.5 py-0.5 rounded text-[9px] font-mono border ${
                                cachedResult.source === "live"
                                  ? "border-emerald-500/30 text-emerald-300 bg-emerald-500/[0.06]"
                                  : "border-amber-500/30 text-amber-300 bg-amber-500/[0.06]"
                              }`}
                              title={
                                cachedResult.source === "live"
                                  ? "Live models fetched from provider API"
                                  : cachedResult.error
                                  ? `Fallback list (${cachedResult.error})`
                                  : "Fallback list"
                              }
                            >
                              {cachedResult.source === "live" ? (
                                <>
                                  <Cloud className="w-2.5 h-2.5" /> {liveCount} live
                                </>
                              ) : (
                                <>fallback ({modelList.length})</>
                              )}
                            </span>
                          )}
                          <button
                            type="button"
                            onClick={() => {
                              const key = apiKeys?.[activeProvider] ?? "";
                              onFetchProviderModels?.(activeProvider, key);
                            }}
                            disabled={isFetching}
                            className="p-1 rounded text-[var(--muted-foreground)] hover:text-[var(--foreground)] hover:bg-[var(--secondary)] disabled:opacity-50"
                            title="Refresh models from provider API"
                            aria-label="Refresh models"
                          >
                            <RefreshCw className={`w-3 h-3 ${isFetching ? "animate-spin" : ""}`} />
                          </button>
                        </div>
                      </div>
                      <div className="px-3 py-2">
                        <label className="text-[9px] uppercase tracking-wider text-[var(--muted-foreground)] font-bold mb-1 block">
                          Model name
                        </label>
                        <input
                          ref={modelInputRef}
                          value={modelDraft}
                          onChange={e => setModelDraft(e.target.value)}
                          onKeyDown={e => {
                            if (e.key === "Enter") {
                              e.preventDefault();
                              onChangeModel(modelDraft.trim() || activeModel);
                              setModelMenuOpen(false);
                            } else if (e.key === "Escape") {
                              setModelMenuOpen(false);
                            }
                          }}
                          placeholder="e.g. claude-sonnet-4-5"
                          className="w-full px-2 py-1.5 bg-[var(--background)] border border-[var(--border)] rounded text-[11px] font-mono text-[var(--foreground)] outline-none focus:border-violet-500/50"
                        />
                      </div>
                      <div className="border-t border-[var(--border)] max-h-56 overflow-y-auto custom-scrollbar">
                        {isFetching && modelList.length === 0 ? (
                          <div className="flex items-center gap-2 px-3 py-3 text-[11px] text-[var(--muted-foreground)]">
                            <Loader2 className="w-3 h-3 animate-spin" />
                            Fetching models from {providerLabel}...
                          </div>
                        ) : modelList.length === 0 ? (
                          <div className="px-3 py-3 text-[11px] text-[var(--muted-foreground)]">
                            No models available. Add an API key in Settings, then click Refresh.
                          </div>
                        ) : (
                          modelList.map(m => (
                            <button
                              key={m}
                              type="button"
                              onClick={() => {
                                onChangeModel(m);
                                setModelDraft(m);
                                setModelMenuOpen(false);
                              }}
                              className={`w-full text-left px-3 py-1.5 text-[11px] font-mono hover:bg-[var(--secondary)] transition-colors ${
                                m === activeModel
                                  ? "text-violet-300 bg-violet-500/[0.04]"
                                  : "text-[var(--foreground)]"
                              }`}
                            >
                              {m}
                            </button>
                          ))
                        )}
                      </div>
                      <div className="px-3 py-2 border-t border-[var(--border)] flex items-center justify-end gap-2">
                        <button
                          type="button"
                          onClick={() => setModelMenuOpen(false)}
                          className="px-2 py-1 text-[10px] text-[var(--muted-foreground)] hover:text-[var(--foreground)]"
                        >
                          Cancel
                        </button>
                        <button
                          type="button"
                          onClick={() => {
                            onChangeModel(modelDraft.trim() || activeModel);
                            setModelMenuOpen(false);
                          }}
                          className="px-2.5 py-1 text-[10px] bg-violet-500/20 hover:bg-violet-500/30 border border-violet-500/30 text-violet-200 rounded font-mono"
                        >
                          Apply
                        </button>
                      </div>
                    </div>
                  )}
                </div>

                <div className="flex items-center gap-1.5 px-2 py-1 rounded text-[10px] font-medium text-[var(--muted-foreground)]">
                  <Shield className="w-3.5 h-3.5" /> Sandbox Secured
                </div>
              </div>

              {isGenerating ? (
                <button
                  type="button"
                  onClick={onStop}
                  className="w-8 h-8 rounded bg-[var(--secondary)] hover:bg-rose-500/20 hover:text-rose-400 flex items-center justify-center text-[var(--foreground)] transition-colors cursor-pointer"
                  title="Stop generation"
                  aria-label="Stop generation"
                >
                  <Square className="w-3.5 h-3.5 fill-current" />
                </button>
              ) : (
                <button
                  type="submit"
                  disabled={!value.trim()}
                  className="w-8 h-8 rounded flex items-center justify-center bg-[var(--foreground)] hover:bg-[#c7c7c7] text-[#131010] disabled:opacity-50 disabled:bg-[var(--secondary)] disabled:text-[var(--muted-foreground)] transition-colors cursor-pointer"
                >
                  <Send className="w-3.5 h-3.5" />
                </button>
              )}
            </div>
          </div>
        </form>
      </div>
    </div>
  );
}

function QirStatusBar({ status }: { status: string }) {
  const isSuccess = /\b(recovered|succeeded)\b/i.test(status);
  const attemptMatch = status.match(/attempt\s+(\d+)/i);
  const recoveredMatch = status.match(/after\s+(\d+)\s+attempts?/i);
  const attempt = attemptMatch?.[1] ?? recoveredMatch?.[1] ?? null;

  const tone = isSuccess
    ? { wrap: "bg-emerald-500/[0.06] border-emerald-500/20 text-emerald-300", icon: "text-emerald-400", label: "Recovered" }
    : { wrap: "bg-amber-500/[0.06] border-amber-500/20 text-amber-300", icon: "text-amber-400", label: "Retrying" };

  return (
    <div
      className={`flex items-center gap-2 px-3 py-1.5 rounded-md border text-[10px] font-mono tracking-wide ${tone.wrap}`}
      role="status"
      aria-live="polite"
    >
      {isSuccess ? (
        <InfinityIcon className={`w-3 h-3 ${tone.icon}`} />
      ) : (
        <Loader2 className={`w-3 h-3 ${tone.icon} animate-spin`} />
      )}
      <span className="font-extrabold uppercase tracking-wider">
        QIR · {tone.label}
        {attempt && <> · attempt {attempt}</>}
      </span>
      <span className="truncate text-[var(--muted-foreground)] font-sans">{status}</span>
    </div>
  );
}
