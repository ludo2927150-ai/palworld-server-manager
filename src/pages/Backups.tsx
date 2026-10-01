import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings, BackupInfo, RestoreTestView } from "../lib/types";

const Field = ({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) => (
  <label className="block text-sm">
    <span className="mb-1 block text-slate-400">{label}</span>
    {children}
    {hint && <span className="mt-1 block text-xs text-slate-500">{hint}</span>}
  </label>
);

/** Intervalle de sauvegarde interne du jeu (AutoSaveSpan, en secondes dans le .ini) exprimé en minutes. */
function GameAutoSave({ notify }: { notify: (m: string) => void }) {
  const [minutes, setMinutes] = useState<string>("");
  useEffect(() => {
    api.readWorld().then(({ options }) => {
      const v = Number(options.find((o) => o.key === "AutoSaveSpan")?.value);
      setMinutes(Number.isFinite(v) && v > 0 ? String(Math.round((v / 60) * 10) / 10) : "");
    }).catch(() => {});
  }, []);
  const apply = async () => {
    const m = Number(minutes);
    if (!(m >= 0.5 && m <= 120)) { notify("Entrez entre 0,5 et 120 minutes"); return; }
    try {
      const { options } = await api.readWorld();
      const seconds = String(Math.round(m * 60));
      const next = options.some((o) => o.key === "AutoSaveSpan")
        ? options.map((o) => (o.key === "AutoSaveSpan" ? { ...o, value: seconds } : o))
        : [...options, { key: "AutoSaveSpan", value: seconds, quoted: false }];
      await api.writeWorld(next);
      notify("Enregistré dans PalWorldSettings.ini — redémarrez le serveur pour l'appliquer");
    } catch (e) { notify(String(e)); }
  };
  return (
    <div className="card space-y-3">
      <h2 className="font-semibold">Sauvegarde interne du jeu</h2>
      <p className="text-sm text-slate-400">Le serveur Palworld écrit lui-même le monde sur le disque à intervalle régulier (réglage <code>AutoSaveSpan</code>). Plus c'est court, moins on perd en cas de crash.</p>
      <div className="flex flex-wrap items-end gap-2">
        <Field label="Toutes les (minutes)">
          <input className="input !w-32" type="number" min={0.5} max={120} step={0.5} value={minutes} onChange={(e) => setMinutes(e.target.value)} />
        </Field>
        <button className="btn" onClick={apply}>Appliquer au jeu</button>
      </div>
    </div>
  );
}

function RestoreTest({ notify }: { notify: (m: string) => void }) {
  const [r, setR] = useState<RestoreTestView | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => { api.restoreTestStatus().then(setR).catch(() => {}); }, []);
  return (
    <div className="card space-y-2">
      <h2 className="font-semibold">Test de restauration</h2>
      <p className="text-sm text-slate-400">Une sauvegarde jamais testée n'est pas une vraie sauvegarde. L'application relit régulièrement la dernière archive, l'extrait dans un dossier temporaire, vérifie que le monde est présent et complet, puis supprime tout. Votre vrai monde n'est jamais touché. Fréquence : onglet Application.</p>
      {r && r.t > 0 ? (
        <p className={`text-sm ${r.ok ? "text-emerald-300" : "text-red-300"}`} role="status">{r.ok ? "✔" : "✖"} {new Date(r.t * 1000).toLocaleString("fr-FR")} — {r.detail}</p>
      ) : <p className="text-sm text-slate-500">Jamais testé.</p>}
      <button className="btn" disabled={busy} onClick={() => { setBusy(true); api.restoreTestNow().then((x) => { setR(x); notify(x.ok ? "Test réussi" : "Test échoué : " + x.detail); }).catch((e) => notify(String(e))).finally(() => setBusy(false)); }}>
        {busy ? "Test en cours…" : "Tester maintenant"}
      </button>
    </div>
  );
}

const ORIGINS: [RegExp, string][] = [
  [/-arret-externe\.zip$/, "arrêt externe"], [/-arret-force\.zip$/, "arrêt forcé"], [/-arret\.zip$/, "arrêt"],
  [/-auto\.zip$/, "automatique"], [/-manuel\.zip$/, "manuel"], [/-avant-restauration\.zip$/, "avant restauration"],
];
const origin = (name: string) => ORIGINS.find(([re]) => re.test(name))?.[1] ?? null;

