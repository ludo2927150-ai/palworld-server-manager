import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { SeasonEvent, AppSettings, RuleAction, ScheduleRule } from "../lib/types";

const DAYS = ["L", "M", "M", "J", "V", "S", "D"];
const DAY_NAMES = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
const ACTIONS: { v: RuleAction; label: string }[] = [
  { v: "start", label: "Démarrer" }, { v: "restart", label: "Redémarrer" }, { v: "stop", label: "Arrêter" },
];
const validTime = (t: string) => /^([01]?\d|2[0-3]):[0-5]\d$/.test(t.trim());
const padTime = (t: string) => (validTime(t) ? t.trim().replace(/^(\d):/, "0$1:") : t);

function ScheduleEditor({ s, setS, profiles }: { s: AppSettings; setS: (v: AppSettings) => void; profiles: string[] }) {
  const set = (rules: ScheduleRule[]) => setS({ ...s, schedule: { ...s.schedule, rules } });
  const upd = (i: number, patch: Partial<ScheduleRule>) => set(s.schedule.rules.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  const toggleDay = (i: number, d: number) => {
    const cur = s.schedule.rules[i].days.length ? s.schedule.rules[i].days : [0, 1, 2, 3, 4, 5, 6];
    const next = cur.includes(d) ? cur.filter((x) => x !== d) : [...cur, d].sort();
    if (next.length === 0) return;                       // au moins un jour
    upd(i, { days: next.length === 7 ? [] : next });     // vide = tous les jours
  };
  return (
    <div className="card space-y-3">
      <div className="flex items-center gap-3">
        <h2 className="font-semibold">Horaires programmés</h2>
        <label className="ml-auto flex items-center gap-2 text-sm">
          <input type="checkbox" checked={s.schedule.enabled} onChange={(e) => setS({ ...s, schedule: { ...s.schedule, enabled: e.target.checked } })} /> Activés
        </label>
      </div>
      <p className="text-xs text-slate-500">
        Heure locale du PC. Avant un arrêt ou un redémarrage, les joueurs sont prévenus en jeu aux minutes indiquées ci-dessous, et une sauvegarde est faite.
        Si l'application est fermée à l'heure prévue, l'action est ignorée.
      </p>
      {s.schedule.rules.length === 0 && <p className="text-sm text-slate-400">Aucun horaire.</p>}
      {s.schedule.rules.map((r, i) => (
        <div key={i} className="flex flex-wrap items-center gap-2">
          <input className={`input !w-24 ${validTime(r.time) ? "" : "!border-red-500"}`} value={r.time} placeholder="HH:MM" aria-label="Heure (HH:MM)"
            onChange={(e) => upd(i, { time: e.target.value })} onBlur={(e) => upd(i, { time: padTime(e.target.value) })} />
          <select className="input !w-40" value={r.action} aria-label="Action" onChange={(e) => upd(i, { action: e.target.value as RuleAction })}>
            {ACTIONS.map((a) => <option key={a.v} value={a.v}>{a.label}</option>)}
          </select>
          {r.action !== "stop" && (
            <select className="input !w-44" value={r.profile ?? ""} aria-label="Profil de configuration" title="Profil appliqué juste avant l'exécution"
              onChange={(e) => upd(i, { profile: e.target.value || null })}>
              <option value="">Profil : inchangé</option>
              {profiles.map((p) => <option key={p} value={p}>Profil : {p}</option>)}
            </select>
          )}
          <div className="flex gap-1" role="group" aria-label="Jours">
            {DAYS.map((d, di) => {
              const on = r.days.length === 0 || r.days.includes(di);
              return (
                <button key={di} type="button" aria-pressed={on} title={DAY_NAMES[di]} onClick={() => toggleDay(i, di)}
                  className={`h-8 w-8 rounded-lg text-xs font-medium ${on ? "bg-pal-600 text-white" : "bg-slate-800 text-slate-400"}`}>{d}</button>
              );
            })}
          </div>
          <button type="button" className="btn ml-auto" onClick={() => set(s.schedule.rules.filter((_, j) => j !== i))}>Supprimer</button>
        </div>
      ))}
      <button type="button" className="btn" onClick={() => set([...s.schedule.rules, { time: "04:00", action: "restart", days: [], profile: null }])}>+ Ajouter un horaire</button>
    </div>
  );
}

function SeasonEditor({ s, setS, profiles }: { s: AppSettings; setS: (v: AppSettings) => void; profiles: string[] }) {
  const [status, setStatus] = useState<string | null>(null);
  useEffect(() => { api.seasonStatus().then((x) => setStatus(x.applied)).catch(() => {}); }, []);
  const set = (events: SeasonEvent[]) => setS({ ...s, schedule: { ...s.schedule, events } });
  const upd = (i: number, patch: Partial<SeasonEvent>) => set(s.schedule.events.map((e, j) => (j === i ? { ...e, ...patch } : e)));
  const today = new Date().toISOString().slice(0, 10);
  return (
    <div className="card space-y-3">
      <h2 className="font-semibold">Calendrier de saisons</h2>
      <p className="text-sm text-slate-400">Un profil de configuration appliqué pendant une période (« semaine XP ×2 » du 1er au 7), puis retour automatique à la configuration d'avant. L'application doit tourner : à partir de 04:00 le premier jour, elle enregistre la configuration actuelle (profil « avant-… »), applique le profil et redémarre le serveur proprement (préavis, sauvegarde, contrôle) ; le lendemain de la fin, elle revient en arrière. Créez d'abord le profil dans l'onglet Configuration.</p>
      {status && <p className="text-sm text-emerald-300" role="status">Événement en cours : {status}</p>}
      {s.schedule.events.map((e, i) => (
        <div key={i} className="flex flex-wrap items-center gap-2">
          <input className="input !w-44" placeholder="Nom (ex. Semaine XP)" value={e.name} onChange={(ev) => upd(i, { name: ev.target.value })} />
          <input className="input !w-40" type="date" aria-label="Début" value={e.start} onChange={(ev) => upd(i, { start: ev.target.value })} />
          <span className="text-slate-500">au</span>
          <input className="input !w-40" type="date" aria-label="Fin (incluse)" min={e.start || today} value={e.end} onChange={(ev) => upd(i, { end: ev.target.value })} />
          <select className="input !w-44" aria-label="Profil" value={e.profile} onChange={(ev) => upd(i, { profile: ev.target.value })}>
            <option value="">Profil…</option>
            {profiles.map((p) => <option key={p} value={p}>{p}</option>)}
          </select>
          <input className="input min-w-[12rem] flex-1" maxLength={300} placeholder="Annonce aux joueurs (facultatif)" value={e.announce ?? ""} onChange={(ev) => upd(i, { announce: ev.target.value || null })} />
          <button type="button" className="btn" onClick={() => set(s.schedule.events.filter((_, j) => j !== i))}>Supprimer</button>
        </div>
      ))}
      <button type="button" className="btn" onClick={() => set([...s.schedule.events, { name: "", start: today, end: today, profile: "", announce: null }])}>+ Ajouter un événement</button>
    </div>
  );
}

function LockCard({ notify }: { notify: (m: string) => void }) {
  const [st, setSt] = useState<{ enabled: boolean; auto_lock_minutes: number } | null>(null);
  const [cur, setCur] = useState("");
  const [pin, setPin] = useState("");
  const [auto, setAuto] = useState(0);
  const load = () => api.lockStatus().then((x) => { setSt(x); setAuto(x.auto_lock_minutes); }).catch(() => {});
  useEffect(() => { load(); }, []);
  if (!st) return null;
  const apply = (newPin: string | null) => api.lockSet(cur || null, newPin, auto).then(() => { notify(newPin ? "Code enregistré" : "Verrou retiré"); setCur(""); setPin(""); load(); }).catch((e) => notify(String(e)));
  return (
    <div className="card space-y-3">
      <h3 className="font-semibold">Verrou par code</h3>
      <p className="text-sm text-slate-400">Demande un code pour utiliser l'application. C'est un garde-fou contre l'usage involontaire ou curieux, pas une protection contre quelqu'un qui a accès à votre session Windows ou à vos fichiers.</p>
      <div className="flex flex-wrap items-end gap-2">
        {st.enabled && <label className="text-sm"><span className="mb-1 block text-slate-400">Code actuel</span><input className="input !w-40" type="password" autoComplete="off" value={cur} onChange={(e) => setCur(e.target.value)} /></label>}
        <label className="text-sm"><span className="mb-1 block text-slate-400">{st.enabled ? "Nouveau code" : "Code (4 à 32 caractères)"}</span><input className="input !w-48" type="password" autoComplete="off" value={pin} onChange={(e) => setPin(e.target.value)} /></label>
        <label className="text-sm"><span className="mb-1 block text-slate-400">Reverrouiller après (min, 0 = jamais)</span><input className="input !w-24" type="number" min={0} max={1440} value={auto} onChange={(e) => setAuto(Math.max(0, Math.floor(+e.target.value || 0)))} /></label>
        <button className="btn-primary" disabled={pin.length < 4} onClick={() => apply(pin)}>{st.enabled ? "Changer le code" : "Activer le verrou"}</button>
        {st.enabled && <button className="btn" onClick={() => apply(null)}>Retirer le verrou</button>}
      </div>
    </div>
  );
}

const Row = ({ label, children }: { label: string; children: React.ReactNode }) => (
  <label className="block text-sm"><span className="mb-1 block text-slate-400">{label}</span>{children}</label>
);

export default function Settings({ notify }: { notify: (m: string) => void }) {
  const [s, setS] = useState<AppSettings | null>(null);
  const [profiles, setProfiles] = useState<string[]>([]);
  useEffect(() => { api.profiles().then((l) => setProfiles(l.map((p) => p.name))).catch(() => {}); }, []);
  const [autostart, setAutostart] = useState(false);
  const [autostartError, setAutostartError] = useState<string | null>(null);
  useEffect(() => { api.getAutostart().then(setAutostart).catch((e) => setAutostartError(String(e))); }, []);
  // Coche tout de suite, puis annule et affiche l'erreur réelle si Windows refuse.
  const toggleAutostart = (on: boolean) => {
    setAutostart(on); setAutostartError(null);
    api.setAutostart(on).catch((e) => { setAutostart(!on); setAutostartError(String(e)); });
  };
  useEffect(() => { api.getSettings().then(setS).catch((e) => notify(String(e))); }, [notify]);
  if (!s) return <p className="text-slate-400">Chargement…</p>;
  const nul = (v: string) => (v.trim() ? v : null);

  return (
    <div className="space-y-4">
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Afficher la console du serveur dans l'application (lance le moteur directement, sans fenêtre noire ; se replie sur PalServer.exe en cas d'échec)"><input type="checkbox" checked={s.capture_console} onChange={(e) => setS({ ...s, capture_console: e.target.checked })} /></Row>
        <Row label="Fichier journal du serveur (vide = détection automatique)"><input className="input" placeholder="C:\\…\\Pal\\Saved\\Logs\\Pal.log" value={s.log_file ?? ""} onChange={(e) => setS({ ...s, log_file: e.target.value.trim() ? e.target.value : null })} /></Row>
        <Row label="Dossier du serveur (PalServer.exe)"><input className="input" value={s.server_dir} onChange={(e) => setS({ ...s, server_dir: e.target.value })} /></Row>
        <Row label="Arguments de lancement"><input className="input" value={s.launch_args.join(" ")} onChange={(e) => setS({ ...s, launch_args: e.target.value.split(" ").filter(Boolean) })} /></Row>
        <Row label="Mot de passe admin (API REST)"><input className="input" type="password" value={s.rest.admin_password} onChange={(e) => setS({ ...s, rest: { ...s.rest, admin_password: e.target.value } })} /></Row>
        <Row label="Lancer avec Windows (réduit dans la zone de notification)">
          <input type="checkbox" checked={autostart} onChange={(e) => toggleAutostart(e.target.checked)} />
          <span className="ml-2 text-xs text-slate-500">à utiliser avec l'application installée (pas avec « npm run tauri dev »)</span>
          {autostartError && <span className="mt-1 block text-xs text-red-400" role="alert">{autostartError}</span>}
        </Row>
        <Row label="Démarrer le serveur automatiquement au lancement de l'application (≈ 15 s après)"><input type="checkbox" checked={s.start_server_on_launch} onChange={(e) => setS({ ...s, start_server_on_launch: e.target.checked })} /></Row>
        <Row label="Fermer la fenêtre = réduire dans la zone de notification"><input type="checkbox" checked={s.close_to_tray} onChange={(e) => setS({ ...s, close_to_tray: e.target.checked })} /></Row>
        <Row label="Redémarrage auto après crash"><input type="checkbox" checked={s.auto_restart} onChange={(e) => setS({ ...s, auto_restart: e.target.checked })} /></Row>
      </div>
      <ScheduleEditor s={s} setS={setS} profiles={profiles} />
      <SeasonEditor s={s} setS={setS} profiles={profiles} />
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Chemin de steamcmd.exe"><input className="input" value={s.steamcmd_path} onChange={(e) => setS({ ...s, steamcmd_path: e.target.value })} /></Row>
        <Row label="Annonces (minutes avant, ex. 15, 5, 1)"><input className="input" value={s.schedule.announce_minutes.join(", ")} onChange={(e) => setS({ ...s, schedule: { ...s.schedule, announce_minutes: e.target.value.split(",").map((t) => parseInt(t, 10)).filter((n) => n > 0) } })} /></Row>
        <Row label="Mémoire : attendre que le serveur soit vide, au plus (minutes, 0 = redémarrer après 1 min de préavis)"><input className="input" type="number" min={0} max={240} value={s.schedule.memory_restart_wait_empty_minutes} onChange={(e) => setS({ ...s, schedule: { ...s.schedule, memory_restart_wait_empty_minutes: Math.max(0, +e.target.value || 0) } })} /></Row>
        <Row label="Redémarrer si RAM ≥ (%) — vide = jamais"><input className="input" type="number" min={1} max={100} value={s.schedule.memory_restart_percent ?? ""} onChange={(e) => setS({ ...s, schedule: { ...s.schedule, memory_restart_percent: e.target.value ? +e.target.value : null } })} /></Row>
      </div>
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Backup automatique"><input type="checkbox" checked={s.backup.enabled} onChange={(e) => setS({ ...s, backup: { ...s.backup, enabled: e.target.checked } })} /></Row>
        <Row label="Intervalle (minutes)"><input className="input" type="number" min={1} value={s.backup.interval_minutes} onChange={(e) => setS({ ...s, backup: { ...s.backup, interval_minutes: +e.target.value } })} /></Row>
        <Row label="Rétention par paliers (24 h complètes, puis 1 par jour sur 7 jours, puis 1 par semaine sur 4 semaines) au lieu d'un nombre fixe"><input type="checkbox" checked={s.backup.tiered} onChange={(e) => setS({ ...s, backup: { ...s.backup, tiered: e.target.checked } })} /></Row>
        <Row label="Sauvegarder aussi la fiche de chaque joueur à chaque sauvegarde du monde"><input type="checkbox" checked={s.backup.player_snapshots} onChange={(e) => setS({ ...s, backup: { ...s.backup, player_snapshots: e.target.checked } })} /></Row>
        <Row label="Test de restauration automatique tous les (jours, vide = désactivé)"><input className="input" type="number" min={1} max={90} value={s.backup.restore_test_days ?? ""} onChange={(e) => setS({ ...s, backup: { ...s.backup, restore_test_days: e.target.value ? Math.max(1, +e.target.value) : null } })} /></Row>
        <Row label="Versions gardées par joueur"><input className="input" type="number" min={1} value={s.backup.player_keep} onChange={(e) => setS({ ...s, backup: { ...s.backup, player_keep: Math.max(1, +e.target.value || 1) } })} /></Row>
        <Row label="Nombre de backups conservés (sans paliers)"><input className="input" type="number" min={1} value={s.backup.retention} onChange={(e) => setS({ ...s, backup: { ...s.backup, retention: +e.target.value } })} /></Row>
        <Row label="Second emplacement (copie de chaque sauvegarde, ex. autre disque)"><input className="input" placeholder="D:\Sauvegardes\Palworld" value={s.backup.mirror_destination ?? ""} onChange={(e) => setS({ ...s, backup: { ...s.backup, mirror_destination: e.target.value.trim() ? e.target.value : null } })} /></Row>
        <Row label="Dossier de destination"><input className="input" value={s.backup.destination} onChange={(e) => setS({ ...s, backup: { ...s.backup, destination: e.target.value } })} /></Row>
      </div>
      <div className="card grid gap-4 md:grid-cols-2">
        <Row label="Webhook Discord"><input className="input" value={s.alerts.discord_webhook ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, discord_webhook: nul(e.target.value) } })} /></Row>
        <Row label="URL ntfy (push mobile)"><input className="input" placeholder="https://ntfy.sh/mon-topic" value={s.alerts.ntfy_url ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, ntfy_url: nul(e.target.value) } })} /></Row>
        <Row label="Notifications Windows"><input type="checkbox" checked={s.alerts.desktop} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, desktop: e.target.checked } })} /></Row>
        <Row label="Résumé quotidien à (HH:MM, vide = désactivé)"><input className="input" placeholder="20:00" value={s.alerts.daily_summary_time ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, daily_summary_time: nul(e.target.value) } })} /></Row>
        <Row label="Seuil mémoire (%)"><input className="input" type="number" min={1} max={100} value={s.alerts.memory_threshold_percent ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, memory_threshold_percent: e.target.value ? +e.target.value : null } })} /></Row>
        <Row label="Rapport hebdomadaire (Discord / ntfy) le"><select className="input" value={s.alerts.weekly_report_day ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, weekly_report_day: e.target.value === "" ? null : +e.target.value } })}>
          <option value="">Désactivé</option>{DAY_NAMES.map((d, i) => <option key={d} value={i}>{d}</option>)}
        </select></Row>
        <Row label="Alerte si aucune sauvegarde depuis (heures, vide = désactivé)"><input className="input" type="number" min={1} value={s.alerts.stale_backup_hours ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, stale_backup_hours: e.target.value ? +e.target.value : null } })} /></Row>
        <Row label="Alerte si espace disque libre sous (Go, vide = désactivé)"><input className="input" type="number" min={1} value={s.alerts.min_free_disk_gb ?? ""} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, min_free_disk_gb: e.target.value ? +e.target.value : null } })} /></Row>
        <Row label="Alertes"><div className="flex gap-4">
          {([["on_crash", "Crash"], ["on_player_join", "Connexion"], ["on_player_leave", "Déconnexion"]] as const).map(([k, l]) => (
            <label key={k}><input type="checkbox" checked={s.alerts[k]} onChange={(e) => setS({ ...s, alerts: { ...s.alerts, [k]: e.target.checked } })} /> {l}</label>
          ))}
        </div></Row>
      </div>
      <div className="card space-y-3">
        <h3 className="font-semibold">Mise à jour automatique du serveur Palworld</h3>
        <p className="text-sm text-slate-400">Compare la version installée à la version publique de Steam (via SteamCMD). Si une mise à jour existe : annonce aux joueurs présents, sauvegarde, arrêt, mise à jour, relance. Une seule tentative par version : en cas d'échec vous êtes alerté, sans boucle.</p>
        <div className="grid gap-4 md:grid-cols-3">
          <Row label="Activée"><input type="checkbox" checked={s.server_update.enabled} onChange={(e) => setS({ ...s, server_update: { ...s.server_update, enabled: e.target.checked } })} /></Row>
          <Row label="Vérifier toutes les (minutes, min. 15)"><input className="input" type="number" min={15} value={s.server_update.check_every_minutes} onChange={(e) => setS({ ...s, server_update: { ...s.server_update, check_every_minutes: +e.target.value } })} /></Row>
          <Row label="Préavis aux joueurs (minutes, max. 30)"><input className="input" type="number" min={0} max={30} value={s.server_update.warn_minutes} onChange={(e) => setS({ ...s, server_update: { ...s.server_update, warn_minutes: +e.target.value } })} /></Row>
        </div>
        <button className="btn" onClick={() => api.checkServerUpdate().then((u) => notify(u.outdated ? `Mise à jour disponible : ${u.installed ?? "?"} → ${u.latest}` : `Serveur à jour (build ${u.latest})`)).catch((e) => notify(String(e)))}>Vérifier maintenant</button>
      </div>
      <div className="card space-y-3">
        <h3 className="font-semibold">Bot Discord (commandes /statut, /joueurs, /sauvegarde, /demarrer, /arreter, /redemarrer, /annonce)</h3>
        <p className="text-sm text-slate-400">Le bot se connecte à Discord depuis ce PC (aucun port ouvert). Créez une application sur discord.com/developers, ajoutez un Bot, copiez son jeton ici, puis invitez-le sur votre serveur Discord avec la portée « applications.commands ». <strong>Seuls les identifiants Discord listés ci-dessous peuvent l'utiliser</strong> (mode développeur Discord → clic droit sur votre profil → « Copier l'identifiant »). Liste vide = personne.</p>
        <div className="grid gap-4 md:grid-cols-2">
          <Row label="Activé"><input type="checkbox" checked={s.discord_bot.enabled} onChange={(e) => setS({ ...s, discord_bot: { ...s.discord_bot, enabled: e.target.checked } })} /></Row>
          <Row label="Jeton du bot"><input className="input" type="password" autoComplete="off" value={s.discord_bot.bot_token} onChange={(e) => setS({ ...s, discord_bot: { ...s.discord_bot, bot_token: e.target.value } })} /></Row>
          <Row label="Identifiants Discord autorisés (séparés par des virgules)"><input className="input" defaultValue={s.discord_bot.allowed_user_ids.join(", ")} onBlur={(e) => setS({ ...s, discord_bot: { ...s.discord_bot, allowed_user_ids: e.target.value.split(/[,\s]+/).filter((x) => /^\d+$/.test(x)) } })} /></Row>
          <Row label="Autoriser le contrôle (démarrer, arrêter, redémarrer, sauvegarder, annoncer) — sinon lecture seule"><input type="checkbox" checked={s.discord_bot.allow_control} onChange={(e) => setS({ ...s, discord_bot: { ...s.discord_bot, allow_control: e.target.checked } })} /></Row>
        </div>
      </div>
      <LockCard notify={notify} />
      <div className="card space-y-3">
        <h3 className="font-semibold">Auto-réparation</h3>
        <p className="text-sm text-slate-400">Si le processus tourne mais que l'API ne répond plus (après avoir répondu au moins une fois), le serveur est redémarré. Après plusieurs crashs rapprochés, la relance automatique s'arrête et vous êtes alerté, au lieu de boucler. Tout redémarrage automatique est suivi d'un contrôle : si le serveur ne répond pas, les mods fautifs sont désactivés, puis le monde est restauré depuis la sauvegarde de sûreté.</p>
        <div className="grid gap-4 md:grid-cols-4">
          <Row label="Activée"><input type="checkbox" checked={s.watchdog.enabled} onChange={(e) => setS({ ...s, watchdog: { ...s.watchdog, enabled: e.target.checked } })} /></Row>
          <Row label="Gel : redémarrer après (minutes)"><input className="input" type="number" min={1} value={s.watchdog.hung_minutes} onChange={(e) => setS({ ...s, watchdog: { ...s.watchdog, hung_minutes: Math.max(1, +e.target.value || 1) } })} /></Row>
          <Row label="Boucle de crashs : nombre"><input className="input" type="number" min={2} value={s.watchdog.crash_loop_max} onChange={(e) => setS({ ...s, watchdog: { ...s.watchdog, crash_loop_max: Math.max(2, +e.target.value || 2) } })} /></Row>
          <Row label="…en (minutes)"><input className="input" type="number" min={1} value={s.watchdog.crash_loop_window_minutes} onChange={(e) => setS({ ...s, watchdog: { ...s.watchdog, crash_loop_window_minutes: Math.max(1, +e.target.value || 1) } })} /></Row>
        </div>
      </div>
      <div className="card space-y-3">
        <h3 className="font-semibold">Mods automatiques</h3>
        <p className="text-sm text-slate-400">Les mods ajoutés depuis l'onglet Mods sont « gérés » : téléchargés et mis à jour via SteamCMD. Quand un mod change, le serveur est redémarré proprement (préavis, sauvegarde, contrôle). Certains mods du Workshop refusent le téléchargement anonyme : abonnez-vous-y alors depuis Steam.</p>
        <div className="grid gap-4 md:grid-cols-3">
          <Row label="Mise à jour automatique"><input type="checkbox" checked={s.mod_automation.auto_update} onChange={(e) => setS({ ...s, mod_automation: { ...s.mod_automation, auto_update: e.target.checked } })} /></Row>
          <Row label="Vérifier toutes les (minutes, min. 30)"><input className="input" type="number" min={30} value={s.mod_automation.check_every_minutes} onChange={(e) => setS({ ...s, mod_automation: { ...s.mod_automation, check_every_minutes: Math.max(30, +e.target.value || 30) } })} /></Row>
          <Row label="Mods gérés"><span className="text-slate-300">{s.mod_automation.managed_ids.length ? s.mod_automation.managed_ids.join(", ") : "aucun"}</span></Row>
        </div>
      </div>
      <div className="card space-y-3">
        <h3 className="font-semibold">Ouverture automatique du port de jeu (UPnP)</h3>
        <p className="text-sm text-amber-300">Attention : cette option ouvre le port UDP du jeu (8211 par défaut) de votre box vers Internet pour que vos amis puissent se connecter. Elle n'ouvre jamais l'API REST ni l'accès mobile. Désactivée par défaut ; elle ne fonctionne que si UPnP est activé sur votre box. Le serveur est alors joignable par n'importe qui connaissant votre adresse : pensez au mot de passe serveur ou à la liste blanche.</p>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={s.upnp.enabled} onChange={(e) => {
            if (e.target.checked && !confirm("Ouvrir le port de jeu de votre box vers Internet ?\n\nLe serveur de jeu sera joignable depuis Internet. L'API REST et l'accès mobile ne sont pas concernés.")) return;
            setS({ ...s, upnp: { enabled: e.target.checked } });
          }} /> Ouvrir le port de jeu automatiquement
        </label>
        <button className="btn" onClick={() => api.upnpTest().then((m) => notify(m)).catch((e) => notify(String(e)))}>Tester maintenant (ouvre le port une fois)</button>
      </div>
      <div className="flex gap-2">
        <button className="btn-primary" onClick={() => s.schedule.rules.some((r) => !validTime(r.time)) ? notify("Un horaire programmé n'a pas un format HH:MM valide") : api.saveSettings(s).then(() => notify("Paramètres enregistrés")).catch((e) => notify(String(e)))}>Enregistrer</button>
        <button className="btn" onClick={() => api.testAlert().then(() => notify("Alerte de test envoyée")).catch((e) => notify(String(e)))}>Tester les alertes</button>
      </div>
    </div>
  );
}
