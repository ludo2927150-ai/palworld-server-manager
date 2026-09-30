import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { GROUPS, SCHEMA } from "../lib/schema";
import type { Opt } from "../lib/types";

export default function Config({ notify }: { notify: (m: string) => void }) {
  const [opts, setOpts] = useState<Opt[] | null>(null);
  const [tab, setTab] = useState(GROUPS[0]);

  useEffect(() => { api.readWorld().then(setOpts).catch((e) => notify(String(e))); }, [notify]);
  if (!opts) return <p className="text-slate-400">Chargement…</p>;

  const get = (k: string) => opts.find((o) => o.key === k)?.value ?? "";
  const set = (k: string, value: string, quoted: boolean) =>
    setOpts((cur) => {
      const list = cur ?? [];
      return list.some((o) => o.key === k) ? list.map((o) => (o.key === k ? { ...o, value } : o)) : [...list, { key: k, value, quoted }];
    });
  const save = () => api.writeWorld(opts).then(() => notify("Configuration enregistrée (redémarrage requis)")).catch((e) => notify(String(e)));
  const known = new Set(SCHEMA.map((f) => f.key));

  return (
    <div className="space-y-4">
      <div className="flex gap-2">
        {[...GROUPS, "Avancé"].map((g) => (
          <button key={g} className={g === tab ? "btn-primary" : "btn"} onClick={() => setTab(g)}>{g}</button>
        ))}
        <button className="btn-primary ml-auto" onClick={save}>Enregistrer</button>
      </div>
      <div className="card grid gap-4 md:grid-cols-2">
        {tab !== "Avancé" && SCHEMA.filter((f) => f.group === tab).map((f) => (
          <label key={f.key} className="block text-sm">
            <span className="mb-1 block text-slate-400">{f.label} <code className="text-xs text-slate-600">{f.key}</code></span>
            {f.type === "bool" ? (
              <input type="checkbox" checked={get(f.key) === "True"} onChange={(e) => set(f.key, e.target.checked ? "True" : "False", false)} />
            ) : f.type === "enum" ? (
              <select className="input" value={get(f.key)} onChange={(e) => set(f.key, e.target.value, false)}>
                {f.options.map((o) => <option key={o}>{o}</option>)}
              </select>
            ) : f.type === "number" ? (
              <input className="input" type="number" min={f.min} max={f.max} step={f.step} value={get(f.key)} onChange={(e) => set(f.key, e.target.value, false)} />
            ) : (
              <input className="input" type={f.type} value={get(f.key)} onChange={(e) => set(f.key, e.target.value, true)} />
            )}
          </label>
        ))}
        {tab === "Avancé" && opts.filter((o) => !known.has(o.key)).map((o) => (
          <label key={o.key} className="block text-sm">
            <span className="mb-1 block text-slate-400">{o.key}</span>
            <input className="input" value={o.value} onChange={(e) => set(o.key, e.target.value, o.quoted)} />
          </label>
        ))}
      </div>
    </div>
  );
}
