import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Finding } from "../lib/types";

const STYLE: Record<Finding["severity"], { chip: string; label: string }> = {
  critical: { chip: "bg-red-500/20 text-red-300", label: "grave" },
  warning: { chip: "bg-amber-500/20 text-amber-300", label: "à surveiller" },
  info: { chip: "bg-slate-700 text-slate-300", label: "info" },
};

/** Causes probables de problèmes d'après la fin du journal du serveur. */
export default function LogAnalysis({ notify }: { notify: (m: string) => void }) {
  const [f, setF] = useState<Finding[] | null>(null);
  const run = useCallback(() => { api.analyzeLog().then(setF).catch((e) => notify(String(e))); }, [notify]);
  useEffect(() => { run(); }, [run]);

  return (
    <div className="card space-y-2">
      <div className="flex items-center">
        <h2 className="font-semibold">Analyse du journal</h2>
        <button className="btn ml-auto" onClick={run}>Relancer l'analyse</button>
      </div>
      {f === null ? <p className="text-sm text-slate-400">Analyse…</p>
        : f.length === 0 ? <p className="text-sm text-emerald-400">✔ Aucun problème reconnu dans la fin du journal.</p>
        : (
          <ul className="space-y-2 text-sm">
            {f.map((x) => (
              <li key={x.id} className="rounded-lg border border-slate-800 p-3">
                <div className="flex flex-wrap items-center gap-2">
                  <span className={`rounded px-1.5 py-0.5 text-xs ${STYLE[x.severity].chip}`}>{STYLE[x.severity].label}</span>
                  <strong>{x.title}</strong>
                  <span className="text-xs text-slate-500">{x.count} ligne{x.count > 1 ? "s" : ""}</span>
                </div>
                <p className="mt-1 text-slate-300">{x.advice}</p>
                <code className="mt-1 block truncate text-xs text-slate-500" title={x.sample}>{x.sample}</code>
              </li>
            ))}
          </ul>
        )}
      <p className="text-xs text-slate-500">Ce sont des causes <em>probables</em>, reconnues d'après des motifs dans le texte du journal.</p>
    </div>
  );
}
