const tokenFormat = new Intl.NumberFormat("en-US");
const compactFormat = new Intl.NumberFormat("en-US", { notation: "compact", maximumFractionDigits: 1 });

/** Money for totals: "$0.00", "<$0.01", "$1.24". */
export function formatUsd(amount: number): string {
  if (amount === 0) return "$0.00";
  if (amount < 0.01) return "<$0.01";
  return `$${amount.toFixed(2)}`;
}

/** Money for a single small call: "$0.0012". */
export function formatUsdPrecise(amount: number): string {
  return amount < 0.01 ? `$${amount.toFixed(4)}` : `$${amount.toFixed(3)}`;
}

export function formatTokens(n: number): string {
  return n >= 100_000 ? compactFormat.format(n) : tokenFormat.format(n);
}

export function formatSeconds(ms: number): string {
  return `${(ms / 1000).toFixed(1)} s`;
}

export function greeting(now = new Date()): string {
  const h = now.getHours();
  if (h < 5) return "Up late";
  if (h < 12) return "Good morning";
  if (h < 18) return "Good afternoon";
  return "Good evening";
}

const purposeLabels: Record<string, string> = {
  connection_test: "Connection tests",
  classify: "Sorting items",
  result_card: "Result cards",
  chat: "Library chat",
  vision_ocr: "Reading images",
};

export function purposeLabel(purpose: string): string {
  return purposeLabels[purpose] ?? purpose.replace(/_/g, " ");
}
