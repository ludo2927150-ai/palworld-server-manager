import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { ProfileInfo } from "../lib/types";

export default function Profiles({ notify, dirty, onApplied }: { notify: (m: string) => void; dirty: boolean; onApplied: () => void }) {
  const [list, setList] = useState<ProfileInfo[]>([]);
  const [name, setName] = useState("");
  const refresh = useCallback(() => api.profiles().then(setList).catch(() => {}), []);
  useEffect(() => { refresh(); }, [refresh]);

  return (
    <div className="card space-y-3">
      <h2 className="font-semibold">Profils de configuration</h2>
      <p className="text-sm text-slate-400">Enregistrez les réglages actuels du monde sous un nom (« Normal », « Hardcore », « Soirée »…), puis rebasculez en un clic. Les mots de passe, les ports et l'API REST ne sont jamais modifiés par un profil. Le serveur doit être redémarré pour appliquer.</p>
      {dirty && <p className="text-sm text-amber-300" role="status">Vous avez des changements non enregistrés : « Enregistrer sous ce nom » enregistre le fichier tel qu'il est sur le disque, pas ces changements. Enregistrez d'abord la configuration.</p>}
      <ul className="divide-y divide-slate-800 text-sm">
        {list.map((p) => (
          <li key={p.name} className="flex flex-wrap items-center gap-3 py-2">
            <div><strong>{p.name}</strong><div className="text-xs text-slate-500">{p.options} options · {new Date(p.saved_at * 1000).toLocaleString("fr-FR")}</div></div>
            <button className="btn-primary ml-auto" onClick={() => confirm(`Appliquer le profil « ${p.name} » ? Une copie .bak de la configuration actuelle est conservée ; redémarrez ensuite le serveur.`) &&
              api.profileApply(p.name).then((n) => { notify(`Profil appliqué (${n} valeurs modifiées) — redémarrage du serveur requis`); onApplied(); }).catch((e) => notify(String(e)))}>Appliquer</button>
            <button className="btn" onClick={() => api.profileSave(p.name).then(() => { notify("Profil mis à jour"); refresh(); }).catch((e) => notify(String(e)))}>Remplacer par l'actuel</button>
            <button className="btn-danger" onClick={() => confirm(`Supprimer le profil « ${p.name} » ?`) && api.profileDelete(p.name).then(refresh).catch((e) => notify(String(e)))}>Supprimer</button>
          </li>
        ))}
        {list.length === 0 && <li className="py-2 text-slate-400">Aucun profil enregistré.</li>}
      </ul>
      <div className="flex gap-2">
        <input className="input max-w-xs" placeholder="Nom du profil" value={name} onChange={(e) => setName(e.target.value)} />
        <button className="btn" disabled={!name.trim()} onClick={() => api.profileSave(name).then(() => { notify("Profil enregistré"); setName(""); refresh(); }).catch((e) => notify(String(e)))}>Enregistrer sous ce nom</button>
      </div>
    </div>
  );
}
