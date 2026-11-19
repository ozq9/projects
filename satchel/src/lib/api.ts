import { invoke } from "@tauri-apps/api/core";
import type { AppStatus, ConnectionTest, KeyStatus, ModelOption, Settings, UsageSummary } from "./types";

/** An error returned by a Rust command: `kind` is e.g. "ai.auth", "invalid", "db". */
export class CommandError extends Error {
  readonly kind: string;
  constructor(kind: string, message: string) {
    super(message);
    this.kind = kind;
  }
}

function toCommandError(e: unknown): CommandError {
  if (e && typeof e === "object" && "kind" in e && "message" in e) {
    return new CommandError(String(e.kind), String(e.message));
  }
  if (typeof e === "string") return new CommandError("internal", e);
  return new CommandError("internal", e instanceof Error ? e.message : "Something went wrong.");
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    throw toCommandError(e);
  }
}

export const api = {
  startupError: () => call<string | null>("get_startup_error"),
  status: () => call<AppStatus>("get_app_status"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  setApiKey: (key: string) => call<KeyStatus>("set_api_key", { key }),
  deleteApiKey: () => call<KeyStatus>("delete_api_key"),
  testConnection: () => call<ConnectionTest>("test_connection"),
  usage: () => call<UsageSummary>("get_usage_summary"),
  models: () => call<ModelOption[]>("list_models"),
  openDataDir: () => call<void>("open_data_dir"),
  openApiKeysPage: () => call<void>("open_api_keys_page"),
  pickFolder: () => call<string | null>("pick_folder"),
  moveDataDir: (target: string) => call<void>("move_data_dir", { target }),
};

export const queryKeys = {
  status: ["status"] as const,
  usage: ["usage"] as const,
  models: ["models"] as const,
};
