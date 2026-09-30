import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "../lib/api";
import { GROUPS, PRESETS, SCHEMA } from "../lib/schema";
import type { Opt } from "../lib/types";

export default function Config({ notify }: { notify: (m: string) => void }) {
  const [orig, setOrig] = useState<Opt[] | null>(null);
  const [opts, setOpts] = useState<Opt[] | null>(null);
  const [tab, setTab] = useState(GROUPS[0]);
  const [reviewing, setReviewing] = useState(false);
  const [fromDefault, setFromDefault] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const load = useCallback(() => api.readWorld().then(({ options, from_default }) => { setOrig(from_default ? [] : options); setOpts(options); setFromDefault(from_default); }).catch((e) => notify(String(e))), [notify]);
  useEffect(() => { load(); }, [load]);

  const changes = useMemo(() => {
    if (!orig || !opts) return [];
    const before = new Map(orig.map((o) => [o.key, o.value]));
    return opts.filter((o) => before.get(o.key) !== o.value).map((o) => ({ key: o.key, from: before.get(o.key) ?? "(absent)", to: o.value }));
  }, [orig, opts]);

  if (!opts) return <p className="text-slate-400">Chargement…</p>;

  const get = (k: string) => opts.find((o) => o.key === k)?.value ?? "";
  const set = (k: string, value: string, quoted: boolean) =>
    setOpts((cur) => {
      const list = cur ?? [];
      return list.some((o) => o.key === k) ? list.map((o) => (o.key === k ? { ...o, value } : o)) : [...list, { key: k, value, quoted }];
    });
  const quotedKeys = new Set(SCHEMA.filter((f) => f.type === "text" || f.type === "password").map((f) => f.key));
  const known = new Set(SCHEMA.map((f) => f.key));

  const applyPreset = (values: Record<string, string>) => {
    Object.entries(values).forEach(([k, v]) => set(k, v, quotedKeys.has(k)));
    notify("Préréglage appliqué (pas encore enregistré)");
  };
  const save = () =>
    api.writeWorld(opts).then(() => { setOrig(opts); setFromDefault(false); setReviewing(false); notify("Configuration enregistrée (redémarrage du serveur requis)"); }).catch((e) => notify(String(e)));

  const exportJson = () => {
    const blob = new Blob([JSON.stringify(opts, null, 2)], { type: "application/json" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob); a.download = "PalWorldSettings.json"; a.click();
    URL.revokeObjectURL(a.href);
  };
  const importJson = async (f: File) => {
    try {
      const data = JSON.parse(await f.text());
      if (!Array.isArray(data) || !data.every((o) => typeof o?.key === "string" && typeof o?.value === "string")) throw new Error("format invalide");
      data.forEach((o: Opt) => set(o.key, o.value, !!o.quoted));
      notify(`${data.length} options importées (pas encore enregistrées)`);
    } catch (e) { notify(`Import impossible : ${e}`); }
  };

  return (
    <div className="space-y-4">
      {fromDefault && (
        <div className="rounded-lg border border-amber-500/50 bg-amber-500/10 p-3 text-sm text-amber-200" role="status">
          PalWorldSettings.ini est vide (serveur jamais configuré) : les valeurs affichées viennent de DefaultPalWorldSettings.ini. Modifiez ce que vous voulez puis enregistrez pour créer le fichier.
        </div>
      )}
      <div className="flex flex-wrap items-center gap-2">
        {[...GROUPS, "Avancé"].map((g) => <button key={g} className={g === tab ? "btn-primary" : "btn"} onClick={() => setTab(g)}>{g}</button>)}
      </div>
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <span className="text-slate-400">Préréglages :</span>
        {PRESETS.map((p) => <button key={p.name} className="btn" title={p.description} onClick={() => applyPreset(p.values)}>{p.name}</button>)}
        <button className="btn ml-2" onClick={exportJson}>Exporter</button>
        <button className="btn" onClick={() => fileRef.current?.click()}>Importer</button>
        <input ref={fileRef} type="file" accept="application/json" hidden onChange={(e) => { const f = e.target.files?.[0]; if (f) importJson(f); e.target.value = ""; }} />
        <button className="btn ml-auto" disabled={!changes.length} onClick={() => setOpts(orig)}>Annuler les changements</button>
        <button className="btn-primary" disabled={!changes.length} onClick={() => setReviewing(true)}>Enregistrer ({changes.length})</button>
      </div>

      <div className="card grid gap-4 md:grid-cols-2">
        {tab !== "Avancé" && SCHEMA.filter((f) => f.group === tab).map((f) => (
          <label key={f.key} className="block text-sm">
            <span className="mb-1 block text-slate-400">{f.label} <code className="text-xs text-slate-600">{f.key}</code></span>
            {f.type === "bool" ? (
              <input type="checkbox" checked={get(f.key) === "True"} onChange={(x) => set(f.key, x.target.checked ? "True" : "False", false)} />
            ) : f.type === "enum" ? (
              <select className="input" value={get(f.key)} onChange={(x) => set(f.key, x.target.value, false)}>
                {!f.options.includes(get(f.key)) && <option value={get(f.key)}>{get(f.key) || "—"}</option>}
                {f.options.map((o) => <option key={o}>{o}</option>)}
              </select>
            ) : f.type === "number" ? (
              <input className="input" type="number" min={f.min} max={f.max} step={f.step} value={get(f.key)} onChange={(x) => set(f.key, x.target.value, false)} />
            ) : (
              <input className="input" type={f.type} value={get(f.key)} onChange={(x) => set(f.key, x.target.value, true)} />
            )}
          </label>
        ))}
        {tab === "Avancé" && opts.filter((o) => !known.has(o.key)).map((o) => (
          <label key={o.key} className="block text-sm">
            <span className="mb-1 block text-slate-400">{o.key}</span>
            <input className="input" value={o.value} onChange={(x) => set(o.key, x.target.value, o.quoted)} />
          </label>
        ))}
        {tab === "Avancé" && opts.every((o) => known.has(o.key)) && <p className="text-sm text-slate-500">Toutes les clés du fichier sont couvertes par les autres onglets.</p>}
      </div>

      {reviewing && (
        <div className="fixed inset-0 z-10 flex items-center justify-center bg-black/60 p-4" role="dialog" aria-modal="true" aria-label="Vérifier les changements">
          <div className="card max-h-[80vh] w-full max-w-lg overflow-y-auto">
            <h3 className="mb-3 font-semibold">{changes.length} changement(s)</h3>
            <table className="w-full text-sm"><tbody>
              {changes.map((c) => (
                <tr key={c.key} className="border-t border-slate-800">
                  <td className="py-1 pr-2 text-slate-400">{c.key}</td>
                  <td className="text-red-300 line-through">{c.from}</td><td className="pl-2 text-emerald-300">{c.to}</td>
                </tr>))}
            </tbody></table>
            <p className="mt-3 text-xs text-slate-500">Une copie PalWorldSettings.ini.bak est créée. Redémarrez le serveur pour appliquer.</p>
            <div className="mt-3 flex justify-end gap-2">
              <button className="btn" onClick={() => setReviewing(false)}>Retour</button>
              <button className="btn-primary" onClick={save}>Confirmer</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
