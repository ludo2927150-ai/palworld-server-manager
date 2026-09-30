import { useState } from "react";
import { api } from "../lib/api";
import type { Snapshot } from "../lib/types";

export default function Players({ snap, notify }: { snap: Snapshot | null; notify: (m: string) => void }) {
  const players = snap?.players ?? [];
  const [msg, setMsg] = useState("");
  const [unbanId, setUnbanId] = useState("");
  const run = (f: () => Promise<unknown>, ok: string) => f().then(() => notify(ok)).catch((e) => notify(String(e)));
  return (
    <div className="space-y-4">
    <div className="card">
      <h2 className="mb-3 font-semibold">Joueurs connectés ({players.length})</h2>
      {players.length === 0 ? <p className="text-sm text-slate-400">Personne en ligne.</p> : (
        <table className="w-full text-left text-sm">
          <thead className="text-slate-400"><tr><th>Nom</th><th>Niveau</th><th>Ping</th><th /></tr></thead>
          <tbody>
            {players.map((p) => (
              <tr key={p.userId} className="border-t border-slate-800">
                <td className="py-2">{p.name}</td><td>{p.level}</td><td>{Math.round(p.ping)} ms</td>
                <td className="text-right">
                  <button className="btn mr-2" onClick={() => run(() => api.kick(p.userId), `${p.name} expulsé`)}>Expulser</button>
                  <button className="btn-danger" onClick={() => confirm(`Bannir ${p.name} ?`) && run(() => api.ban(p.userId), `${p.name} banni`)}>Bannir</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
    <div className="card grid gap-4 md:grid-cols-2">
      <div className="flex gap-2">
        <input className="input" placeholder="Annonce à tous les joueurs" value={msg} onChange={(e) => setMsg(e.target.value)} />
        <button className="btn-primary" disabled={!msg.trim()} onClick={() => run(() => api.announce(msg), "Annonce envoyée").then(() => setMsg(""))}>Envoyer</button>
      </div>
      <div className="flex gap-2">
        <input className="input" placeholder="ID à débannir (ex. steam_7656…)" value={unbanId} onChange={(e) => setUnbanId(e.target.value)} />
        <button className="btn" disabled={!unbanId.trim()} onClick={() => run(() => api.unban(unbanId.trim()), "Joueur débanni").then(() => setUnbanId(""))}>Débannir</button>
      </div>
    </div>
    </div>
  );
}
