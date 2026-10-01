import { useState } from "react";
import type { Sample } from "../lib/types";

interface Props {
  title: string;
  unit: string;
  data: Sample[];
  pick: (s: Sample) => number;
  /** Borne haute fixe de l'axe (ex. 100 pour un pourcentage) ; sinon auto. */
  max?: number;
}

const W = 600, H = 160, PAD = { l: 36, r: 8, t: 8, b: 20 };
const hhmm = (t: number) => new Date(t * 1000).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });

export default function LineChart({ title, unit, data, pick, max }: Props) {
  const [hover, setHover] = useState<number | null>(null);
  const [table, setTable] = useState(false);
  const vals = data.map(pick);
  const top = max ?? Math.max(1, ...vals) * 1.15;
  const t0 = data[0]?.t ?? 0, t1 = data[data.length - 1]?.t ?? 1;
  const x = (t: number) => PAD.l + ((t - t0) / Math.max(1, t1 - t0)) * (W - PAD.l - PAD.r);
  const y = (v: number) => PAD.t + (1 - v / top) * (H - PAD.t - PAD.b);
  const path = data.map((s, i) => `${i ? "L" : "M"}${x(s.t).toFixed(1)},${y(pick(s)).toFixed(1)}`).join(" ");
  const h = hover !== null ? data[hover] : null;

  const onMove = (e: React.MouseEvent<SVGSVGElement>) => {
    if (!data.length) return;
    const r = e.currentTarget.getBoundingClientRect();
    const t = t0 + (((e.clientX - r.left) / r.width) * W - PAD.l) / (W - PAD.l - PAD.r) * (t1 - t0);
    let best = 0;
    data.forEach((s, i) => { if (Math.abs(s.t - t) < Math.abs(data[best].t - t)) best = i; });
    setHover(best);
  };

  return (
    <div className="card">
      <div className="mb-1 flex items-baseline gap-2">
        <h3 className="font-semibold">{title}</h3>
        <span className="text-sm text-slate-400">{h ? `${hhmm(h.t)} · ${pick(h).toFixed(unit === "%" ? 0 : 0)} ${unit}` : vals.length ? `dernier : ${vals[vals.length - 1].toFixed(0)} ${unit}` : ""}</span>
        <button className="btn ml-auto !py-0.5 text-xs" onClick={() => setTable(!table)}>{table ? "Graphique" : "Tableau"}</button>
      </div>
      {data.length < 2 ? <p className="py-8 text-center text-sm text-slate-500">Pas encore assez de données (un point toutes les 30 s pendant que le serveur tourne).</p>
        : table ? (
          <div className="max-h-40 overflow-y-auto text-sm"><table className="w-full"><tbody>
            {[...data].reverse().slice(0, 100).map((s) => <tr key={s.t} className="border-t border-slate-800"><td className="py-0.5 text-slate-400">{hhmm(s.t)}</td><td>{pick(s).toFixed(0)} {unit}</td></tr>)}
          </tbody></table></div>
        ) : (
          <svg viewBox={`0 0 ${W} ${H}`} className="w-full" role="img" aria-label={`${title} sur la période`} onMouseMove={onMove} onMouseLeave={() => setHover(null)}>
            {[0, 0.5, 1].map((f) => (
              <g key={f}>
                <line x1={PAD.l} x2={W - PAD.r} y1={y(top * f)} y2={y(top * f)} className="stroke-slate-800" />
                <text x={PAD.l - 4} y={y(top * f) + 3} textAnchor="end" className="fill-slate-500 text-[10px]">{Math.round(top * f)}</text>
              </g>
            ))}
            <text x={PAD.l} y={H - 4} className="fill-slate-500 text-[10px]">{hhmm(t0)}</text>
            <text x={W - PAD.r} y={H - 4} textAnchor="end" className="fill-slate-500 text-[10px]">{hhmm(t1)}</text>
            <path d={path} fill="none" strokeWidth={2} strokeLinejoin="round" className="stroke-pal-500" />
            {h && (
              <g>
                <line x1={x(h.t)} x2={x(h.t)} y1={PAD.t} y2={H - PAD.b} className="stroke-slate-600" />
                <circle cx={x(h.t)} cy={y(pick(h))} r={4} strokeWidth={2} className="fill-pal-500 stroke-slate-900" />
              </g>
            )}
          </svg>
        )}
    </div>
  );
}
