import { useState } from "react";
import { api } from "../lib/api";
import type { Snapshot } from "../lib/types";
import { Meter, Stat, gb } from "../components/ui";
import LogViewer from "../components/LogViewer";

export default function Dashboard({ snap, notify }: { snap: Snapshot | null; notify: (m: string) => void }) {
  const [busy, setBusy] = useState(false);
  const run = async (f: () => Promise<unknown>) => {
    setBusy(true);
    try { await f(); } catch (e) { notify(String(e)); } finally { setBusy(false); }
  };
  const s = snap;
  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <span className={`h-3 w-3 rounded-full ${s?.running ? "bg-emerald-400" : "bg-red-500"}`} />
        <h2 className="text-lg font-semibold">{s?.running ? "Serveur en ligne" : "Serveur arrêté"}</h2>
        <div className="ml-auto flex gap-2">
          <button className="btn-primary" disabled={busy || s?.running} onClick={() => run(api.start)}>Démarrer</button>
          <button className="btn" disabled={busy || !s?.running} onClick={() => run(api.restart)}>Redémarrer</button>
          <button className="btn" disabled={busy} onClick={() => confirm("Mettre à jour via SteamCMD ? Le serveur sera arrêté puis relancé.") && run(async () => notify(await api.updateServer()))}>Mettre à jour</button>
          <button className="btn-danger" disabled={busy || !s?.running} onClick={() => run(api.stop)}>Arrêter</button>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <Stat label="CPU" value={`${(s?.cpu_percent ?? 0).toFixed(0)} %`} />
        <div className="card">
          <div className="text-xs uppercase tracking-wide text-slate-400">Mémoire</div>
          <div className="mt-1 text-2xl font-semibold">{gb(s?.memory_bytes ?? 0)}</div>
          <Meter percent={s?.memory_percent ?? 0} />
        </div>
        <Stat label="Joueurs" value={`${s?.players.length ?? 0} / ${s?.metrics?.maxplayernum ?? "–"}`} />
        <Stat label="FPS serveur" value={s?.metrics?.serverfps ?? "–"} sub={s?.metrics ? `Jour ${s.metrics.days}` : undefined} />
      </div>
      <section aria-labelledby="logs-title" className="space-y-2">
        <h2 id="logs-title" className="font-semibold">Journal du serveur</h2>
        <LogViewer notify={notify} compact />
      </section>
    </div>
  );
}
