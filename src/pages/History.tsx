import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Sample, Session } from "../lib/types";
import LineChart from "../components/LineChart";

const fmt = (t: number) => new Date(t * 1000).toLocaleString("fr-FR", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
const dur = (s: number) => (s >= 3600 ? `${Math.floor(s / 3600)} h ${Math.floor((s % 3600) / 60)} min` : `${Math.max(1, Math.floor(s / 60))} min`);

export default function History({ notify }: { notify: (m: string) => void }) {
  const [hours, setHours] = useState(24);
  const [data, setData] = useState<Sample[]>([]);
  const [sessions, setSessions] = useState<Session[]>([]);

  useEffect(() => {
    const load = () => {
      api.history(hours).then(setData).catch((e) => notify(String(e)));
      api.sessions(7).then(setSessions).catch((e) => notify(String(e)));
    };
    load();
    const id = setInterval(load, 30_000);
    return () => clearInterval(id);
  }, [hours, notify]);

  const now = Math.floor(Date.now() / 1000);
  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        {[1, 6, 24, 168].map((h) => <button key={h} className={h === hours ? "btn-primary" : "btn"} onClick={() => setHours(h)}>{h === 168 ? "7 j" : `${h} h`}</button>)}
      </div>
      <div className="grid gap-4 lg:grid-cols-2">
        <LineChart title="CPU" unit="%" max={100} data={data} pick={(s) => s.cpu} />
        <LineChart title="Mémoire" unit="%" max={100} data={data} pick={(s) => s.mem_percent} />
        <LineChart title="Joueurs connectés" unit="joueurs" data={data} pick={(s) => s.players} />
        <LineChart title="FPS serveur" unit="FPS" data={data} pick={(s) => s.fps} />
      </div>
      <div className="card">
        <h2 className="mb-2 font-semibold">Sessions de jeu (7 derniers jours)</h2>
        {sessions.length === 0 ? <p className="text-sm text-slate-400">Aucune session enregistrée.</p> : (
          <table className="w-full text-left text-sm">
            <thead className="text-slate-400"><tr><th>Joueur</th><th>Arrivée</th><th>Durée</th></tr></thead>
            <tbody>{sessions.map((s, i) => (
              <tr key={i} className="border-t border-slate-800">
                <td className="py-1.5">{s.name}</td><td>{fmt(s.start)}</td>
                <td>{s.end ? dur(s.end - s.start) : <span className="text-emerald-400">en ligne · {dur(now - s.start)}</span>}</td>
              </tr>))}</tbody>
          </table>
        )}
      </div>
    </div>
  );
}
