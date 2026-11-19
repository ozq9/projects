// Mirrors the Rust types serialised by the Tauri commands (src-tauri/src/commands).

export interface Settings {
  model: string;
}

export interface KeyStatus {
  present: boolean;
  hint: string | null;
}

export interface KeyStoreInfo {
  name: string;
  persistent: boolean;
  error: string | null;
}

export interface AppStatus {
  version: string;
  dataDir: string;
  defaultDataDir: string;
  isCustomDataDir: boolean;
  dbPath: string;
  schemaVersion: number;
  keyStore: KeyStoreInfo;
  apiKey: KeyStatus;
  settings: Settings;
}

export interface ConnectionTest {
  modelRequested: string;
  modelServed: string;
  greeting: string;
  latencyMs: number;
  inputTokens: number;
  outputTokens: number;
  costUsd: number | null;
}

export interface UsageTotals {
  calls: number;
  inputTokens: number;
  outputTokens: number;
  cacheCreationTokens: number;
  cacheReadTokens: number;
  costUsd: number;
  unpricedCalls: number;
}

export interface UsageGroup extends UsageTotals {
  key: string;
}

export interface UsageSummary {
  allTime: UsageTotals;
  thisMonth: UsageTotals;
  byPurpose: UsageGroup[];
  byModel: UsageGroup[];
}

export interface ModelOption {
  id: string;
  name: string;
  inputPerMtok: number;
  outputPerMtok: number;
  isDefault: boolean;
}
