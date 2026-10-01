import { useState } from "react";
import { api } from "../lib/api";

export default function LockScreen({ onUnlock }: { onUnlock: () => void }) {
  const [pin, setPin] = useState("");
  const [err, setErr] = useState(false);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    setBusy(true); setErr(false);
    try { if (await api.lockVerify(pin)) onUnlock(); else { setErr(true); setPin(""); } } finally { setBusy(false); }
  };
  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-slate-950 p-4" role="dialog" aria-modal="true" aria-label="Application verrouillée">
      <form className="card w-full max-w-sm space-y-3" onSubmit={(e) => { e.preventDefault(); submit(); }}>
        <h1 className="text-lg font-bold text-pal-500">Palworld Manager</h1>
        <p className="text-sm text-slate-400">Application verrouillée. Entrez le code.</p>
        <input className="input" type="password" autoFocus autoComplete="off" value={pin} onChange={(e) => setPin(e.target.value)} aria-label="Code" />
        {err && <p className="text-sm text-red-400" role="alert">Code incorrect.</p>}
        <button className="btn-primary w-full" type="submit" disabled={busy || !pin}>Déverrouiller</button>
      </form>
    </div>
  );
}
