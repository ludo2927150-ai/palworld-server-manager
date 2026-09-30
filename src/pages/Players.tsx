import { api } from "../lib/api";
import type { Snapshot } from "../lib/types";

export default function Players({ snap, notify }: { snap: Snapshot | null; notify: (m: string) => void }) {
  const players = snap?.players ?? [];
  return (
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
                  <button className="btn" onClick={() => api.kick(p.userId).catch((e) => notify(String(e)))}>Expulser</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
