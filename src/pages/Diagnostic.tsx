import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Check } from "../lib/types";

export default function Diagnostic({ notify }: { notify: (m: string) => void }) {
  const [checks, setChecks] = useState<Check[] | null>(null);
  const [pw, setPw] = useState("");
  const run = useCallback(() => api.diagnose().then(setChecks).catch((e) => notify(String(e))), [notify]);
  useEffect(() => { run(); }, [run]);

  const restBroken = checks?.some((c) => ["rest_enabled", "admin_password", "password_match", "port_match"].includes(c.id) && !c.ok);
  return (
    <div className="space-y-4">
      <div className="card">
        <div className="mb-3 flex items-center">
          <h2 className="font-semibold">Diagnostic de l'installation</h2>
          <button className="btn ml-auto" onClick={run}>Relancer</button>
        </div>
        {!checks ? <p className="text-slate-400">Analyse…</p> : (
          <ul className="space-y-1.5 text-sm">
            {checks.map((c) => (
              <li key={c.id} className="flex gap-2">
                <span aria-label={c.ok ? "OK" : "Problème"} className={c.ok ? "text-emerald-400" : "text-red-400"}>{c.ok ? "✔" : "✖"}</span>
                <span>{c.label}{c.detail && <span className="ml-2 text-xs text-slate-500">{c.detail}</span>}</span>
              </li>
            ))}
          </ul>
        )}
        <p className="mt-3 text-xs text-slate-500">Le dossier du serveur, SteamCMD et le port se règlent dans l'onglet « Application ».</p>
      </div>
      {checks?.some((c) => c.id === "server_dir_match" && !c.ok) && (
        <div className="card space-y-3">
          <h3 className="font-semibold">Le serveur qui tourne n'est pas dans le dossier configuré</h3>
          <p className="text-sm text-slate-400">Sauvegardes, journal et configuration visent le dossier configuré : s'il est faux, rien de tout cela ne fonctionne. Vous pouvez adopter le dossier du serveur en cours d'exécution.</p>
          <button className="btn-primary" onClick={() => api.useRunningServerDir().then((d) => { notify(`Dossier du serveur : ${d}`); run(); }).catch((e) => notify(String(e)))}>Utiliser le dossier du serveur en marche</button>
        </div>
      )}
      {restBroken && (
        <div className="card space-y-3">
          <h3 className="font-semibold">Corriger l'API REST en un clic</h3>
          <p className="text-sm text-slate-400">Active <code>RESTAPIEnabled</code> dans PalWorldSettings.ini (copie <code>.bak</code> créée), définit le mot de passe admin et l'enregistre dans l'application. Redémarrez ensuite le serveur.</p>
          <div className="flex gap-2">
            <input className="input max-w-xs" type="password" placeholder="Mot de passe admin" value={pw} onChange={(e) => setPw(e.target.value)} />
            <button className="btn-primary" disabled={!pw.trim()} onClick={() => api.fixRest(pw).then(() => { notify("API REST configurée — redémarrez le serveur"); setPw(""); run(); }).catch((e) => notify(String(e)))}>Appliquer</button>
          </div>
        </div>
      )}
    </div>
  );
}
