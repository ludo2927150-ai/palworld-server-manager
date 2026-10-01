import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { AuditEntry } from "../lib/types";

const who = (w: string) => (w === "app" ? "Application" : w.startsWith("mobile:") ? `Mobile · ${w.slice(7)}` : w.startsWith("discord:") ? `Discord · ${w.slice(8)}` : w.startsWith("auto:") ? `Automatique · ${w.slice(5)}` : w);

/** Qui a fait quoi : application, mobile, Discord et automatismes (90 jours). */
export default function AuditPanel() {
  const [rows, setRows] = useState<AuditEntry[]>([]);
  const [q, setQ] = useState("");
  const load = () => api.auditRecent(300).then(setRows).catch(() => {});
  useEffect(() => { load(); const id = setInterval(load, 10_000); return () => clearInterval(id); }, []);
  const f = q.trim().toLowerCase();
  const shown = f ? rows.filter((r) => `${who(r.who)} ${r.action} ${r.detail}`.toLowerCase().includes(f)) : rows;
  return (
    <div className="card space-y-2">
      <div className="flex items-center gap-3">
        <h2 className="font-semibold">Journal d'audit</h2>
        <input className="input ml-auto max-w-xs" placeholder="Filtrer (ex. Discord, restauration)" value={q} onChange={(e) => setQ(e.target.value)} />
      </div>
      <p className="text-xs text-slate-500">Qui a lancé quoi (application, téléphone, Discord, automatismes), conservé 90 jours.</p>
      <div className="max-h-72 overflow-y-auto">
        <table className="w-full text-left text-sm">
          <thead className="text-slate-400"><tr><th className="w-40">Quand</th><th className="w-48">Qui</th><th>Action</th></tr></thead>
          <tbody>
            {shown.map((r, i) => (
              <tr key={i} className="border-t border-slate-800 align-top">
                <td className="py-1.5 text-xs text-slate-400">{new Date(r.t * 1000).toLocaleString("fr-FR")}</td>
                <td>{who(r.who)}</td>
                <td>{r.action}{r.detail && <span className="ml-2 break-all text-xs text-slate-500">{r.detail}</span>}</td>
              </tr>
            ))}
            {shown.length === 0 && <tr><td colSpan={3} className="py-2 text-slate-500">Rien à afficher.</td></tr>}
          </tbody>
        </table>
      </div>
    </div>
  );
}
