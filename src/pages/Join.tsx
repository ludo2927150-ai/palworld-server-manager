import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "../lib/api";
import type { Opt } from "../lib/types";

/** Copie dans le presse-papiers ; repli sur une zone de texte cachée si l'API moderne est refusée. */
async function copy(text: string): Promise<boolean> {
  try { await navigator.clipboard.writeText(text); return true; } catch { /* repli ci-dessous */ }
  const ta = document.createElement("textarea");
  ta.value = text; ta.style.position = "fixed"; ta.style.opacity = "0";
  document.body.appendChild(ta); ta.select();
  const ok = document.execCommand("copy");
  document.body.removeChild(ta);
  return ok;
}

function CopyRow({ label, value, secret, notify }: { label: string; value: string; secret?: boolean; notify: (m: string) => void }) {
  const [show, setShow] = useState(!secret);
  return (
    <div>
      <div className="mb-1 text-xs uppercase tracking-wide text-slate-400">{label}</div>
      <div className="flex gap-2">
        <input className="input font-mono" readOnly aria-label={label} value={show ? value : "••••••••"} />
        {secret && <button className="btn" onClick={() => setShow(!show)}>{show ? "Masquer" : "Afficher"}</button>}
        <button className="btn" onClick={() => copy(value).then((ok) => notify(ok ? `${label} copié` : "Copie impossible"))}>Copier</button>
      </div>
    </div>
  );
}

export default function Join({ notify }: { notify: (m: string) => void }) {
  const [opts, setOpts] = useState<Opt[] | null>(null);
  const [lan, setLan] = useState<string | null>(null);
  const [pub, setPub] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const [w, n] = await Promise.all([api.readWorld(), api.networkInfo()]);
      setOpts(w.options); setLan(n.lan_ip);
    } catch (e) { notify(String(e)); }
  }, [notify]);
  useEffect(() => { load(); }, [load]);

  const get = (k: string) => opts?.find((o) => o.key === k)?.value ?? "";
  const name = get("ServerName") || "Serveur Palworld";
  const port = get("PublicPort") || "8211";
  const password = get("ServerPassword");
  const maxPlayers = get("ServerPlayerMaxNum");
  const lanAddr = lan ? `${lan}:${port}` : null;
  const pubAddr = pub ? `${pub}:${port}` : null;

  const detectPublic = async () => {
    setBusy(true);
    try { setPub(await api.publicIp()); } catch (e) { notify(`IP publique introuvable : ${e}`); } finally { setBusy(false); }
  };

  const message = useMemo(() => {
    const lines = [`Rejoins mon serveur Palworld « ${name} » !`];
    if (pubAddr) lines.push(`Adresse (depuis Internet) : ${pubAddr}`);
    if (lanAddr) lines.push(`Adresse (même réseau local) : ${lanAddr}`);
    lines.push(password ? `Mot de passe : ${password}` : "Pas de mot de passe.");
    lines.push("Dans le jeu : Multijoueur → Rejoindre un serveur → saisis l'adresse en bas de la liste.");
    return lines.join("\n");
  }, [name, pubAddr, lanAddr, password]);

  if (!opts) return <p className="text-slate-400">Chargement…</p>;

  return (
    <div className="space-y-4">
      <div className="card space-y-4">
        <div>
          <h2 className="text-lg font-semibold">{name}</h2>
          <p className="text-sm text-slate-400">{maxPlayers ? `${maxPlayers} joueurs maximum · ` : ""}port du jeu {port} (UDP)</p>
        </div>
        <div className="grid gap-4 md:grid-cols-2">
          {lanAddr
            ? <CopyRow label="Adresse — même réseau local (même box/Wi-Fi)" value={lanAddr} notify={notify} />
            : <p className="text-sm text-amber-300">Adresse locale introuvable : le PC est-il connecté à un réseau ?</p>}
          {pubAddr
            ? <CopyRow label="Adresse — depuis Internet" value={pubAddr} notify={notify} />
            : (
              <div>
                <div className="mb-1 text-xs uppercase tracking-wide text-slate-400">Adresse — depuis Internet</div>
                <button className="btn" disabled={busy} onClick={detectPublic}>{busy ? "Recherche…" : "Détecter mon IP publique"}</button>
                <p className="mt-1 text-xs text-slate-500">Interroge le service api.ipify.org (un appel, uniquement quand vous cliquez).</p>
              </div>
            )}
          {password ? <CopyRow label="Mot de passe du serveur" value={password} secret notify={notify} /> : (
            <p className="text-sm text-amber-300">Aucun mot de passe : toute personne connaissant l'adresse peut entrer. Définissez <code>ServerPassword</code> dans l'onglet Configuration.</p>
          )}
        </div>
      </div>

      <div className="card space-y-2">
        <div className="flex items-center">
          <h3 className="font-semibold">Message à envoyer à vos amis</h3>
          <button className="btn ml-auto" onClick={() => copy(message).then((ok) => notify(ok ? "Message copié" : "Copie impossible"))}>Copier le message</button>
        </div>
        <pre className="whitespace-pre-wrap rounded-lg bg-slate-950 p-3 text-sm" aria-label="Message à partager">{message}</pre>
      </div>

      <div className="card space-y-2 text-sm">
        <h3 className="font-semibold">Où saisir ces informations dans le jeu</h3>
        <ol className="list-decimal space-y-1 pl-5 text-slate-300">
          <li>Lancez Palworld et choisissez <strong>Multijoueur / Rejoindre un serveur</strong> dans le menu principal.</li>
          <li>Tout en bas de la liste des serveurs, il y a un champ pour saisir une adresse : tapez <code>adresse:port</code> (copiée ci-dessus).</li>
          <li>Si un mot de passe est demandé, saisissez celui du serveur, puis validez.</li>
        </ol>
        <p className="text-xs text-slate-500">Le libellé exact des menus peut varier selon la version du jeu.</p>
      </div>

      <div className="card space-y-1 text-sm text-slate-400">
        <h3 className="font-semibold text-slate-200">Pour que vos amis entrent depuis Internet</h3>
        <ul className="list-disc space-y-1 pl-5">
          <li>Sur votre box, redirigez le port <strong>{port} en UDP</strong> vers l'adresse locale de ce PC ({lan ?? "—"}).</li>
          <li>Autorisez le serveur dans le pare-feu Windows (réseau privé et public).</li>
          <li>Ne redirigez <strong>jamais</strong> le port de l'API REST (8212) vers Internet : il sert uniquement à cette application.</li>
          <li>Votre IP publique peut changer ; pensez à renvoyer le message si besoin.</li>
        </ul>
      </div>
    </div>
  );
}
