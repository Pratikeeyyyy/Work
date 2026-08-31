import type { ReactNode } from "react";

export type Tone = "slate" | "sky" | "indigo" | "amber" | "violet" | "emerald" | "rose";

const tones: Record<Tone, string> = {
  slate: "bg-slate-100 text-slate-700 ring-slate-600/10",
  sky: "bg-sky-100 text-sky-700 ring-sky-600/10",
  indigo: "bg-indigo-100 text-indigo-700 ring-indigo-600/10",
  amber: "bg-amber-100 text-amber-800 ring-amber-600/10",
  violet: "bg-violet-100 text-violet-700 ring-violet-600/10",
  emerald: "bg-emerald-100 text-emerald-700 ring-emerald-600/10",
  rose: "bg-rose-100 text-rose-700 ring-rose-600/10",
};

const STATUS_TONE: Record<string, Tone> = {
  new: "sky",
  shortlisted: "indigo",
  applied: "amber",
  responded: "violet",
  won: "emerald",
  lost: "rose",
  archived: "slate",
  active: "emerald",
  inactive: "slate",
  blacklisted: "rose",
  draft: "slate",
  deployed: "indigo",
  funded: "sky",
  in_progress: "violet",
  submitted: "amber",
  completed: "emerald",
  disputed: "rose",
  refunded: "slate",
  upwork: "sky",
  freelancer: "violet",
  fiverr: "emerald",
  manual: "slate",
};

const uppercase = new Set(["upwork", "fiverr", "manual"]);

export function Badge({
  tone = "slate",
  className = "",
  children,
}: {
  tone?: Tone;
  className?: string;
  children: ReactNode;
}) {
  return (
    <span
      className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ring-1 ring-inset ${tones[tone]} ${className}`}
    >
      {children}
    </span>
  );
}

export function statusTone(status: string): Tone {
  const key = status.toLowerCase();
  return STATUS_TONE[key] ?? "slate";
}

export function displayLabel(status: string): string {
  const key = status.toLowerCase();
  if (uppercase.has(key)) return key.toUpperCase();
  return key.charAt(0).toUpperCase() + key.slice(1);
}