import React from "react";
import {
  CheckCircle2,
  XCircle,
  Cpu,
  Infinity as InfinityIcon,
  AlertTriangle,
  CheckCheck,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  HighlightedParagraph,
  HighlightedSpan,
  HighlightedListItem,
  detectCallout,
} from "../lib/textHighlight";
import ToolCallCard from "./ToolCallCard";
import type { ToolEvent } from "../App";

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

interface ChatAreaProps {
  messages: Message[];
  expandedThoughts: Record<string, boolean>;
  onToggleThought: (id: string) => void;
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
}

function plainTextOf(children: React.ReactNode): string {
  if (typeof children === "string") return children;
  if (typeof children === "number") return String(children);
  if (Array.isArray(children)) return children.map(plainTextOf).join("");
  if (React.isValidElement(children)) {
    return plainTextOf((children.props as { children?: React.ReactNode }).children);
  }
  return "";
}

export default function ChatArea({
  messages,
  expandedThoughts,
  onToggleThought,
  messagesEndRef,
}: ChatAreaProps) {
  return (
    <div className="flex-1 overflow-y-auto px-6 py-8 flex flex-col gap-6 bg-[var(--background)]">
      {messages.map(msg => {
        const isUser = msg.sender === "user";
        const isSuccess = msg.sender === "system-success";
        const isError = msg.sender === "system-error";

        if (isSuccess) {
          return (
            <div key={msg.id} className="self-start max-w-[85%] animate-fade-slide">
              <div className="flex items-center gap-3 px-4 py-3 bg-[var(--surface)] border border-[var(--border)] text-[#ededed] rounded-md">
                <CheckCircle2 className="w-4 h-4 text-[#858585] shrink-0" />
                <span className="text-xs font-mono text-[var(--foreground)]">{msg.text}</span>
              </div>
            </div>
          );
        }

        if (isError) {
          return (
            <div key={msg.id} className="self-start max-w-[85%] animate-fade-slide">
              <div className="flex items-center gap-3 px-4 py-3 bg-[var(--surface)] border border-rose-500/30 text-rose-400 rounded-md">
                <XCircle className="w-4 h-4 text-rose-400 shrink-0" />
                <span className="text-xs font-mono">{msg.text}</span>
              </div>
            </div>
          );
        }

        return (
          <div
            key={msg.id}
            className={`flex flex-col gap-1.5 w-full max-w-[100%] animate-fade-slide ${
              isUser ? "items-end" : "items-start"
            }`}
          >
            {!isUser && msg.isStreaming && msg.qirStatus && (
              <QirStatusBadge status={msg.qirStatus} />
            )}

            <div className={`w-full max-w-[85%] flex flex-col gap-2 ${isUser ? "items-end" : "items-start"}`}>
              {!isUser && msg.toolEvents && msg.toolEvents.length > 0 && (
                <div className="flex flex-col gap-2 w-full">
                  {msg.toolEvents.map(ev => (
                    <ToolCallCard key={ev.id} event={ev} />
                  ))}
                </div>
              )}
              <div className={`px-5 py-4 text-sm leading-relaxed w-full ${
                isUser
                  ? "bg-[var(--surface)] border border-[var(--border)] text-[var(--foreground)] rounded-lg whitespace-pre-wrap"
                  : "bg-transparent text-[var(--foreground)]"
              }`}>
                {msg.thought && (
                  <div className="mb-4 border border-[var(--border)] rounded-md overflow-hidden bg-[var(--surface)]">
                    <div
                      onClick={() => onToggleThought(msg.id)}
                      className="flex items-center justify-between px-3 py-2 cursor-pointer select-none hover:bg-[var(--secondary)] text-[var(--muted-foreground)] text-xs gap-2"
                    >
                      <span className="flex items-center gap-2 font-medium">
                        <Cpu className="w-3.5 h-3.5 animate-spin" style={{ animationDuration: '3s' }} />
                        Reasoning
                      </span>
                      <span className="text-[10px] font-mono text-[var(--muted-foreground)]">
                        {expandedThoughts[msg.id] ? "collapse" : "expand"}
                      </span>
                    </div>

                    {expandedThoughts[msg.id] && (
                      <div className="px-3 py-2 border-t border-[var(--border)] text-xs text-[var(--muted-foreground)] font-mono leading-relaxed bg-[var(--background)] whitespace-pre-wrap">
                        {msg.thought}
                      </div>
                    )}
                  </div>
                )}

                {isUser ? (
                  msg.text
                ) : (
                  <ReactMarkdown
                    remarkPlugins={[remarkGfm]}
                    components={{
                      pre: ({ children }) => <>{children}</>,
                      code: ({ className, children, ...props }) => {
                        const match = /language-(\w+)/.exec(className || '');
                        const text = String(children ?? "");
                        const isInline = !match && !text.includes('\n');

                        if (isInline) {
                          return (
                            <code
                              className="rounded bg-rose-500/[0.10] border border-rose-500/20 px-1 py-0.5 font-mono text-[12.5px] text-rose-200"
                              {...props}
                            >
                              {children}
                            </code>
                          );
                        }

                        return (
                          <div className="relative my-4 overflow-hidden rounded-md border border-[var(--border)] bg-[#131010]">
                            <div className="flex items-center justify-between border-b border-[var(--border)] bg-[var(--surface)] px-3 py-1.5 text-xs text-[var(--muted-foreground)]">
                              <span className="font-mono">{match ? match[1] : "code"}</span>
                            </div>
                            <pre className="overflow-x-auto p-4 font-mono text-[13px] leading-relaxed text-[var(--foreground)]">
                              <code className={className} {...props}>
                                {children}
                              </code>
                            </pre>
                          </div>
                        );
                      },
                      p: ({ children }) => {
                        const txt = plainTextOf(children);
                        const callout = detectCallout(txt);
                        return (
                          <HighlightedParagraph callout={callout}>
                            {children}
                          </HighlightedParagraph>
                        );
                      },
                      ul: ({ children }) => (
                        <ul className="mb-4 pl-5 list-disc space-y-1 text-[var(--foreground)]">
                          {React.Children.map(children, (child, i) => (
                            <HighlightedListItem key={i}>{child}</HighlightedListItem>
                          ))}
                        </ul>
                      ),
                      ol: ({ children }) => (
                        <ol className="mb-4 pl-5 list-decimal space-y-1 text-[var(--foreground)]">
                          {React.Children.map(children, (child, i) => (
                            <HighlightedListItem key={i}>{child}</HighlightedListItem>
                          ))}
                        </ol>
                      ),
                      li: ({ children }) => <HighlightedListItem>{children}</HighlightedListItem>,
                      h1: ({ children }) => (
                        <h1 className="mt-6 mb-4 text-xl font-semibold text-[var(--foreground)]">
                          <HighlightedSpan>{children}</HighlightedSpan>
                        </h1>
                      ),
                      h2: ({ children }) => (
                        <h2 className="mt-5 mb-3 text-lg font-semibold text-[var(--foreground)]">
                          <HighlightedSpan>{children}</HighlightedSpan>
                        </h2>
                      ),
                      h3: ({ children }) => (
                        <h3 className="mt-4 mb-2 text-base font-medium text-[var(--foreground)]">
                          <HighlightedSpan>{children}</HighlightedSpan>
                        </h3>
                      ),
                      h4: ({ children }) => (
                        <h4 className="mt-3 mb-1 text-sm font-medium text-[var(--foreground)]">
                          <HighlightedSpan>{children}</HighlightedSpan>
                        </h4>
                      ),
                      blockquote: ({ children }) => (
                        <blockquote className="my-4 border-l-2 border-[var(--border)] bg-[var(--surface)] py-2 pl-4 pr-3 text-[var(--muted-foreground)] italic">
                          {children}
                        </blockquote>
                      ),
                      a: ({ children, ...props }) => (
                        <a
                          className="text-sky-300 hover:text-sky-200 underline-offset-2 hover:underline transition-colors"
                          target="_blank"
                          rel="noopener noreferrer"
                          {...props}
                        >
                          {children}
                        </a>
                      ),
                      strong: ({ children }) => (
                        <strong className="font-semibold text-[var(--foreground)]">
                          <HighlightedSpan>{children}</HighlightedSpan>
                        </strong>
                      ),
                      em: ({ children }) => <em className="italic">{children}</em>,
                      table: ({ children }) => (
                        <div className="my-4 w-full overflow-x-auto rounded-md border border-[var(--border)] bg-[var(--surface)]">
                          <table className="w-full border-collapse text-left text-sm text-[var(--foreground)]">
                            {children}
                          </table>
                        </div>
                      ),
                      thead: ({ children }) => (
                        <thead className="border-b border-[var(--border)] bg-[var(--background)] text-xs font-medium text-[var(--muted-foreground)]">
                          {children}
                        </thead>
                      ),
                      tr: ({ children }) => <tr className="border-b border-[var(--border)] last:border-0">{children}</tr>,
                      th: ({ children }) => (
                        <th className="px-4 py-2 font-medium border-r border-[var(--border)] last:border-r-0">
                          {children}
                        </th>
                      ),
                      td: ({ children }) => (
                        <td className="px-4 py-2 border-r border-[var(--border)] last:border-r-0 text-[var(--foreground)]">
                          {children}
                        </td>
                      ),
                    }}
                  >
                    {msg.text}
                  </ReactMarkdown>
                )}
              </div>
            </div>
          </div>
        );
      })}
      <div ref={messagesEndRef} />
    </div>
  );
}

function QirStatusBadge({ status }: { status: string }) {
  const isSuccess = /\b(recovered|succeeded)\b/i.test(status);
  const Icon = isSuccess ? CheckCheck : AlertTriangle;
  const iconClass = isSuccess ? "text-emerald-400" : "text-amber-400";
  const spin = !isSuccess;
  return (
    <div
      className={`flex items-center gap-2 px-3 py-1.5 rounded-md border text-[10px] font-mono tracking-wide ${
        isSuccess
          ? "bg-emerald-500/[0.06] border-emerald-500/20 text-emerald-300"
          : "bg-amber-500/[0.06] border-amber-500/20 text-amber-300"
      }`}
      role="status"
      aria-live="polite"
    >
      {isSuccess ? (
        <InfinityIcon className={`w-3 h-3 ${iconClass}`} />
      ) : (
        <Icon className={`w-3 h-3 ${iconClass} ${spin ? "animate-pulse" : ""}`} />
      )}
      <span className="truncate">{status}</span>
    </div>
  );
}
