import { useEffect, useRef, useState } from "react";
import { api } from "../lib/api";

const MAX_LINES = 2000;

export default function Logs({ notify }: { notify: (m: string) => void }) {
  const [lines, setLines] = useState<string[]>([]);
  const [filter, setFilter] = useState("");
  const [follow, setFollow] = useState(true);
  const offset = useRef<number | null>(null);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let dead = false;
    const poll = async () => {
      try {
        const c = await api.logs(offset.current);
        if (dead) return;
        // Offset plus petit que le précédent = fichier tronqué/rotaté : on repart de zéro côté affichage.
        const rotated = offset.current !== null && c.offset < offset.current;
        offset.current = c.offset;
        if (rotated) setLines(c.lines);
        else if (c.lines.length) setLines((cur) => [...cur, ...c.lines].slice(-MAX_LINES));
      } catch (e) { notify(String(e)); }
    };
    poll();
    const id = setInterval(poll, 2000);
    return () => { dead = true; clearInterval(id); };
  }, [notify]);

  useEffect(() => { if (follow && box.current) box.current.scrollTop = box.current.scrollHeight; }, [lines, follow]);

  const q = filter.trim().toLowerCase();
  const shown = q ? lines.filter((l) => l.toLowerCase().includes(q)) : lines;
  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex items-center gap-3">
        <input className="input max-w-xs" placeholder="Filtrer (ex. error, joined)" value={filter} onChange={(e) => setFilter(e.target.value)} />
        <label className="text-sm"><input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} /> Suivre</label>
        <span className="text-xs text-slate-500">{shown.length} / {lines.length} lignes · Pal/Saved/Logs/Pal.log</span>
      </div>
      <div ref={box} className="card min-h-0 flex-1 overflow-y-auto font-mono text-xs leading-5" aria-live="off">
        {shown.length === 0 ? <p className="text-slate-500">Aucune ligne (le serveur n'a pas encore écrit de journal).</p>
          : shown.map((l, i) => <div key={i} className={/error|fatal|crash/i.test(l) ? "text-red-400" : /warn/i.test(l) ? "text-amber-300" : ""}>{l}</div>)}
      </div>
    </div>
  );
}
