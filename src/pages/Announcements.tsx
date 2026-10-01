import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AnnouncementRule, AppSettings, KnownPlayer, PersonalMessage } from "../lib/types";

const newId = () => Math.random().toString(36).slice(2, 10);
const preview = (t: string, nom?: string) => t.replace("{nom}", nom ?? "").replace("{joueurs}", "3").replace("{max}", "32");

export default function Announcements({ notify }: { notify: (m: string) => void }) {
  const [s, setS] = useState<AppSettings | null>(null);
  const [dirty, setDirty] = useState(false);
  const [known, setKnown] = useState<KnownPlayer[]>([]);
  useEffect(() => { api.playersKnown().then(setKnown).catch(() => {}); }, []);

  const load = useCallback(() => { api.getSettings().then((x) => { setS(x); setDirty(false); }).catch((e) => notify(String(e))); }, [notify]);
  useEffect(() => { load(); }, [load]);
  if (!s) return <p className="text-slate-400">Chargement…</p>;

  const a = s.announcements;
  const set = (patch: Partial<typeof a>) => { setS({ ...s, announcements: { ...a, ...patch } }); setDirty(true); };
  const setRule = (id: string, patch: Partial<AnnouncementRule>) => set({ rules: a.rules.map((r) => (r.id === id ? { ...r, ...patch } : r)) });
  const setPersonal = (i: number, patch: Partial<PersonalMessage>) => set({ personal: a.personal.map((p, j) => (j === i ? { ...p, ...patch } : p)) });
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
        <h3 className="font-semibold">Messages personnels et retours</h3>
        <p className="text-sm text-slate-400">Un message réservé à un joueur remplace le message de bienvenue général quand il se connecte ; le jour de son anniversaire, c'est le message spécial. Le message de retour s'adresse à ceux qui reviennent après une longue absence (<code>{"{jours}"}</code> = nombre de jours).</p>
        {a.personal.map((p, i) => (
          <div key={i} className="flex flex-wrap items-center gap-2">
            <select className="input !w-48" value={p.user_id} aria-label="Joueur" onChange={(e) => { const k = known.find((x) => x.user_id === e.target.value); setPersonal(i, { user_id: e.target.value, name: k?.name ?? p.name }); }}>
              <option value="">Choisir un joueur…</option>
              {known.map((k) => <option key={k.user_id} value={k.user_id}>{k.name}</option>)}
              {p.user_id && !known.some((k) => k.user_id === p.user_id) && <option value={p.user_id}>{p.name || p.user_id}</option>}
            </select>
            <input className="input min-w-[14rem] flex-1" maxLength={300} placeholder="Message à chaque connexion (ex. Salut la chef {nom} !)" value={p.text ?? ""} onChange={(e) => setPersonal(i, { text: e.target.value || null })} />
            <input className="input !w-28" maxLength={5} placeholder="MM-JJ" title="Anniversaire, ex. 03-15" value={p.birthday ?? ""} onChange={(e) => setPersonal(i, { birthday: e.target.value || null })} />
            <button className="btn" onClick={() => set({ personal: a.personal.filter((_, j) => j !== i) })}>Supprimer</button>
          </div>
        ))}
        <button className="btn" onClick={() => set({ personal: [...a.personal, { user_id: "", name: "", text: null, birthday: null }] })}>+ Ajouter un message personnel</button>
        <div className="flex flex-wrap items-center gap-2 border-t border-slate-800 pt-3 text-sm">
          <span className="text-slate-400">Retour après</span>
          <input className="input !w-20" type="number" min={0} max={3650} value={a.welcome_back_days} onChange={(e) => set({ welcome_back_days: Math.max(0, Math.floor(+e.target.value || 0)) })} />
          <span className="text-slate-400">jours d'absence (0 = désactivé) :</span>
          <input className="input min-w-[14rem] flex-1" maxLength={300} placeholder="ex. Content de te revoir {nom}, ça fait {jours} jours !" value={a.welcome_back_text ?? ""} onChange={(e) => set({ welcome_back_text: e.target.value || null })} />
        </div>
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
