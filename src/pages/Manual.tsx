import { useEffect, useMemo, useRef, useState } from "react";
import manualMd from "../../docs/MANUEL.md?raw";
import { renderManual } from "../lib/manual";

/** Manuel d'utilisation intégré : sommaire, recherche et lecture, hors ligne (le texte est embarqué dans l'application). */
export default function Manual() {
  const { html, sections } = useMemo(() => renderManual(manualMd), []);
  const [q, setQ] = useState("");
  const box = useRef<HTMLDivElement>(null);
  const toc = sections.filter((s) => s.level === 2);
  const query = q.trim().toLowerCase();
  const hits = query.length >= 2 ? sections.filter((s) => s.level >= 2 && (s.title.toLowerCase().includes(query) || s.text.includes(query))).slice(0, 30) : [];

  const go = (id: string) => { box.current?.querySelector(`#${CSS.escape(id)}`)?.scrollIntoView({ behavior: "smooth", block: "start" }); };

  // Les liens « #chapitre » du sommaire défilent dans la page au lieu de changer l'adresse de l'application.
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const onClick = (e: MouseEvent) => {
      const a = (e.target as HTMLElement).closest("a");
      const href = a?.getAttribute("href");
      if (href?.startsWith("#")) { e.preventDefault(); go(decodeURIComponent(href.slice(1))); }
    };
    el.addEventListener("click", onClick);
    return () => el.removeEventListener("click", onClick);
  }, []);

  return (
    <div className="flex h-full min-h-0 gap-4">
      <aside className="hidden w-64 shrink-0 flex-col gap-2 overflow-y-auto lg:flex" aria-label="Sommaire du manuel">
        <input className="input" type="search" placeholder="Rechercher dans le manuel…" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Rechercher dans le manuel" />
        {query.length >= 2 ? (
          <ul className="space-y-1 text-sm">
            {hits.map((s) => <li key={s.id}><button className="w-full rounded px-2 py-1 text-left hover:bg-slate-800" onClick={() => go(s.id)}>{s.title}</button></li>)}
            {hits.length === 0 && <li className="px-2 text-slate-500">Aucun résultat.</li>}
          </ul>
        ) : (
          <ul className="space-y-0.5 text-sm">
            {toc.map((s) => <li key={s.id}><button className="w-full rounded px-2 py-1 text-left text-slate-300 hover:bg-slate-800" onClick={() => go(s.id)}>{s.title}</button></li>)}
          </ul>
        )}
      </aside>
      <div ref={box} className="manual min-h-0 flex-1 overflow-y-auto pr-2" dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  );
}
