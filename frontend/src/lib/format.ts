function toDate(value: string): Date | null {
  const normalized = value.includes("T") ? value : `${value.replace(" ", "T")}Z`;
  const d = new Date(normalized);
  return Number.isNaN(d.getTime()) ? null : d;
}

export function timeAgo(value?: string | null): string {
  if (!value) return "—";
  const d = toDate(value);
  if (!d) return "—";
  const diff = Date.now() - d.getTime();
  if (diff < 60_000) return "just now";
  const mins = Math.floor(diff / 60_000);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}

export function formatDate(value?: string | null): string {
  if (!value) return "—";
  const d = toDate(value);
  return d
    ? d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" })
    : "—";
}

export function shortAddress(address?: string | null): string {
  if (!address) return "—";
  return `${address.slice(0, 6)}…${address.slice(-4)}`;
}

export function formatWei(wei: string | null | undefined): string {
  if (!wei) return "—";
  try {
    const DECIMALS = 4n;
    const divisor = 10n ** (18n - DECIMALS);
    const scaled = BigInt(wei) / divisor;
    const whole = scaled / 10n ** DECIMALS;
    const frac = (scaled % 10n ** DECIMALS).toString().padStart(4, "0").replace(/0+$/, "");
    return `${frac ? `${whole}.${frac}` : whole} ETH`;
  } catch {
    return wei;
  }
}

export function joinTags(value: string | null | undefined): string[] {
  if (!value) return [];
  return value
    .split(",")
    .map((t) => t.trim())
    .filter(Boolean);
}