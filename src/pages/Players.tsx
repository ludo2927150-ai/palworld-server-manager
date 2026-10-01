import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import { t } from "../lib/i18n";
import type { AppSettings, BanEntry, KnownPlayer, Snapshot } from "../lib/types";
import PlayerDetail from "../components/PlayerDetail";

const TABS = ["En ligne", "Historique", "Bannis", "Liste blanche"] as const;
type Tab = (typeof TABS)[number];

const when = (t: number) => new Date(t * 1000).toLocaleString("fr-FR", { day: "2-digit", month: "2-digit", year: "2-digit", hour: "2-digit", minute: "2-digit" });
const dur = (s: number) => (s >= 3600 ? `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min` : `${Math.max(0, Math.floor(s / 60))} min`);

export default function Players({ snap, notify }: { snap: Snapshot | null; notify: (m: string) => void }) {
  const [tab, setTab] = useState<Tab>("En ligne");
  const [known, setKnown] = useState<KnownPlayer[]>([]);
  const [bans, setBans] = useState<BanEntry[]>([]);
  const [s, setS] = useState<AppSettings | null>(null);
  const [msg, setMsg] = useState("");
  const [unbanId, setUnbanId] = useState("");
  const [newId, setNewId] = useState("");
  const [q, setQ] = useState("");
  const [detailId, setDetailId] = useState<string | null>(null);
  const detail = known.find((k) => k.user_id === detailId) ?? null;

  const reload = useCallback(() => {
    api.playersKnown().then(setKnown).catch(() => {});
    api.playersBans().then(setBans).catch(() => {});
    api.getSettings().then(setS).catch(() => {});
  }, []);
  useEffect(() => { reload(); }, [reload]);

  const run = (f: () => Promise<unknown>, ok: string) => f().then(() => { notify(ok); reload(); }).catch((e) => notify(String(e)));
  const players = snap?.players ?? [];
  const saveAccess = (patch: Partial<AppSettings["access"]>) => {
    if (!s) return;
    const next = { ...s, access: { ...s.access, ...patch } };
    setS(next);
    api.saveSettings(next).then(reload).catch((e) => notify(String(e)));
  };
  const allow = (id: string, name: string) => s && !s.access.allowed.some((a) => a.user_id === id) && saveAccess({ allowed: [...s.access.allowed, { user_id: id, name }] });

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap gap-2">
        {TABS.map((tb) => <button key={tb} className={tb === tab ? "btn-primary" : "btn"} onClick={() => setTab(tb)}>{t(tb)}{tb === "Bannis" && bans.length ? ` (${bans.length})` : ""}</button>)}
      </div>

      {tab === "En ligne" && (
        <>
          <div className="card">
            <h2 className="mb-3 font-semibold">Joueurs connectés ({players.length})</h2>
            {players.length === 0 ? <p className="text-sm text-slate-400">{t("Personne en ligne.")}</p> : (
              <table className="w-full text-left text-sm">
                <thead className="text-slate-400"><tr><th>{t("Nom")}</th><th>{t("Niveau")}</th><th>Ping</th><th /></tr></thead>
                <tbody>
                  {players.map((p) => (
                    <tr key={p.userId} className="border-t border-slate-800">
                      <td className="py-2">{p.name}</td><td>{p.level}</td><td>{Math.round(p.ping)} ms</td>
                      <td className="space-x-2 text-right">
                        <button className="btn" onClick={() => run(() => api.kick(p.userId), `${p.name} expulsé`)}>{t("Expulser")}</button>
                        <button className="btn-danger" onClick={() => { const reason = prompt(`Bannir ${p.name} ? Raison (facultative) :`, ""); if (reason !== null) run(() => api.ban(p.userId, p.name, reason), `${p.name} banni`); }}>{t("Bannir")}</button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
          <div className="card">
            <div className="flex gap-2">
              <input className="input" placeholder={t("Annonce à tous les joueurs")} value={msg} onChange={(e) => setMsg(e.target.value)} />
              <button className="btn-primary" disabled={!msg.trim()} onClick={() => run(() => api.announce(msg), "Annonce envoyée").then(() => setMsg(""))}>{t("Envoyer")}</button>
            </div>
          </div>
        </>
      )}

      {tab === "Historique" && detail && <PlayerDetail key={detail.user_id} p={detail} notify={notify} onClose={() => setDetailId(null)} />}
      {tab === "Historique" && (
        <div className="card space-y-3">
          <div className="flex items-center gap-3">
            <h2 className="font-semibold">Joueurs déjà vus ({known.length})</h2>
            <input className="input ml-auto max-w-xs" placeholder="Rechercher un pseudo ou un identifiant" value={q} onChange={(e) => setQ(e.target.value)} />
          </div>
          {known.length === 0 ? <p className="text-sm text-slate-400">Aucun joueur enregistré pour l'instant : le carnet se remplit tant que le serveur tourne.</p> : (
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm">
                <thead className="text-slate-400"><tr><th>Joueur</th><th>{t("Dernière vue")}</th><th>{t("Temps de jeu")}</th><th>Sessions</th><th>{t("Première vue")}</th><th /></tr></thead>
                <tbody>
                  {known.filter((p) => !q.trim() || `${p.name} ${p.user_id} ${p.previous_names.join(" ")}`.toLowerCase().includes(q.trim().toLowerCase())).map((p) => (
                    <tr key={p.user_id} className="border-t border-slate-800">
                      <td className="py-2">
                        <div className="flex items-center gap-2">{p.online && <span className="h-2 w-2 rounded-full bg-emerald-400" title="en ligne" />}<strong>{p.name}</strong>
                          {p.banned && <span className="rounded bg-red-500/20 px-1.5 text-xs text-red-300">{t("banni")}</span>}
                          {p.allowed && <span className="rounded bg-emerald-500/20 px-1.5 text-xs text-emerald-300">{t("autorisé")}</span>}</div>
                        <div className="text-xs text-slate-500">{p.user_id}{p.previous_names.length ? ` · anciens pseudos : ${p.previous_names.join(", ")}` : ""}</div>
                      </td>
                      <td>{p.online ? "maintenant" : when(p.last_seen)}</td><td>{dur(p.total_secs)}</td><td>{p.sessions}</td><td>{when(p.first_seen)}</td>
                      <td className="space-x-2 text-right">
                        <button className="btn" onClick={() => setDetailId(p.user_id)}>Détails</button>
                        {!p.allowed && <button className="btn" onClick={() => allow(p.user_id, p.name)}>{t("Autoriser")}</button>}
                        {p.banned
                          ? <button className="btn" onClick={() => run(() => api.unban(p.user_id), `${p.name} débanni`)}>{t("Débannir")}</button>
                          : <button className="btn-danger" onClick={() => { const reason = prompt(`Bannir ${p.name} ? Raison (facultative) :`, ""); if (reason !== null) run(() => api.ban(p.user_id, p.name, reason), `${p.name} banni`); }}>{t("Bannir")}</button>}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          <p className="text-xs text-slate-500">Le temps de jeu est compté toutes les 5 secondes de présence observée par l'application (elle doit tourner pour compter).</p>
        </div>
      )}

      {tab === "Bannis" && (
        <div className="card space-y-3">
          <h2 className="font-semibold">Joueurs bannis depuis l'application ({bans.length})</h2>
          {bans.length === 0 ? <p className="text-sm text-slate-400">Aucun banni enregistré.</p> : (
            <ul className="divide-y divide-slate-800 text-sm">
              {bans.map((b) => (
                <li key={b.user_id} className="flex flex-wrap items-center gap-3 py-2">
                  <div><strong>{b.name || "(pseudo inconnu)"}</strong><div className="text-xs text-slate-500">{b.user_id} · {when(b.banned_at)}{b.reason ? ` · ${b.reason}` : ""}</div></div>
                  <button className="btn ml-auto" onClick={() => run(() => api.unban(b.user_id), "Joueur débanni")}>{t("Débannir")}</button>
                </li>
              ))}
            </ul>
          )}
          <div className="flex gap-2 border-t border-slate-800 pt-3">
            <input className="input" placeholder="Débannir un identifiant (ex. steam_7656…)" value={unbanId} onChange={(e) => setUnbanId(e.target.value)} />
            <button className="btn" disabled={!unbanId.trim()} onClick={() => run(() => api.unban(unbanId.trim()), "Joueur débanni").then(() => setUnbanId(""))}>{t("Débannir")}</button>
          </div>
          <p className="text-xs text-slate-500">Seuls les bannissements faits depuis l'application sont listés ; le serveur garde sa propre liste.</p>
        </div>
      )}

      {tab === "Liste blanche" && s && (
        <div className="card space-y-3">
          <div className="flex items-center gap-3">
            <h2 className="font-semibold">{t("Liste blanche")}</h2>
            <label className="ml-auto flex items-center gap-2 text-sm">
              <input type="checkbox" checked={s.access.whitelist_enabled} onChange={(e) => saveAccess({ whitelist_enabled: e.target.checked })} /> Activée (les autres sont expulsés)
            </label>
          </div>
          <p className="text-sm text-slate-400">Quand elle est activée, tout joueur absent de la liste est expulsé quelques secondes après sa connexion. Elle ne fonctionne que si l'application tourne et que l'API REST répond.</p>
          {s.access.whitelist_enabled && s.access.allowed.length === 0 && <p className="text-sm text-amber-300" role="status">{t("La liste est vide : par sécurité, personne n'est expulsé tant que vous n'avez ajouté aucun joueur.")}</p>}
          <ul className="divide-y divide-slate-800 text-sm">
            {s.access.allowed.map((a) => (
              <li key={a.user_id} className="flex items-center gap-3 py-2">
                <div><strong>{a.name || "(sans nom)"}</strong><div className="text-xs text-slate-500">{a.user_id}</div></div>
                <button className="btn ml-auto" onClick={() => saveAccess({ allowed: s.access.allowed.filter((x) => x.user_id !== a.user_id) })}>{t("Retirer")}</button>
              </li>
            ))}
            {s.access.allowed.length === 0 && <li className="py-2 text-slate-400">{t("Aucun joueur autorisé.")}</li>}
          </ul>
          <div className="flex flex-wrap gap-2">
            <button className="btn" disabled={players.length === 0} onClick={() => players.forEach((p) => allow(p.userId, p.name))}>Ajouter les joueurs connectés ({players.length})</button>
            <input className="input max-w-xs" placeholder="Identifiant (ex. steam_7656…)" value={newId} onChange={(e) => setNewId(e.target.value)} />
            <button className="btn" disabled={!newId.trim()} onClick={() => { const id = newId.trim(); const k = known.find((x) => x.user_id === id); allow(id, k?.name ?? ""); setNewId(""); }}>{t("Ajouter")}</button>
          </div>
          <label className="block text-sm"><span className="mb-1 block text-slate-400">{t("Message montré à l'expulsé")}</span>
            <input className="input" placeholder="Ce serveur est privé (liste blanche)." value={s.access.kick_message} onChange={(e) => setS({ ...s, access: { ...s.access, kick_message: e.target.value } })} onBlur={() => saveAccess({ kick_message: s.access.kick_message })} /></label>
        </div>
      )}
    </div>
  );
}
