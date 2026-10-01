import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AppSettings, ModsState } from "../lib/types";

const workshopUrl = (id: string) => `https://steamcommunity.com/sharedfiles/filedetails/?id=${id}`;

export default function Mods({ notify }: { notify: (m: string) => void }) {
  const [st, setSt] = useState<ModsState | null>(null);
  const [input, setInput] = useState("");
  const [rootInput, setRootInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [changed, setChanged] = useState(false); // un changement attend un redémarrage du serveur
  const [allowClient, setAllowClient] = useState<boolean | null>(null);
  const [packs, setPacks] = useState<AppSettings["mod_automation"]["packs"]>([]);
  const [packName, setPackName] = useState("");
  const loadPacks = useCallback(() => { api.getSettings().then((s) => setPacks(s.mod_automation.packs)).catch(() => {}); }, []);
  useEffect(() => { loadPacks(); }, [loadPacks]);

  const load = useCallback(() => {
    api.modsState().then((s) => { setSt(s); setRootInput(s.workshop_root ?? ""); }).catch((e) => notify(String(e)));
  }, [notify]);
  useEffect(() => { load(); }, [load]);
  useEffect(() => {
    api.readWorld().then(({ options }) => setAllowClient(options.find((o) => o.key === "bAllowClientMod")?.value !== "False")).catch(() => {});
  }, []);

  if (!st) return <p className="text-slate-400">Chargement…</p>;

  const run = async (f: () => Promise<unknown>, ok?: string, needsRestart = true) => {
    setBusy(true);
    try { await f(); if (ok) notify(ok); if (needsRestart) setChanged(true); load(); }
    catch (e) { notify(String(e)); } finally { setBusy(false); }
  };

  const setAllowClientMod = async (on: boolean) => {
    try {
      const { options } = await api.readWorld();
      const v = on ? "True" : "False";
      await api.writeWorld(options.some((o) => o.key === "bAllowClientMod") ? options.map((o) => (o.key === "bAllowClientMod" ? { ...o, value: v } : o)) : [...options, { key: "bAllowClientMod", value: v, quoted: false }]);
      setAllowClient(on); setChanged(true); notify("Enregistré dans PalWorldSettings.ini");
    } catch (e) { notify(String(e)); }
  };

  return (
    <div className="space-y-4">
      <div className="card space-y-3">
        <h2 className="font-semibold">Packs de mods</h2>
        <p className="text-sm text-slate-400">Enregistrez l'ensemble des mods activés sous un nom (« Vanilla », « Avec mods »…) et basculez en un clic. Un pack n'active que les mods présents et compatibles serveur ; le serveur doit être redémarré. Astuce : un pack « Vanilla » vide s'enregistre en désactivant d'abord tous les mods.</p>
        <ul className="divide-y divide-slate-800 text-sm">
          {packs.map((p) => (
            <li key={p.name} className="flex flex-wrap items-center gap-3 py-2">
              <div><strong>{p.name}</strong><div className="text-xs text-slate-500">{p.package_names.length ? p.package_names.join(", ") : "aucun mod"}</div></div>
              <button className="btn-primary ml-auto" disabled={busy} onClick={() => confirm(`Activer le pack « ${p.name} » ?`) && run(() => api.modPackApply(p.name).then((missing) => { if (missing.length) notify(`Mods introuvables ou incompatibles : ${missing.join(", ")}`); }), `Pack « ${p.name} » appliqué`)}>Appliquer</button>
              <button className="btn" disabled={busy} onClick={() => run(() => api.modPackSave(p.name).then(loadPacks), "Pack mis à jour avec les mods actuels", false)}>Remplacer par l'actuel</button>
              <button className="btn-danger" disabled={busy} onClick={() => confirm(`Supprimer le pack « ${p.name} » ?`) && run(() => api.modPackDelete(p.name).then(loadPacks), undefined, false)}>Supprimer</button>
            </li>
          ))}
          {packs.length === 0 && <li className="py-2 text-slate-400">Aucun pack.</li>}
        </ul>
        <div className="flex gap-2">
          <input className="input max-w-xs" placeholder="Nom du pack" value={packName} onChange={(e) => setPackName(e.target.value)} />
          <button className="btn" disabled={busy || !packName.trim()} onClick={() => run(() => api.modPackSave(packName).then(() => { setPackName(""); loadPacks(); }), "Pack enregistré", false)}>Enregistrer les mods actifs sous ce nom</button>
        </div>
      </div>
      {changed && (
        <div className="flex flex-wrap items-center gap-3 rounded-lg border border-amber-500/50 bg-amber-500/10 p-3 text-sm text-amber-200" role="status">
          Les mods sont lus au démarrage du serveur : redémarrez-le pour appliquer les changements.
          <button className="btn ml-auto" onClick={() => confirm("Redémarrer le serveur maintenant ? Les joueurs seront déconnectés.") && api.restart().then(() => { notify("Redémarrage demandé"); setChanged(false); }).catch((e) => notify(String(e)))}>Redémarrer maintenant</button>
        </div>
      )}

      <div className="card space-y-3">
        <div className="flex items-center gap-3">
          <h2 className="font-semibold">Mods du serveur</h2>
          <label className="ml-auto flex items-center gap-2 text-sm">
            <input type="checkbox" checked={st.global_enable} disabled={busy} onChange={(e) => run(() => api.modsSetGlobal(e.target.checked))} /> Mods activés
          </label>
        </div>
        <p className="text-sm text-slate-400">
          Palworld 1.0 charge les mods du <strong>Steam Workshop</strong>. L'application gère pour vous le fichier <code className="text-xs">Mods\PalModSettings.ini</code> : vous choisissez les mods actifs, elle écrit la liste (une copie <code>.bak</code> est faite à chaque changement).
        </p>
        <p className="text-xs text-amber-300">⚠ Un mod est du code tiers exécuté sur votre serveur : n'installez que des mods dont vous avez lu la page Workshop. Seuls les mods marqués « compatible serveur » fonctionnent sur un serveur dédié.</p>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={allowClient !== false} disabled={allowClient === null} onChange={(e) => setAllowClientMod(e.target.checked)} />
          Autoriser les joueurs qui ont des mods côté client <code className="text-xs text-slate-500">bAllowClientMod</code>
        </label>
      </div>

      <div className="card space-y-3">
        <h3 className="font-semibold">Ajouter un mod</h3>
        <div className="flex flex-wrap gap-2">
          <input className="input flex-1" placeholder="Identifiant ou adresse du mod Workshop (ex. https://steamcommunity.com/sharedfiles/filedetails/?id=…)" value={input} onChange={(e) => setInput(e.target.value)} />
          <button className="btn-primary" disabled={busy || !input.trim()} onClick={() => run(() => api.modsAdd(input).then((m) => { notify(m); setInput(""); }), undefined, false)}>
            {busy ? "Téléchargement…" : "Télécharger"}
          </button>
        </div>
        <p className="text-xs text-slate-500">
          L'application télécharge le mod avec SteamCMD (connexion anonyme) dans <code>{st.download_root}</code>. Si Steam refuse (certains mods exigent un compte propriétaire du jeu) : abonnez-vous au mod depuis le <strong>client Steam</strong> de ce PC, puis choisissez ci-dessous le dossier Workshop de Steam.
        </p>
      </div>

      <div className="card space-y-3">
        <h3 className="font-semibold">Dossier Workshop lu par le serveur</h3>
        <div className="flex flex-wrap gap-2">
          <input className="input flex-1 font-mono text-xs" value={rootInput} onChange={(e) => setRootInput(e.target.value)} placeholder="C:\Program Files (x86)\Steam\steamapps\workshop\content\1623730" />
          <button className="btn" disabled={busy || !rootInput.trim() || rootInput === st.workshop_root} onClick={() => run(() => api.modsSetRoot(rootInput), "Dossier Workshop enregistré")}>Appliquer</button>
        </div>
        {st.candidates.length > 0 && (
          <div className="flex flex-wrap items-center gap-2 text-xs text-slate-400">
            Trouvés sur ce PC :
            {st.candidates.map((c) => <button key={c} className="btn !py-0.5 text-xs" onClick={() => setRootInput(c)}>{c}</button>)}
          </div>
        )}
        {!st.root_exists && <p className="text-sm text-amber-300">Ce dossier n'existe pas encore : téléchargez un mod ci-dessus, ou abonnez-vous à un mod dans Steam.</p>}
      </div>

      <div className="card">
        <h3 className="mb-3 font-semibold">Mods trouvés ({st.mods.length})</h3>
        {st.mods.length === 0 ? <p className="text-sm text-slate-400">Aucun mod détecté dans ce dossier.</p> : (
          <ul className="divide-y divide-slate-800 text-sm">
            {st.mods.map((m) => (
              <li key={m.workshop_id} className="flex flex-wrap items-center gap-3 py-3">
                <label className="flex items-center gap-3">
                  <input type="checkbox" checked={m.active} disabled={busy} aria-label={`Activer ${m.name ?? m.package_name}`} onChange={(e) => {
                    if (e.target.checked && !m.server_compatible && !confirm("Ce mod ne déclare aucune règle pour serveur dédié : il sera probablement sans effet, et peut parfois poser problème. L'activer quand même ?")) return;
                    run(() => api.modsSetActive(m.package_name, e.target.checked));
                  }} />
                  <span>
                    <span className="font-medium">{m.name ?? m.package_name}</span>
                    {m.version && <span className="ml-2 text-xs text-slate-500">v{m.version}</span>}
                    <span className="block text-xs text-slate-500">{m.package_name}{m.author ? ` · ${m.author}` : ""} · n° {m.workshop_id}</span>
                  </span>
                </label>
                <span className={`rounded px-1.5 py-0.5 text-xs ${m.server_compatible ? "bg-emerald-500/20 text-emerald-300" : "bg-amber-500/20 text-amber-300"}`}>
                  {m.server_compatible ? "compatible serveur" : "client seulement ⚠"}
                </span>
                <a className="ml-auto text-xs underline" href={workshopUrl(m.workshop_id)} target="_blank" rel="noreferrer">Page Workshop</a>
                {m.removable && <button className="btn" disabled={busy} onClick={() => confirm(`Supprimer « ${m.name ?? m.package_name} » du dossier du serveur ?`) && run(() => api.modsRemove(m.workshop_id), "Mod supprimé")}>Supprimer</button>}
              </li>
            ))}
          </ul>
        )}
        <p className="mt-3 text-xs text-slate-500">Fichier géré : <code>{st.settings_path}</code>. Mises à jour : un mod téléchargé par l'application se met à jour en le retéléchargeant ; ceux du client Steam se mettent à jour par Steam.</p>
      </div>
    </div>
  );
}
