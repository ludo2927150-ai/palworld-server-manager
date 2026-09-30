import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";
import type { BackupInfo } from "../lib/types";

export default function Backups({ notify }: { notify: (m: string) => void }) {
  const [list, setList] = useState<BackupInfo[]>([]);
  const refresh = useCallback(() => api.listBackups().then(setList).catch((e) => notify(String(e))), [notify]);
  useEffect(() => { refresh(); }, [refresh]);

  return (
    <div className="card">
      <div className="mb-3 flex items-center">
        <h2 className="font-semibold">Sauvegardes</h2>
        <button className="btn-primary ml-auto" onClick={() => api.backupNow().then(refresh).catch((e) => notify(String(e)))}>Sauvegarder maintenant</button>
      </div>
      <ul className="divide-y divide-slate-800 text-sm">
        {list.map((b) => (
          <li key={b.path} className="flex items-center gap-3 py-2">
            <span>{b.file_name}</span>
            <span className="text-slate-500">{(b.size_bytes / 1e6).toFixed(1)} Mo · {new Date(b.created).toLocaleString("fr-FR")}</span>
            <button className="btn ml-auto" onClick={() => confirm("Restaurer ? L'état actuel sera déplacé en SaveGames.bak.") && api.restoreBackup(b.path).then(() => notify("Restauré")).catch((e) => notify(String(e)))}>Restaurer</button>
          </li>
        ))}
        {list.length === 0 && <li className="py-2 text-slate-400">Aucune sauvegarde.</li>}
      </ul>
    </div>
  );
}
