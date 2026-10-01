import type { ReactNode } from "react";

export function Stat({ label, value, sub }: { label: string; value: ReactNode; sub?: string }) {
  return (
    <div className="card">
      <div className="text-xs uppercase tracking-wide text-slate-400">{label}</div>
      <div className="mt-1 text-2xl font-semibold">{value}</div>
      {sub && <div className="text-xs text-slate-500">{sub}</div>}
    </div>
  );
}

export function Meter({ percent }: { percent: number }) {
  const p = Math.min(100, Math.max(0, percent));
  return (
    <div className="mt-2 h-2 rounded bg-slate-800" role="progressbar" aria-valuenow={Math.round(p)} aria-valuemin={0} aria-valuemax={100}>
      <div className={`h-2 rounded ${p > 85 ? "bg-red-500" : p > 65 ? "bg-amber-400" : "bg-pal-500"}`} style={{ width: `${p}%` }} />
    </div>
  );
}

export const gb = (b: number) => `${(b / 1e9).toFixed(1)} Go`;
