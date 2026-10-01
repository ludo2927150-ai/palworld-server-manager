import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { KnownPlayer, PlayerSnapshot } from "../lib/types";

const when = (t: number) => new Date(t * 1000).toLocaleString("fr-FR", { day: "2-digit", month: "2-digit", year: "2-digit", hour: "2-digit", minute: "2-digit" });
const dur = (s: number) => (s >= 3600 ? `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min` : `${Math.max(0, Math.floor(s / 60))} min`);

function LevelChart({ points }: { points: [number, number][] }) {
  if (points.length < 2) return <p className="text-xs text-slate-500">Pas encore assez de points pour tracer la progression.</p>;
  const [t0, t1] = [points[0][0], points[points.length - 1][0]];
  const maxL = Math.max(...points.map((p) => p[1]));
  const x = (t: number) => 8 + ((t - t0) / Math.max(1, t1 - t0)) * 284;
  const y = (l: number) => 52 - (l / Math.max(1, maxL)) * 44;
  return (
    <svg viewBox="0 0 300 60" className="h-16 w-full max-w-md" role="img" aria-label={`Niveau de ${points[0][1]} à ${points[points.length - 1][1]}`}>
      <polyline fill="none" stroke="currentColor" strokeWidth="2" className="text-pal-500" points={points.map((p) => `${x(p[0])},${y(p[1])}`).join(" ")} />
      {points.map((p, i) => <circle key={i} cx={x(p[0])} cy={y(p[1])} r="2" className="fill-current text-pal-500"><title>{`Niveau ${p[1]} — ${when(p[0])}`}</title></circle>)}
    </svg>
  );
}

export default function PlayerDetail({ p, notify, onClose }: { p: KnownPlayer; notify: (m: string) => void; onClose: () => void }) {
  const [snaps, setSnaps] = useState<PlayerSnapshot[]>([]);
  const [busy, setBusy] = useState(false);
  const [exportDir, setExportDir] = useState("");
  const load = useCallback(() => { if (p.player_id) api.playerSnapshots(p.player_id).then(setSnaps).catch(() => {}); }, [p.player_id]);
  useEffect(() => { load(); }, [load]);
  const run = async (f: () => Promise<unknown>, ok: string) => { setBusy(true); try { await f(); notify(ok); load(); } catch (e) { notify(String(e)); } finally { setBusy(false); } };

  return (
    <div className="card space-y-4">
      <div className="flex items-center gap-3">
        <h2 className="text-lg font-semibold">{p.name}</h2>
        {p.online && <span className="rounded bg-emerald-500/20 px-1.5 text-xs text-emerald-300">en ligne</span>}
        {p.banned && <span className="rounded bg-red-500/20 px-1.5 text-xs text-red-300">banni</span>}
        <button className="btn ml-auto" onClick={onClose}>Fermer</button>
      </div>
      <dl className="grid gap-x-6 gap-y-2 text-sm md:grid-cols-2">
        <div><dt className="text-slate-400">Niveau</dt><dd>{p.level || "inconnu"}{p.max_level > p.level ? ` (max. ${p.max_level})` : ""}</dd></div>
        <div><dt className="text-slate-400">Constructions</dt><dd>{p.buildings}</dd></div>
        <div><dt className="text-slate-400">Dernière position (coordonnées du jeu)</dt><dd>{p.location ? `X ${Math.round(p.location[0])} · Y ${Math.round(p.location[1])}` : "inconnue"}</dd></div>
        <div><dt className="text-slate-400">Temps de jeu / sessions</dt><dd>{dur(p.total_secs)} · {p.sessions}</dd></div>
        <div><dt className="text-slate-400">Première / dernière vue</dt><dd>{when(p.first_seen)} · {p.online ? "maintenant" : when(p.last_seen)}</dd></div>
        <div><dt className="text-slate-400">Anciens pseudos</dt><dd>{p.previous_names.length ? p.previous_names.join(", ") : "aucun"}</dd></div>
        <div><dt className="text-slate-400">Identifiant de compte</dt><dd className="break-all text-xs">{p.user_id}</dd></div>
        <div><dt className="text-slate-400">Identifiant joueur (fichiers de sauvegarde)</dt><dd className="break-all text-xs">{p.player_id || "inconnu (le joueur doit se connecter une fois avec l'application ouverte)"}</dd></div>
      </dl>
      <div><h3 className="mb-1 text-sm font-medium text-slate-300">Progression du niveau</h3><LevelChart points={p.level_history} /></div>

      <div className="space-y-2 border-t border-slate-800 pt-3">
        <h3 className="font-medium">Sauvegardes de ce joueur</h3>
        <p className="text-xs text-amber-300">Chaque sauvegarde du monde enregistre aussi la fiche du joueur (niveau, statistiques, technologies). Ses objets, ses Pals et ses bases sont dans le fichier du monde, pas dans sa fiche : pour les récupérer, utilisez « Revenir à celle-ci » dans Sauvegardes (monde entier). Une fiche ne se restaure que dans le même monde ; la transférer sur un autre serveur n'est pas possible sans outil externe.</p>
        {!p.player_id ? <p className="text-sm text-slate-400">Identifiant joueur inconnu : impossible de retrouver ses fichiers.</p> : (
          <>
            <ul className="divide-y divide-slate-800 text-sm">
              {snaps.map((s, i) => (
                <li key={s.path} className="flex flex-wrap items-center gap-3 py-2">
                  <div>{new Date(s.taken).toLocaleString("fr-FR")}{i === 0 && <span className="ml-2 rounded bg-pal-600/30 px-1.5 py-0.5 text-xs text-pal-500">la plus récente</span>}<div className="text-xs text-slate-500">{(s.size_bytes / 1024).toFixed(0)} Ko</div></div>
                  <button className="btn ml-auto" disabled={busy} onClick={() => run(() => api.playerExport(s.path, exportDir || undefined).then((f) => notify(`Exporté : ${f}`)), "Export terminé")}>Exporter</button>
                  <button className="btn-primary" disabled={busy} onClick={() => confirm(`Restaurer la fiche de ${p.name} du ${new Date(s.taken).toLocaleString("fr-FR")} ?\n\nSi le serveur tourne, il sera arrêté (monde sauvegardé d'abord) puis relancé. Seule la fiche du joueur est remplacée ; l'ancienne est conservée en .avant-restauration.`) && run(() => api.playerRestore(s.path), "Fiche du joueur restaurée")}>Restaurer cette fiche</button>
                </li>
              ))}
              {snaps.length === 0 && <li className="py-2 text-slate-400">Aucune sauvegarde pour l'instant (la prochaine sauvegarde du monde en créera une).</li>}
            </ul>
            <div className="flex flex-wrap gap-2">
              <button className="btn" disabled={busy} onClick={() => run(() => api.playerSnapshotNow(p.player_id).then((s) => { if (!s) notify("Rien n'a changé depuis la dernière sauvegarde"); }), "Sauvegarde terminée")}>Sauvegarder maintenant</button>
              <input className="input max-w-xs" placeholder="Dossier d'export (vide = Bureau)" value={exportDir} onChange={(e) => setExportDir(e.target.value)} />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
