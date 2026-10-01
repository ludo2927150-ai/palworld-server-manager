import { useCallback, useEffect, useState } from "react";
import QRCode from "qrcode";
import { api } from "../lib/api";
import type { AppSettings, Guest, Perm, RemoteInfo } from "../lib/types";

const mask = (url: string) => url.replace(/#token=.*/, "#token=••••••••");

const PERMS: { p: Perm; label: string; risky?: boolean }[] = [
  { p: "status", label: "Voir l'état du serveur (en ligne, CPU, RAM, nombre de joueurs)" },
  { p: "players", label: "Voir les pseudos des joueurs et l'historique des sessions" },
  { p: "logs", label: "Voir le journal du serveur" },
  { p: "charts", label: "Voir les graphiques" },
  { p: "announce", label: "Envoyer une annonce en jeu" },
  { p: "backup", label: "Lancer une sauvegarde" },
  { p: "start", label: "Démarrer le serveur", risky: true },
  { p: "restart", label: "Redémarrer le serveur", risky: true },
  { p: "stop", label: "Arrêter le serveur", risky: true },
  { p: "kick", label: "Expulser un joueur", risky: true },
];
const PRESETS: { name: string; perms: Perm[] }[] = [
  { name: "Spectateur", perms: ["status", "players", "charts"] },
  { name: "Modérateur", perms: ["status", "players", "charts", "logs", "announce", "kick"] },
  { name: "Contrôle total (sans réglages)", perms: ["status", "players", "charts", "logs", "announce", "backup", "start", "restart", "stop", "kick"] },
];
const DURATIONS: { label: string; hours: number | null }[] = [
  { label: "1 heure", hours: 1 }, { label: "24 heures", hours: 24 }, { label: "7 jours", hours: 168 }, { label: "30 jours", hours: 720 }, { label: "Illimitée", hours: null },
];
const permLabel = (p: Perm) => PERMS.find((x) => x.p === p)?.label.split(" (")[0] ?? p;
const expiry = (g: Guest) => {
  if (g.expires_at === null) return "n'expire pas";
  const left = g.expires_at * 1000 - Date.now();
  if (left <= 0) return "expirée";
  const h = Math.ceil(left / 3_600_000);
  return h < 48 ? `expire dans ${h} h` : `expire dans ${Math.ceil(h / 24)} j`;
};

function GuestEditor({ s, info, reload, notify }: { s: AppSettings; info: RemoteInfo; reload: () => void; notify: (m: string) => void }) {
  const [name, setName] = useState("");
  const [perms, setPerms] = useState<Perm[]>(PRESETS[0].perms);
  const [hours, setHours] = useState<number | null>(24);
  const [baseIdx, setBaseIdx] = useState(0);
  const [openId, setOpenId] = useState<string | null>(null);
  const toggle = (p: Perm) => setPerms((cur) => (cur.includes(p) ? cur.filter((x) => x !== p) : [...cur, p]));
  const base = info.bases[Math.min(baseIdx, Math.max(0, info.bases.length - 1))];

  const create = () => api.createGuest(name, perms, hours).then((g) => { setName(""); setOpenId(g.id); notify(`Invitation créée pour ${g.name}`); reload(); }).catch((e) => notify(String(e)));

  return (
    <div className="card space-y-4">
      <h2 className="font-semibold">Invités : donner un accès limité à vos amis</h2>
      <p className="text-sm text-slate-400">Chaque invité reçoit son propre QR code avec uniquement les droits que vous choisissez. Vous pouvez révoquer un invité à tout moment, sans toucher aux autres.</p>

      {s.remote.guests.length === 0 && <p className="text-sm text-slate-500">Aucun invité pour l'instant.</p>}
      <ul className="space-y-3">
        {s.remote.guests.map((g) => (
          <li key={g.id} className="space-y-2 rounded-lg border border-slate-800 p-3">
            <div className="flex flex-wrap items-center gap-2">
              <strong>{g.name}</strong>
              <span className={`text-xs ${expiry(g) === "expirée" ? "text-red-400" : "text-slate-500"}`}>{expiry(g)}</span>
              <button className="btn ml-auto" onClick={() => setOpenId(openId === g.id ? null : g.id)}>{openId === g.id ? "Masquer le QR code" : "QR code"}</button>
              <button className="btn-danger" onClick={() => confirm(`Révoquer l'accès de ${g.name} ? Son QR code cessera de fonctionner immédiatement.`) && api.revokeGuest(g.id).then(() => { notify("Accès révoqué"); reload(); }).catch((e) => notify(String(e)))}>Révoquer</button>
            </div>
            <div className="flex flex-wrap gap-1 text-xs">
              {g.perms.map((p) => <span key={p} className="rounded bg-slate-800 px-1.5 py-0.5 text-slate-300">{permLabel(p)}</span>)}
            </div>
            {openId === g.id && (base
              ? <UrlCard openQr label={`Invitation de ${g.name} — ${base.label}`} url={`${base.url}#token=${g.token}`} notify={notify} />
              : <p className="text-sm text-amber-300">Activez l'accès mobile ci-dessus pour obtenir l'adresse.</p>)}
          </li>
        ))}
      </ul>
      {info.bases.length > 1 && (
        <label className="block text-sm"><span className="mb-1 block text-slate-400">Adresse utilisée dans les QR codes des invités</span>
          <select className="input max-w-sm" value={baseIdx} onChange={(e) => setBaseIdx(+e.target.value)}>{info.bases.map((b, i) => <option key={b.label} value={i}>{b.label}</option>)}</select>
        </label>
      )}

      <div className="space-y-3 border-t border-slate-800 pt-4">
        <h3 className="font-semibold">Nouvelle invitation</h3>
        <div className="grid gap-3 md:grid-cols-2">
          <label className="block text-sm"><span className="mb-1 block text-slate-400">Nom de l'invité</span>
            <input className="input" maxLength={40} placeholder="ex. Thomas" value={name} onChange={(e) => setName(e.target.value)} /></label>
          <label className="block text-sm"><span className="mb-1 block text-slate-400">Valable</span>
            <select className="input" value={hours ?? "inf"} onChange={(e) => setHours(e.target.value === "inf" ? null : +e.target.value)}>
              {DURATIONS.map((d) => <option key={d.label} value={d.hours ?? "inf"}>{d.label}</option>)}
            </select></label>
        </div>
        <div className="flex flex-wrap gap-2 text-sm"><span className="text-slate-400">Modèles :</span>
          {PRESETS.map((pr) => <button key={pr.name} type="button" className="btn !py-0.5 text-xs" onClick={() => setPerms(pr.perms)}>{pr.name}</button>)}
        </div>
        <div className="grid gap-1 md:grid-cols-2">
          {PERMS.map(({ p, label, risky }) => (
            <label key={p} className="flex items-start gap-2 text-sm">
              <input type="checkbox" className="mt-1" checked={perms.includes(p)} onChange={() => toggle(p)} />
              <span>{label}{risky && <span className="ml-1 text-xs text-amber-400">⚠ action sensible</span>}</span>
            </label>
          ))}
        </div>
        <button className="btn-primary" disabled={!name.trim() || perms.length === 0} onClick={create}>Créer l'invitation</button>
        <p className="text-xs text-slate-500">Jamais accessibles aux invités : réglages de l'application, configuration du monde, restauration de sauvegarde, mise à jour, bannissement.</p>
      </div>

      <div className="rounded-lg border border-slate-800 p-3 text-sm text-slate-400">
        <strong className="text-slate-200">Comment vos amis se connectent</strong>
        <ul className="mt-1 list-disc space-y-1 pl-5">
          <li><strong>Sur votre Wi-Fi</strong> : ils scannent le QR code « Chez vous (même Wi-Fi) », rien d'autre à faire.</li>
          <li><strong>Ailleurs (4G, chez eux)</strong> : leur téléphone doit faire partie de votre réseau privé Tailscale. Dans Tailscale (admin → Machines → votre PC → « Share… »), invitez leur adresse e-mail ; ils installent l'application Tailscale, acceptent le partage, puis le QR code « Partout (Tailscale) » fonctionne. Le libellé exact peut varier selon les versions.</li>
          <li>Le lien contient leur clé : traitez-le comme un mot de passe. Il expire selon la durée choisie ou dès que vous révoquez.</li>
        </ul>
      </div>
    </div>
  );
}

function UrlCard({ label, url, primary, openQr, notify }: { label: string; url: string; primary?: boolean; openQr?: boolean; notify: (m: string) => void }) {
  const [qr, setQr] = useState<string | null>(null);
  const [show, setShow] = useState(false);
  const [qrShown, setQrShown] = useState(!!openQr); // caché par défaut : un QR code contient la clé et se scanne de loin
  useEffect(() => { QRCode.toDataURL(url, { margin: 1, width: 200 }).then(setQr).catch(() => setQr(null)); }, [url]);
  return (
    <div className={`card grid gap-4 md:grid-cols-[auto_1fr] ${primary ? "border-pal-600" : ""}`}>
      {qrShown
        ? (qr ? <img src={qr} alt={`QR code — ${label}`} width={200} height={200} className="rounded-lg bg-white" /> : <div className="h-[200px] w-[200px] rounded-lg bg-slate-800" />)
        : <button type="button" className="flex h-[200px] w-[200px] items-center justify-center rounded-lg bg-slate-800 text-sm text-slate-300 hover:bg-slate-700" onClick={() => setQrShown(true)}>Afficher le QR code</button>}
      <div className="space-y-2 text-sm">
        <h3 className="font-semibold">{label}</h3>
        <div className="flex gap-2">
          <input className="input font-mono text-xs" readOnly aria-label={label} value={show ? url : mask(url)} />
          <button className="btn" onClick={() => setShow(!show)}>{show ? "Masquer" : "Afficher"}</button>
          <button className="btn" onClick={() => navigator.clipboard.writeText(url).then(() => notify("Adresse copiée")).catch(() => notify("Copie impossible"))}>Copier</button>
        </div>
        <p className="text-xs text-slate-500">Le QR code contient la clé secrète : ne l'affichez que pour la personne concernée.{qrShown && <button type="button" className="ml-2 underline" onClick={() => setQrShown(false)}>Le cacher</button>}</p>
      </div>
    </div>
  );
}

export default function Mobile({ notify }: { notify: (m: string) => void }) {
  const [s, setS] = useState<AppSettings | null>(null);
  const [info, setInfo] = useState<RemoteInfo | null>(null);
  const [port, setPort] = useState(8765);

  const load = useCallback(async () => {
    try {
      const [st, i] = await Promise.all([api.getSettings(), api.remoteInfo()]);
      setS(st); setInfo(i); setPort(st.remote.port);
    } catch (e) { notify(String(e)); }
  }, [notify]);
  useEffect(() => { load(); }, [load]);
  if (!s || !info) return <p className="text-slate-400">Chargement…</p>;

  const apply = (remote: Partial<AppSettings["remote"]>) =>
    api.saveSettings({ ...s, remote: { ...s.remote, ...remote } }).then(load).catch((e) => notify(String(e)));

  return (
    <div className="space-y-4">
      <div className="card space-y-3">
        <div className="flex items-center gap-3">
          <h2 className="font-semibold">Contrôler le serveur depuis votre téléphone</h2>
          <label className="ml-auto flex items-center gap-2 text-sm">
            <input type="checkbox" checked={s.remote.enabled} onChange={(e) => apply({ enabled: e.target.checked })} /> Activé
          </label>
        </div>
        <p className="text-sm text-slate-400">
          L'application sert une page web adaptée au mobile : état, joueurs, journal, graphiques, et boutons démarrer / redémarrer / arrêter, sauvegarde, annonce, expulsion.
          Les réglages, la configuration du monde, la restauration et le bannissement ne sont volontairement pas accessibles à distance.
        </p>
        {s.remote.enabled && (
          <p className={`text-sm ${info.running ? "text-emerald-400" : "text-red-400"}`} role="status">
            {info.running ? "● En écoute" : `● Arrêté${info.error ? ` — ${info.error}` : ""}`}
          </p>
        )}
      </div>

      {s.remote.enabled && info.running && (
        <>
          {!info.tailscale_found && (
            <div className="rounded-lg border border-amber-500/50 bg-amber-500/10 p-3 text-sm text-amber-200" role="status">
              Tailscale n'est pas détecté sur ce PC : l'accès en 4G n'est pas encore possible. Suivez les étapes ci-dessous, puis revenez ici (le bouton « Actualiser » en bas).
            </div>
          )}
          {info.urls.map((u, i) => <UrlCard key={u.label} label={u.label} url={u.url} primary={i === 0} notify={notify} />)}
        </>
      )}

      {s.remote.enabled && info.running && <GuestEditor s={s} info={info} reload={load} notify={notify} />}

      <div className="card space-y-3 text-sm">
        <h3 className="font-semibold">Accès partout (4G, autre Wi-Fi) avec Tailscale — étapes sur Android</h3>
        <ol className="list-decimal space-y-2 pl-5 text-slate-300">
          <li>Sur <strong>ce PC</strong> : installez <a className="underline" href="https://tailscale.com/download" target="_blank" rel="noreferrer">Tailscale</a>, ouvrez-le et connectez-vous (compte Google, Microsoft ou Apple, gratuit).</li>
          <li>Sur le <strong>téléphone Android</strong> : installez « Tailscale » depuis le Play Store, connectez-vous avec <strong>le même compte</strong> et activez le bouton de connexion (une icône de clé apparaît en haut).</li>
          <li>Ici, cochez « Activé » : une carte « Partout (Tailscale) » apparaît avec un QR code. Scannez-le avec l'appareil photo du téléphone, ou copiez l'adresse.</li>
          <li>La page s'ouvre dans Chrome et se connecte toute seule. Menu ⋮ → « Ajouter à l'écran d'accueil » pour l'avoir comme une application.</li>
        </ol>
        <p className="text-xs text-slate-500">Le PC doit être allumé, Tailscale connecté et cette application ouverte (elle peut rester réduite). Tout le trafic passe par le réseau privé chiffré de Tailscale : aucune redirection de port sur la box, et le serveur mobile refuse de toute façon les adresses publiques d'Internet.</p>
        <div className="flex flex-wrap items-center gap-2">
          <button className="btn" onClick={load}>Actualiser</button>
          <label className="ml-auto flex items-center gap-2 text-slate-400">Port
            <input className="input !w-24" type="number" min={1024} max={65535} value={port} onChange={(e) => setPort(+e.target.value)} />
          </label>
          <button className="btn" disabled={port === s.remote.port} onClick={() => apply({ port })}>Appliquer</button>
          <button className="btn" onClick={() => confirm("Régénérer la clé ? Les téléphones déjà connectés devront se reconnecter avec le nouveau QR code.") && api.regenerateToken().then(() => { notify("Nouvelle clé générée"); load(); }).catch((e) => notify(String(e)))}>Régénérer la clé</button>
        </div>
      </div>

      <div className="card space-y-1 text-sm text-slate-400">
        <h3 className="font-semibold text-slate-200">Sécurité</h3>
        <ul className="list-disc space-y-1 pl-5">
          <li>Chaque requête exige la clé secrète (128 bits, générée au hasard). Sans elle, seule une page de connexion vide est servie.</li>
          <li>Seuls le réseau local et Tailscale peuvent atteindre le serveur mobile ; une adresse publique est refusée même avec la bonne clé.</li>
          <li>Au premier lancement, Windows demande d'autoriser l'application dans le pare-feu : cochez « Réseaux privés ». Ne redirigez <strong>jamais</strong> ce port sur votre box.</li>
          <li>Si vous perdez le téléphone : « Régénérer la clé » coupe immédiatement son accès.</li>
        </ul>
      </div>
    </div>
  );
}
