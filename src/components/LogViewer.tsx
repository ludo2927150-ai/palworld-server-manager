import { useEffect, useRef, useState } from "react";
import { api } from "../lib/api";

interface Props {
  notify: (m: string) => void;
  /** Mode compact (tableau de bord) : moins de lignes gardées, hauteur fixe. */
  compact?: boolean;
}

/** Suit `Pal.log` en direct (sondage toutes les 2 s) avec filtre et défilement automatique. */
export default function LogViewer({ notify, compact = false }: Props) {
  const maxLines = compact ? 300 : 2000;
  const [lines, setLines] = useState<string[]>([]);
  const [filter, setFilter] = useState("");
  const [follow, setFollow] = useState(true);
  const offset = useRef<number | null>(null);
  const source = useRef<string>("");
  const [info, setInfo] = useState<{ source: string; hint: string | null }>({ source: "", hint: null });
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let dead = false;
    const poll = async () => {
      try {
        const c = await api.logs(offset.current);
        if (dead) return;
        setInfo((cur) => (cur.source === c.source && cur.hint === c.hint ? cur : { source: c.source, hint: c.hint }));
        // Un autre fichier est devenu le plus récent (rotation, nouveau démarrage) : on repart de sa fin.
        if (offset.current !== null && c.source !== source.current) { source.current = c.source; offset.current = null; setLines([]); return poll(); }
        source.current = c.source;
        // Offset plus petit que le précédent = fichier tronqué/rotaté : on repart de zéro côté affichage.
        const rotated = offset.current !== null && c.offset < offset.current;
        offset.current = c.offset;
        if (rotated) setLines(c.lines);
        else if (c.lines.length) setLines((cur) => [...cur, ...c.lines].slice(-maxLines));
      } catch (e) { notify(String(e)); }
    };
    poll();
    const id = setInterval(poll, 2000);
    return () => { dead = true; clearInterval(id); };
  }, [notify, maxLines]);

  useEffect(() => { if (follow && box.current) box.current.scrollTop = box.current.scrollHeight; }, [lines, follow]);

  const q = filter.trim().toLowerCase();
  const shown = q ? lines.filter((l) => l.toLowerCase().includes(q)) : lines;
  return (
    <div className={compact ? "space-y-2" : "flex h-full flex-col gap-3"}>
      <div className="flex flex-wrap items-center gap-3">
        <input className="input max-w-xs" placeholder="Filtrer (ex. error, joined)" value={filter} onChange={(e) => setFilter(e.target.value)} />
        <label className="text-sm"><input type="checkbox" checked={follow} onChange={(e) => setFollow(e.target.checked)} /> Suivre</label>
        <span className="text-xs text-slate-500">{shown.length} / {lines.length} lignes · {info.source || "aucun fichier"}</span>
      </div>
      <div ref={box} aria-live="off"
        className={`card overflow-y-auto font-mono text-xs leading-5 ${compact ? "h-64" : "min-h-0 flex-1"}`}>
        {shown.length === 0 ? <p className="text-slate-500">{info.hint ?? "Aucune ligne pour l'instant."}</p>
          : shown.map((l, i) => <div key={i} className={/error|fatal|crash/i.test(l) ? "text-red-400" : /warn/i.test(l) ? "text-amber-300" : ""}>{l}</div>)}
      </div>
    </div>
  );
}
