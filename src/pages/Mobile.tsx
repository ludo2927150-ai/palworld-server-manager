import { useCallback, useEffect, useState } from "react";
import QRCode from "qrcode";
import { api } from "../lib/api";
import type { AppSettings, RemoteInfo } from "../lib/types";

const mask = (url: string) => url.replace(/#token=.*/, "#token=••••••••");

function UrlCard({ label, url, primary, notify }: { label: string; url: string; primary?: boolean; notify: (m: string) => void }) {
  const [qr, setQr] = useState<string | null>(null);
  const [show, setShow] = useState(false);
  useEffect(() => { QRCode.toDataURL(url, { margin: 1, width: 200 }).then(setQr).catch(() => setQr(null)); }, [url]);
  return (
    <div className={`card grid gap-4 md:grid-cols-[auto_1fr] ${primary ? "border-pal-600" : ""}`}>
      {qr ? <img src={qr} alt={`QR code — ${label}`} width={200} height={200} className="rounded-lg bg-white" /> : <div className="h-[200px] w-[200px] rounded-lg bg-slate-800" />}
      <div className="space-y-2 text-sm">
        <h3 className="font-semibold">{label}</h3>
        <div className="flex gap-2">
          <input className="input font-mono text-xs" readOnly aria-label={label} value={show ? url : mask(url)} />
          <button className="btn" onClick={() => setShow(!show)}>{show ? "Masquer" : "Afficher"}</button>
          <button className="btn" onClick={() => navigator.clipboard.writeText(url).then(() => notify("Adresse copiée")).catch(() => notify("Copie impossible"))}>Copier</button>
        </div>
        <p className="text-xs text-slate-500">Le QR code contient la clé secrète : ne le montrez qu'à vous-même.</p>
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
