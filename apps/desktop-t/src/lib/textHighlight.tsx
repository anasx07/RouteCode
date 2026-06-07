import React from "react";

const FILE_PATH_RE =
  /(?:[A-Za-z0-9_\-.]+\/)+[A-Za-z0-9_\-.]+\.(?:ts|tsx|js|jsx|mjs|cjs|rs|py|json|toml|yaml|yml|md|sh|bash|zsh|css|html|svg|lock|sql)(?:\b|:|"|`|\)|\]|,)/g;
const IDENT_RE = /\b(?:[a-z]+[A-Z][A-Za-z0-9]*|[A-Z][A-Z0-9_]+|[a-z_][a-z0-9_]*__[a-z0-9_]+|[a-z][a-z0-9]*(?:_[a-z0-9]+)+)\b/g;
const URL_RE = /\bhttps?:\/\/[^\s<>"'`)\]]+/g;

function isPathContext(text: string, matchIndex: number, matchLength: number): boolean {
  const before = text.slice(Math.max(0, matchIndex - 1), matchIndex);
  const after = text.slice(matchIndex + matchLength, matchIndex + matchLength + 1);
  if (before === "=" || before === "(" || before === ">") return true;
  if (after === ":") {
    const tail = text.slice(matchIndex + matchLength + 1, matchIndex + matchLength + 4);
    if (/^\d/.test(tail)) return true;
  }
  return false;
}

interface Seg {
  text: string;
  kind: "text" | "path" | "ident" | "url";
}

function splitIntoSegments(text: string): Seg[] {
  if (!text) return [];
  const ranges: { start: number; end: number; kind: Exclude<Seg["kind"], "text"> }[] = [];

  let m: RegExpExecArray | null;
  FILE_PATH_RE.lastIndex = 0;
  while ((m = FILE_PATH_RE.exec(text)) !== null) {
    let end = m.index + m[0].length;
    while (end > m.index && /[":,)\]`]/.test(text[end - 1])) end--;
    ranges.push({ start: m.index, end, kind: "path" });
  }

  URL_RE.lastIndex = 0;
  while ((m = URL_RE.exec(text)) !== null) {
    const start = m.index;
    const end = m.index + m[0].length;
    const overlaps = ranges.some(r => start < r.end && end > r.start);
    if (!overlaps) {
      ranges.push({ start, end, kind: "url" });
    }
  }

  IDENT_RE.lastIndex = 0;
  while ((m = IDENT_RE.exec(text)) !== null) {
    const start = m.index;
    const end = m.index + m[0].length;
    const insidePath = ranges.some(
      r => r.kind === "path" && start >= r.start && end <= r.end
    );
    const insideUrl = ranges.some(
      r => r.kind === "url" && start >= r.start && end <= r.end
    );
    if (insidePath || insideUrl) continue;
    if (isPathContext(text, start, end - start)) continue;
    if (text.slice(Math.max(0, start - 1), start) === ".") continue;
    if (text.slice(end, end + 1) === "(") continue;
    ranges.push({ start, end, kind: "ident" });
  }

  ranges.sort((a, b) => a.start - b.start);
  const merged: typeof ranges = [];
  for (const r of ranges) {
    const last = merged[merged.length - 1];
    if (last && r.start < last.end) continue;
    merged.push(r);
  }

  const out: Seg[] = [];
  let cursor = 0;
  for (const r of merged) {
    if (r.start > cursor) {
      out.push({ text: text.slice(cursor, r.start), kind: "text" });
    }
    out.push({ text: text.slice(r.start, r.end), kind: r.kind });
    cursor = r.end;
  }
  if (cursor < text.length) {
    out.push({ text: text.slice(cursor), kind: "text" });
  }
  return out;
}

function highlightTextNode(text: string, keyPrefix: string): React.ReactNode {
  const segs = splitIntoSegments(text);
  return segs.map((s, i) => {
    if (s.kind === "text") return <React.Fragment key={`${keyPrefix}-${i}`}>{s.text}</React.Fragment>;
    const cls =
      s.kind === "path"
        ? "text-orange-300/90 font-mono"
        : s.kind === "ident"
        ? "text-emerald-300/90 font-mono"
        : "text-sky-300/90 underline-offset-2 hover:underline";
    return (
      <span key={`${keyPrefix}-${i}`} className={cls}>
        {s.text}
      </span>
    );
  });
}

function walkChildren(
  children: React.ReactNode,
  keyPrefix: string
): React.ReactNode {
  return React.Children.map(children, (child, i) => {
    if (typeof child === "string") {
      return <React.Fragment key={`${keyPrefix}-${i}`}>{highlightTextNode(child, `${keyPrefix}-${i}`)}</React.Fragment>;
    }
    if (typeof child === "number") {
      return <React.Fragment key={`${keyPrefix}-${i}`}>{child}</React.Fragment>;
    }
    if (React.isValidElement(child)) {
      const el = child as React.ReactElement<{ children?: React.ReactNode }>;
      if (el.type === "code") {
        return el;
      }
      if (el.props && el.props.children !== undefined) {
        return React.cloneElement(el, {
          ...el.props,
          children: walkChildren(el.props.children, `${keyPrefix}-${i}`),
        } as any);
      }
      return el;
    }
    return child;
  });
}

export function HighlightedParagraph({
  children,
  className,
  callout,
}: {
  children: React.ReactNode;
  className?: string;
  callout?: "warning" | "info" | null;
}) {
  const baseClass = "mb-4 last:mb-0 leading-relaxed text-[var(--foreground)]";
  if (callout) {
    const tone =
      callout === "warning"
        ? "border-amber-500/30 bg-amber-500/[0.04] text-[var(--foreground)]"
        : "border-sky-500/30 bg-sky-500/[0.04] text-[var(--foreground)]";
    return (
      <p
        className={`${baseClass} ${className ?? ""} my-4 rounded-md border ${tone} px-4 py-3 text-[13px] font-mono leading-relaxed`}
      >
        {walkChildren(children, "p")}
      </p>
    );
  }
  return <p className={`${baseClass} ${className ?? ""}`}>{walkChildren(children, "p")}</p>;
}

export function HighlightedSpan({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return <span className={className}>{walkChildren(children, "s")}</span>;
}

export function HighlightedListItem({
  children,
}: {
  children: React.ReactNode;
}) {
  return <li className="leading-relaxed">{walkChildren(children, "li")}</li>;
}

const CALLOUT_PREFIXES = [
  /^Main-Process\b/,
  /^Security\b/i,
  /^Warning\b/i,
  /^Problem:/i,
  /^Note:/i,
  /^TODO\b/,
];

export function detectCallout(plainText: string): "warning" | "info" | null {
  const trimmed = plainText.trim();
  for (const re of CALLOUT_PREFIXES) {
    if (re.test(trimmed)) return "warning";
  }
  if (/\bsecurity\b/i.test(trimmed) && /\bexposed\b/i.test(trimmed)) return "warning";
  return null;
}
