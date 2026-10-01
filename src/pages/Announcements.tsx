import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AnnouncementRule, AppSettings } from "../lib/types";

const newId = () => Math.random().toString(36).slice(2, 10);
const preview = (t: string, nom?: string) => t.replace("{nom}", nom ?? "").replace("{joueurs}", "3").replace("{max}", "32");

export default function Announcements({ notify }: { notify: (m: string) => void }) {
  const [s, setS] = useState<AppSettings | null>(null);
  const [dirty, setDirty] = useState(false);

  const load = useCallback(() => { api.getSettings().then((x) => { setS(x); setDirty(false); }).catch((e) => notify(String(e))); }, [notify]);
  useEffect(() => { load(); }, [load]);
  if (!s) return <p className="text-slate-400">Chargement…</p>;

  const a = s.announcements;
  const set = (patch: Partial<typeof a>) => { setS({ ...s, announcements: { ...a, ...patch } }); setDirty(true); };
  const setRule = (id: string, patch: Partial<AnnouncementRule>) => set({ rules: a.rules.map((r) => (r.id === id ? { ...r, ...patch } : r)) });
  const save = () => api.saveSettings(s).then(() => { notify("Annonces enregistrées"); setDirty(false); }).catch((e) => notify(String(e)));

  return (
    <div className="space-y-4">
      <div className="card space-y-3">
        <div className="flex items-center gap-3">
          <h2 className="font-semibold">Annonces automatiques en jeu</h2>
          <label className="ml-auto flex items-center gap-2 text-sm"><input type="checkbox" checked={a.enabled} onChange={(e) => set({ enabled: e.target.checked })} /> Activées</label>
        </div>
        <p className="text-sm text-slate-400">Messages envoyés à tous les joueurs via l'API REST du serveur. Variables : <code>{"{nom}"}</code> (bienvenue seulement), <code>{"{joueurs}"}</code> (connectés), <code>{"{max}"}</code> (places).</p>
        <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={a.only_with_players} onChange={(e) => set({ only_with_players: e.target.checked })} /> N'envoyer les rappels que si au moins un joueur est connecté</label>
      </div>

      <div className="card space-y-2">
        <h3 className="font-semibold">Message de bienvenue</h3>
        <input className="input" maxLength={300} placeholder="ex. Bienvenue {nom} ! Il y a {joueurs}/{max} joueurs." value={a.welcome ?? ""} onChange={(e) => set({ welcome: e.target.value || null })} />
        {a.welcome && <p className="text-xs text-slate-500">Aperçu : « {preview(a.welcome, "Alice")} »</p>}
        <p className="text-xs text-slate-500">Envoyé quand un joueur se connecte. Les joueurs déjà présents quand l'application démarre ne sont pas accueillis.</p>
      </div>

      <div className="card space-y-3">
        <h3 className="font-semibold">Rappels réguliers</h3>
        {a.rules.length === 0 && <p className="text-sm text-slate-400">Aucun rappel.</p>}
        {a.rules.map((r) => (
          <div key={r.id} className="flex flex-wrap items-center gap-2">
            <input type="checkbox" aria-label="Activé" checked={r.enabled} onChange={(e) => setRule(r.id, { enabled: e.target.checked })} />
            <input className="input min-w-[16rem] flex-1" maxLength={300} placeholder="Texte du rappel" value={r.text} onChange={(e) => setRule(r.id, { text: e.target.value })} />
            <label className="flex items-center gap-1 text-sm text-slate-400">toutes les
              <input className="input !w-20" type="number" min={1} max={1440} value={r.every_minutes} onChange={(e) => setRule(r.id, { every_minutes: Math.max(1, Math.floor(+e.target.value || 1)) })} /> min</label>
            <button className="btn" title="Envoyer maintenant pour tester" onClick={() => api.announce(preview(r.text)).then(() => notify("Annonce envoyée")).catch((e) => notify(String(e)))}>Tester</button>
            <button className="btn" onClick={() => set({ rules: a.rules.filter((x) => x.id !== r.id) })}>Supprimer</button>
          </div>
        ))}
        <button className="btn" onClick={() => set({ rules: [...a.rules, { id: newId(), text: "", every_minutes: 30, enabled: true }] })}>+ Ajouter un rappel</button>
      </div>

      <div className="flex gap-2">
        <button className="btn-primary" disabled={!dirty} onClick={save}>Enregistrer</button>
        <button className="btn" disabled={!dirty} onClick={load}>Annuler</button>
      </div>
    </div>
  );
}
