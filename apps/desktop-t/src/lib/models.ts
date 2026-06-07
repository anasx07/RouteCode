import { invoke } from "@tauri-apps/api/core";

const isTauri = typeof window !== "undefined" && (window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !== undefined;

export const DEFAULT_MODELS: Record<string, string[]> = {
  anthropic: [
    "claude-sonnet-4-5",
    "claude-sonnet-4-20250514",
    "claude-opus-4-20250514",
    "claude-3-7-sonnet-20250219",
    "claude-3-5-sonnet-20240620",
    "claude-3-opus-20240229",
    "claude-3-haiku-20240307",
  ],
  openai: [
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
    "o1",
    "o1-mini",
    "o3",
    "o3-mini",
  ],
  openrouter: ["auto", "meta-llama/llama-3.3-70b-instruct", "qwen/qwen-2.5-72b-instruct"],
  deepseek: ["deepseek-chat", "deepseek-reasoner", "deepseek-coder"],
  google: ["gemini-2.0-pro", "gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"],
  nvidia: [
    "meta/llama-3.1-70b-instruct",
    "meta/llama-3.1-8b-instruct",
    "nvidia/nemotron-4-340b-instruct",
    "nvidia/llama-3.1-nemotron-70b-instruct",
  ],
  "cloudflare-workers": [
    "@cf/meta/llama-3.1-8b-instruct",
    "@cf/mistral/mistral-7b-instruct",
    "@cf/google/gemma-2-9b-it",
  ],
};

export interface FetchResult {
  models: string[];
  source: "live" | "fallback" | "cached";
  error?: string;
}

export async function fetchProviderModels(
  providerId: string,
  apiKey: string
): Promise<FetchResult> {
  if (!providerId) {
    return { models: DEFAULT_MODELS["anthropic"] ?? [], source: "fallback", error: "No provider" };
  }
  if (!isTauri) {
    return { models: DEFAULT_MODELS[providerId] ?? [], source: "fallback" };
  }
  if (!apiKey || apiKey.trim() === "" || apiKey === "your-api-key-here") {
    return {
      models: DEFAULT_MODELS[providerId] ?? [],
      source: "fallback",
      error: "No API key configured",
    };
  }
  try {
    const models = await invoke<string[]>("fetch_provider_models", {
      providerId,
      apiKey,
    });
    if (!models || models.length === 0) {
      return { models: DEFAULT_MODELS[providerId] ?? [], source: "fallback" };
    }
    return { models, source: "live" };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { models: DEFAULT_MODELS[providerId] ?? [], source: "fallback", error: message };
  }
}
