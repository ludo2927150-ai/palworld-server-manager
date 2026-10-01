import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings, Check } from "../lib/types";

const STEPS = ["Dossier du serveur", "SteamCMD", "API REST", "Options", "Vérification"] as const;

export default function Wizard({ onClose, notify }: { onClose: () => void; notify: (m: string) => void }) {
  const [step, setStep] = useState(0);
  const [s, setS] = useState<AppSettings | null>(null);
  const [dirs, setDirs] = useState<string[]>([]);
  const [cmds, setCmds] = useState<string[]>([]);
  const [pw, setPw] = useState("");
  const [checks, setChecks] = useState<Check[] | null>(null);

  useEffect(() => {
    api.getSettings().then(setS).catch((e) => notify(String(e)));
    api.detectSetup().then(([d, c]) => { setDirs(d); setCmds(c); }).catch(() => {});
  }, [notify]);

  if (!s) return null;
  const save = (next: AppSettings) => { setS(next); return api.saveSettings(next); };
  const finish = () => save({ ...s, setup_done: true }).then(onClose).catch((e) => notify(String(e)));
  const patch = (p: Partial<AppSettings>) => setS({ ...s, ...p });

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4" role="dialog" aria-modal="true" aria-label="Assistant de configuration">
      <div className="card max-h-[90vh] w-full max-w-2xl space-y-4 overflow-y-auto">
        <div className="flex items-center gap-3">
          <h2 className="text-lg font-semibold">Bienvenue — configuration en 5 étapes</h2>
          <button className="btn ml-auto" onClick={finish}>Passer</button>
        </div>
        <ol className="flex flex-wrap gap-2 text-xs">
          {STEPS.map((t, i) => <li key={t} className={`rounded px-2 py-1 ${i === step ? "bg-pal-500 text-black" : "bg-slate-800 text-slate-400"}`}>{i + 1}. {t}</li>)}
        </ol>

        {step === 0 && (
          <div className="space-y-2 text-sm">
            <p className="text-slate-400">Le dossier qui contient <code>PalServer.exe</code>.</p>
            {dirs.length > 0 && <div className="space-y-1">{dirs.map((d) => <button key={d} className={`block w-full rounded border px-3 py-2 text-left ${s.server_dir === d ? "border-pal-500" : "border-slate-700"}`} onClick={() => patch({ server_dir: d })}>{d}</button>)}</div>}
            {dirs.length === 0 && <p className="text-amber-300">Aucune installation détectée automatiquement. Saisissez le chemin, ou installez le serveur via SteamCMD (onglet Application).</p>}
            <input className="input" value={s.server_dir} onChange={(e) => patch({ server_dir: e.target.value })} placeholder="C:\steamcmd\steamapps\common\PalServer" />
          </div>
        )}

        {step === 1 && (
          <div className="space-y-2 text-sm">
            <p className="text-slate-400">SteamCMD sert à installer et mettre à jour le serveur (et à télécharger les mods).</p>
            {cmds.map((d) => <button key={d} className={`block w-full rounded border px-3 py-2 text-left ${s.steamcmd_path === d ? "border-pal-500" : "border-slate-700"}`} onClick={() => patch({ steamcmd_path: d })}>{d}</button>)}
            <input className="input" value={s.steamcmd_path} onChange={(e) => patch({ steamcmd_path: e.target.value })} placeholder="C:\steamcmd\steamcmd.exe" />
          </div>
        )}

        {step === 2 && (
          <div className="space-y-2 text-sm">
            <p className="text-slate-400">L'application pilote le serveur par son API REST (joueurs, annonces, sauvegarde propre). Choisissez un mot de passe admin : il sera écrit dans <code>PalWorldSettings.ini</code> (copie <code>.bak</code>) et mémorisé ici.</p>
            <div className="flex gap-2">
              <input className="input" type="password" value={pw} onChange={(e) => setPw(e.target.value)} placeholder="Mot de passe admin" />
              <button className="btn-primary" disabled={!pw.trim()} onClick={() => save(s).then(() => api.fixRest(pw)).then(() => { notify("API REST configurée"); setPw(""); }).catch((e) => notify(String(e)))}>Appliquer</button>
            </div>
          </div>
        )}

        {step === 3 && (
          <div className="space-y-3 text-sm">
            <label className="flex items-center gap-2"><input type="checkbox" checked={s.backup.enabled} onChange={(e) => patch({ backup: { ...s.backup, enabled: e.target.checked } })} /> Sauvegardes automatiques toutes les
              <input className="input w-20" type="number" min={5} value={s.backup.interval_minutes} onChange={(e) => patch({ backup: { ...s.backup, interval_minutes: Math.max(5, Number(e.target.value) || 5) } })} /> minutes</label>
            <label className="flex items-center gap-2"><input type="checkbox" checked={s.auto_restart} onChange={(e) => patch({ auto_restart: e.target.checked })} /> Relancer le serveur après un crash</label>
            <label className="flex items-center gap-2"><input type="checkbox" checked={s.start_server_on_launch} onChange={(e) => patch({ start_server_on_launch: e.target.checked })} /> Démarrer le serveur au lancement de l'application</label>
            <label className="flex items-center gap-2"><input type="checkbox" checked={s.close_to_tray} onChange={(e) => patch({ close_to_tray: e.target.checked })} /> Fermer la fenêtre = réduire dans la zone de notification</label>
          </div>
        )}

        {step === 4 && (
          <div className="space-y-2 text-sm">
            <button className="btn" onClick={() => save(s).then(() => api.diagnose()).then(setChecks).catch((e) => notify(String(e)))}>Lancer le diagnostic</button>
            {checks && <ul className="space-y-1">{checks.map((c) => <li key={c.id}><span className={c.ok ? "text-emerald-400" : "text-red-400"}>{c.ok ? "✔" : "✖"}</span> {c.label}</li>)}</ul>}
            <p className="text-xs text-slate-500">« API REST joignable » ne passe qu'une fois le serveur démarré. Vous pourrez tout modifier plus tard.</p>
          </div>
        )}

        <div className="flex justify-between border-t border-slate-800 pt-3">
          <button className="btn" disabled={step === 0} onClick={() => setStep(step - 1)}>Précédent</button>
          {step < STEPS.length - 1
            ? <button className="btn-primary" onClick={() => save(s).then(() => setStep(step + 1)).catch((e) => notify(String(e)))}>Suivant</button>
            : <button className="btn-primary" onClick={finish}>Terminer</button>}
        </div>
      </div>
    </div>
  );
}
