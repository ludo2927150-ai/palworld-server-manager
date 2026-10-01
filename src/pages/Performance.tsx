import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings, Priority, Snapshot, SystemInfo } from "../lib/types";
import { Meter, gb } from "../components/ui";

const PRIORITIES: { v: Priority; label: string; hint: string }[] = [
  { v: "belownormal", label: "Basse", hint: "Le serveur cède la place aux autres programmes." },
  { v: "normal", label: "Normale", hint: "Réglage par défaut de Windows." },
  { v: "abovenormal", label: "Haute", hint: "Recommandé si le PC fait aussi autre chose." },
  { v: "high", label: "Très haute", hint: "Priorité maximale utile ; peut ralentir le reste du PC." },
];

/** Options de lancement liées aux threads (appliquées au prochain démarrage du serveur). */
const FLAGS: { flag: string; label: string; hint: string }[] = [
  { flag: "-useperfthreads", label: "Threads de performance", hint: "Utilise des threads dédiés pour les tâches lourdes." },
  { flag: "-NoAsyncLoadingThread", label: "Chargement asynchrone désactivé", hint: "Évite des conflits de chargement ; recommandé par la communauté." },
  { flag: "-UseMultithreadForDS", label: "Multithread serveur dédié", hint: "Répartit le serveur sur plusieurs cœurs." },
];
const WORKERS = "-NumberOfWorkerThreadsServer=";

const has = (args: string[], flag: string) => args.some((a) => a.toLowerCase() === flag.toLowerCase());
const toggle = (args: string[], flag: string, on: boolean) => [...args.filter((a) => a.toLowerCase() !== flag.toLowerCase()), ...(on ? [flag] : [])];
const workers = (args: string[]) => args.find((a) => a.toLowerCase().startsWith(WORKERS.toLowerCase()))?.slice(WORKERS.length) ?? "";
const setWorkers = (args: string[], n: string) => [...args.filter((a) => !a.toLowerCase().startsWith(WORKERS.toLowerCase())), ...(Number(n) >= 1 ? [`${WORKERS}${Math.floor(Number(n))}`] : [])];