export default function Backups({ notify }: { notify: (m: string) => void }) {
  const [list, setList] = useState<BackupInfo[]>([]);
  const [s, setS] = useState<AppSettings | null>(null);
  const [busy, setBusy] = useState(false);
  const [restoring, setRestoring] = useState<string | null>(null);

  const refresh = useCallback(() => api.listBackups().then(setList).catch((e) => notify(String(e))), [notify]);
  useEffect(() => { refresh(); api.getSettings().then(setS).catch((e) => notify(String(e))); }, [refresh, notify]);

  const saveBackupSettings = (patch: Partial<AppSettings["backup"]>) => {
    if (!s) return;
    const next = { ...s, backup: { ...s.backup, ...patch } };
    setS(next);
    api.saveSettings(next).catch((e) => notify(String(e)));
  };

  return (
    <div className="space-y-4">
      {s && (
        <div className="card space-y-4">
          <h2 className="font-semibold">Sauvegardes automatiques</h2>
          <div className="grid gap-4 md:grid-cols-2">
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={s.backup.enabled} onChange={(e) => saveBackupSettings({ enabled: e.target.checked })} />
              Sauvegarde automatique pendant que le serveur tourne
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={s.backup.on_stop} onChange={(e) => saveBackupSettings({ on_stop: e.target.checked })} />
              Sauvegarde à chaque arrêt ou redémarrage du serveur
            </label>
            <Field label="Intervalle (minutes)" hint="S'applique immédiatement, sans redémarrer.">
              <input className="input !w-32" type="number" min={1} value={s.backup.interval_minutes} disabled={!s.backup.enabled}
                onChange={(e) => setS({ ...s, backup: { ...s.backup, interval_minutes: Math.max(1, Math.floor(+e.target.value || 1)) } })}
                onBlur={() => saveBackupSettings({ interval_minutes: s.backup.interval_minutes })} />
            </Field>
            <Field label="Nombre de sauvegardes conservées" hint="Les plus anciennes sont supprimées automatiquement.">
              <input className="input !w-32" type="number" min={1} value={s.backup.retention}
                onChange={(e) => setS({ ...s, backup: { ...s.backup, retention: Math.max(1, Math.floor(+e.target.value || 1)) } })}
                onBlur={() => saveBackupSettings({ retention: s.backup.retention })} />
            </Field>
          </div>
          <p className="text-xs text-slate-500">
            Le nom du fichier indique l'origine : <code>-auto</code> (intervalle), <code>-arret</code> (arrêt ou redémarrage), <code>-arret-externe</code> (serveur fermé hors de l'application), <code>-manuel</code> (bouton), <code>-avant-restauration</code> (état conservé juste avant un retour en arrière), <code>-garde</code> (protégée : jamais supprimée).
            Le second emplacement de copie se règle dans « Application ».
          </p>
        </div>
      )}

      <RestoreTest notify={notify} />
      <GameAutoSave notify={notify} />

      <div className="card">
        <div className="mb-3 flex items-center">
          <h2 className="font-semibold">Sauvegardes</h2>
          <button className="btn ml-auto" disabled={busy} title="Jamais supprimée par la rotation (avant un boss, un raid, une grosse modification…)"
            onClick={() => { const n = prompt("Nom de cette sauvegarde protégée (facultatif, ex. avant-raid) :", ""); if (n === null) return; setBusy(true); api.backupNowProtected(n).then(() => { notify("Sauvegarde protégée créée"); return refresh(); }).catch((e) => notify(String(e))).finally(() => setBusy(false)); }}>
            Sauvegarde protégée
          </button>
          <button className="btn-primary" disabled={busy}
            onClick={() => { setBusy(true); api.backupNow().then(() => { notify("Sauvegarde créée"); return refresh(); }).catch((e) => notify(String(e))).finally(() => setBusy(false)); }}>
            Sauvegarder maintenant
          </button>
        </div>
        <ul className="divide-y divide-slate-800 text-sm">
          {list.map((b, i) => {
            const o = origin(b.file_name);
            return (
              <li key={b.path} className="flex flex-wrap items-center gap-3 py-2">
                <div className="min-w-0">
                  <div className="truncate">{new Date(b.created).toLocaleString("fr-FR")}{i === 0 && <span className="ml-2 rounded bg-pal-600/30 px-1.5 py-0.5 text-xs text-pal-500">la plus récente</span>}{b.protected && <span className="ml-2 rounded bg-amber-500/20 px-1.5 py-0.5 text-xs text-amber-300">protégée</span>}</div>
                  <div className="truncate text-xs text-slate-500">{b.file_name} · {(b.size_bytes / 1e6).toFixed(1)} Mo{o ? ` · ${o}` : ""}</div>
                </div>
                <button className="btn ml-auto" disabled={restoring !== null} title={b.protected ? "Retirer la protection (elle pourra être supprimée par la rotation)" : "Protéger : ne sera jamais supprimée par la rotation"}
                  onClick={() => api.backupSetProtected(b.path, !b.protected).then(() => refresh()).catch((e) => notify(String(e)))}>
                  {b.protected ? "Déprotéger" : "Protéger"}
                </button>
                <button className="btn" disabled={restoring !== null}
                  onClick={() => api.verifyBackup(b.path).then((r) => notify(`Sauvegarde saine : ${r.files} fichiers, ${(r.bytes / 1e6).toFixed(1)} Mo`)).catch((e) => notify(`Sauvegarde défectueuse : ${e}`))}>
                  Vérifier
                </button>
                <button className="btn-primary" disabled={restoring !== null}
                  onClick={() => {
                    if (!confirm(`Revenir à la sauvegarde du ${new Date(b.created).toLocaleString("fr-FR")} ?\n\nSi le serveur tourne, il sera arrêté (l'état actuel est sauvegardé d'abord), la sauvegarde sera restaurée, puis le serveur redémarrera.`)) return;
                    setRestoring(b.path);
                    api.restoreBackup(b.path).then(() => { notify("Sauvegarde restaurée"); return refresh(); }).catch((e) => notify(String(e))).finally(() => setRestoring(null));
                  }}>
                  {restoring === b.path ? "Restauration…" : "Revenir à celle-ci"}
                </button>
              </li>
            );
          })}
          {list.length === 0 && <li className="py-2 text-slate-400">Aucune sauvegarde.</li>}
        </ul>
      </div>
    </div>
  );
}