export default function Performance({ snap, notify }: { snap: Snapshot | null; notify: (m: string) => void }) {
  const [s, setS] = useState<AppSettings | null>(null);
  const [sys, setSys] = useState<SystemInfo | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(() => {
    Promise.all([api.getSettings(), api.systemInfo()]).then(([st, si]) => { setS(st); setSys(si); }).catch((e) => notify(String(e)));
  }, [notify]);
  useEffect(() => { load(); }, [load]);
  if (!s || !sys) return <p className="text-slate-400">Chargement…</p>;

  const perf = s.performance;
  const setPerf = (patch: Partial<typeof perf>) => setS({ ...s, performance: { ...perf, ...patch } });
  const selected = perf.cpu_cores ?? Array.from({ length: sys.cpu_cores }, (_, i) => i);
  const toggleCore = (i: number) => {
    const next = selected.includes(i) ? selected.filter((c) => c !== i) : [...selected, i].sort((a, b) => a - b);
    if (next.length === 0) return; // au moins un cœur
    setPerf({ cpu_cores: next.length === sys.cpu_cores ? null : next });
  };
  const totalGb = sys.total_memory_bytes / 1e9;
  const usedGb = (snap?.memory_bytes ?? 0) / 1e9;

  const save = async () => {
    setBusy(true);
    try {
      await api.saveSettings(s);
      if (snap?.running) {
        const n = await api.applyPerformance().catch((e) => { notify(`Enregistré, mais application impossible : ${e}`); return null; });
        if (n !== null) notify(n > 0 ? `Enregistré et appliqué (${n} processus)` : "Enregistré — aucun processus modifiable trouvé (droits administrateur ?)");
      } else notify("Enregistré — sera appliqué au démarrage du serveur");
    } catch (e) { notify(String(e)); } finally { setBusy(false); }
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4 lg:grid-cols-3">
        <div className="card">
          <div className="text-xs uppercase tracking-wide text-slate-400">RAM du serveur</div>
          <div className="mt-1 text-2xl font-semibold">{gb(snap?.memory_bytes ?? 0)} <span className="text-sm font-normal text-slate-400">/ {totalGb.toFixed(0)} Go sur le PC</span></div>
          <Meter percent={totalGb ? (usedGb / totalGb) * 100 : 0} />
        </div>
        <div className="card">
          <div className="text-xs uppercase tracking-wide text-slate-400">CPU du serveur</div>
          <div className="mt-1 text-2xl font-semibold">{(snap?.cpu_percent ?? 0).toFixed(0)} %</div>
          <Meter percent={snap?.cpu_percent ?? 0} />
        </div>
        <div className="card">
          <div className="text-xs uppercase tracking-wide text-slate-400">Processeur du PC</div>
          <div className="mt-1 text-2xl font-semibold">{sys.cpu_cores} <span className="text-sm font-normal text-slate-400">cœurs logiques</span></div>
        </div>
      </div>

      <div className="card space-y-3">
        <h2 className="font-semibold">Mémoire (RAM)</h2>
        <p className="text-sm text-slate-400">
          Palworld n'a pas de réglage « RAM allouée » : il prend la mémoire dont il a besoin, et elle augmente avec le temps (fuites connues). Ce que vous pouvez fixer, c'est une <strong>limite</strong> : au-delà, l'application vous alerte ou redémarre le serveur proprement (annonce d'1 minute, puis sauvegarde).
        </p>
        <div className="flex flex-wrap items-end gap-3">
          <label className="text-sm">
            <span className="mb-1 block text-slate-400">Limite de RAM du serveur (Go) — vide = aucune</span>
            <input className="input !w-40" type="number" min={1} max={Math.max(1, Math.floor(totalGb))} step={0.5} value={perf.memory_limit_gb ?? ""}
              onChange={(e) => setPerf({ memory_limit_gb: e.target.value ? Number(e.target.value) : null })} />
          </label>
          <label className="flex items-center gap-2 pb-2 text-sm">
            <input type="checkbox" checked={perf.memory_limit_restart} disabled={perf.memory_limit_gb === null} onChange={(e) => setPerf({ memory_limit_restart: e.target.checked })} />
            Redémarrer automatiquement au-delà (sinon : alerte seulement)
          </label>
        </div>
        <p className="text-xs text-slate-500">Au plus un redémarrage automatique toutes les 30 minutes (évite une boucle si la limite est trop basse). Gardez de la marge pour Windows : ne dépassez pas environ {Math.max(1, Math.floor(totalGb * 0.85))} Go.</p>
      </div>

      <div className="card space-y-3">
        <h2 className="font-semibold">Processeur (CPU)</h2>
        <div className="grid gap-4 md:grid-cols-2">
          <label className="block text-sm">
            <span className="mb-1 block text-slate-400">Priorité du serveur</span>
            <select className="input" value={perf.priority} onChange={(e) => setPerf({ priority: e.target.value as Priority })}>
              {PRIORITIES.map((p) => <option key={p.v} value={p.v}>{p.label}</option>)}
            </select>
            <span className="mt-1 block text-xs text-slate-500">{PRIORITIES.find((p) => p.v === perf.priority)?.hint}</span>
          </label>
          <div className="text-sm">
            <span className="mb-1 block text-slate-400">Cœurs utilisables par le serveur ({selected.length} / {sys.cpu_cores})</span>
            <div className="flex flex-wrap gap-1" role="group" aria-label="Cœurs">
              {Array.from({ length: sys.cpu_cores }, (_, i) => (
                <button key={i} type="button" aria-pressed={selected.includes(i)} onClick={() => toggleCore(i)}
                  className={`h-8 w-10 rounded-lg text-xs font-medium ${selected.includes(i) ? "bg-pal-600 text-white" : "bg-slate-800 text-slate-400"}`}>{i}</button>
              ))}
            </div>
            <div className="mt-2 flex gap-2">
              <button type="button" className="btn !py-0.5 text-xs" onClick={() => setPerf({ cpu_cores: null })}>Tous</button>
              <button type="button" className="btn !py-0.5 text-xs" disabled={sys.cpu_cores <= 2} onClick={() => setPerf({ cpu_cores: Array.from({ length: sys.cpu_cores - 2 }, (_, i) => i + 2) })}>Laisser 2 cœurs à Windows</button>
            </div>
          </div>
        </div>
        <p className="text-xs text-slate-500">Priorité et cœurs sont réappliqués à chaque démarrage du serveur. Si rien ne change, lancez l'application en administrateur (le serveur peut tourner avec des droits plus élevés).</p>
      </div>

      <div className="card space-y-3">
        <h2 className="font-semibold">Options de lancement (threads)</h2>
        <div className="grid gap-3 md:grid-cols-2">
          {FLAGS.map((f) => (
            <label key={f.flag} className="block text-sm">
              <span className="flex items-center gap-2"><input type="checkbox" checked={has(s.launch_args, f.flag)} onChange={(e) => setS({ ...s, launch_args: toggle(s.launch_args, f.flag, e.target.checked) })} /> {f.label} <code className="text-xs text-slate-500">{f.flag}</code></span>
              <span className="ml-6 block text-xs text-slate-500">{f.hint}</span>
            </label>
          ))}
          <label className="block text-sm">
            <span className="mb-1 block text-slate-400">Threads de travail du serveur <code className="text-xs text-slate-500">{WORKERS}N</code> — vide = automatique</span>
            <input className="input !w-32" type="number" min={1} max={sys.cpu_cores} value={workers(s.launch_args)} onChange={(e) => setS({ ...s, launch_args: setWorkers(s.launch_args, e.target.value) })} />
          </label>
        </div>
        <p className="text-xs text-slate-500">Ces options ne s'appliquent qu'au <strong>prochain démarrage</strong> du serveur. Arguments actuels : <code>{s.launch_args.join(" ") || "(aucun)"}</code></p>
      </div>

      <div className="card space-y-1 text-sm text-slate-400">
        <h3 className="font-semibold text-slate-200">Réglages du jeu qui allègent le serveur</h3>
        <p>Dans l'onglet « Configuration » : <code>bEnableInvaderEnemy</code> (désactivé = moins de fuite mémoire), <code>ServerReplicatePawnCullDistance</code> (plus bas = moins de charge réseau), <code>BaseCampWorkerMaxNum</code>, <code>DropItemMaxNum</code> et <code>MaxBuildingLimitNum</code>.</p>
      </div>

      <div className="flex gap-2">
        <button className="btn-primary" disabled={busy} onClick={save}>{busy ? "Enregistrement…" : "Enregistrer et appliquer"}</button>
        <button className="btn" onClick={load}>Annuler les changements</button>
      </div>
    </div>
  );
}
