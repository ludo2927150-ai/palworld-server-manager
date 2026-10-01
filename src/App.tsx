import { useCallback, useEffect, useState } from "react";
import { api, inTauri, onSnapshot } from "./lib/api";
import type { Snapshot, UpdateInfo } from "./lib/types";
import Dashboard from "./pages/Dashboard";
import Players from "./pages/Players";
import Config from "./pages/Config";
import Backups from "./pages/Backups";
import Settings from "./pages/Settings";
import History from "./pages/History";
import Logs from "./pages/Logs";
import Diagnostic from "./pages/Diagnostic";
import Join from "./pages/Join";
import Performance from "./pages/Performance";
import Mobile from "./pages/Mobile";
import Mods from "./pages/Mods";
import { t, getLang, setLang } from "./lib/i18n";
import Wizard from "./pages/Wizard";
import Announcements from "./pages/Announcements";

const TABS = ["Tableau de bord", "Rejoindre", "Historique", "Performance", "Mods", "Joueurs", "Annonces", "Configuration", "Journal", "Sauvegardes", "Diagnostic", "Mobile", "Application"] as const;
type Tab = (typeof TABS)[number];

export default function App() {
  const [, bump] = useState(0);
  const [tab, setTab] = useState<Tab>("Tableau de bord");
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [wizard, setWizard] = useState(false);
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [updating, setUpdating] = useState(false);
  const notify = useCallback((m: string) => { setToast(m); setTimeout(() => setToast(null), 4000); }, []);

  useEffect(() => {
    if (!inTauri) return;
    const check = () => api.checkUpdate().then(setUpdate).catch(() => {});
    const t0 = setTimeout(check, 8000);
    const iv = setInterval(check, 6 * 3600 * 1000);
    return () => { clearTimeout(t0); clearInterval(iv); };
  }, []);

  useEffect(() => { if (inTauri) api.getSettings().then((s) => setWizard(!s.setup_done)).catch(() => {}); }, []);

  useEffect(() => {
    api.snapshot().then(setSnap).catch(() => {});
    let un: (() => void) | undefined;
    let dead = false;
    onSnapshot(setSnap).then((f) => (dead ? f() : (un = f)));
    return () => { dead = true; un?.(); };
  }, []);

  return (
    <div className="flex h-screen flex-col">
      {wizard && <Wizard onClose={() => setWizard(false)} notify={notify} />}
      {!inTauri && (
        <div className="bg-amber-500 px-4 py-1.5 text-center text-sm font-medium text-black" role="alert">
          {t("MODE DÉMO — données fictives affichées dans le navigateur. Lancez « npm run tauri dev » (ou l'installeur) pour piloter un vrai serveur.")}
        </div>
      )}
      {update && (
        <div className="flex items-center gap-3 bg-pal-500 px-4 py-1.5 text-sm font-medium text-black" role="status">
          {t("Nouvelle version disponible : {latest} (vous avez {current}).", { latest: update.latest, current: update.current })}
          <button className="rounded bg-black/80 px-3 py-0.5 text-white disabled:opacity-50" disabled={updating}
            onClick={() => { setUpdating(true); api.installUpdate(update).catch((e) => { setUpdating(false); notify(String(e)); }); }}>
            {updating ? t("Téléchargement…") : t("Télécharger et installer")}
          </button>
          <button className="ml-auto underline" onClick={() => setUpdate(null)}>{t("Plus tard")}</button>
        </div>
      )}
      <div className="flex min-h-0 flex-1">
      <nav className="w-56 shrink-0 border-r border-slate-800 p-3">
        <h1 className="mb-4 px-2 text-lg font-bold text-pal-500">Palworld Manager</h1>
        {TABS.map((tb) => (
          <button key={tb} onClick={() => setTab(tb)} className={`mb-1 block w-full rounded-lg px-3 py-2 text-left text-sm ${tb === tab ? "bg-slate-800" : "hover:bg-slate-900"}`}>{t(tb)}</button>
        ))}
        <label className="mt-4 block px-2 text-xs text-slate-500">{t("Langue")}
          <select className="input mt-1" value={getLang()} onChange={(e) => { setLang(e.target.value as "fr" | "en"); bump((n) => n + 1); }}>
            <option value="fr">Français</option><option value="en">English</option>
          </select>
        </label>
      </nav>
      <main className="min-h-0 flex-1 overflow-y-auto p-6">
        {tab === "Tableau de bord" && <Dashboard snap={snap} notify={notify} />}
        {tab === "Rejoindre" && <Join notify={notify} />}
        {tab === "Performance" && <Performance snap={snap} notify={notify} />}
        {tab === "Historique" && <History notify={notify} />}
        {tab === "Journal" && <Logs notify={notify} />}
        {tab === "Annonces" && <Announcements notify={notify} />}
        {tab === "Mods" && <Mods notify={notify} />}
        {tab === "Mobile" && <Mobile notify={notify} />}
        {tab === "Diagnostic" && <Diagnostic notify={notify} />}
        {tab === "Joueurs" && <Players snap={snap} notify={notify} />}
        {tab === "Configuration" && <Config notify={notify} />}
        {tab === "Sauvegardes" && <Backups notify={notify} />}
        {tab === "Application" && <Settings notify={notify} />}
      </main>
      </div>
      {toast && <div className="fixed bottom-4 right-4 rounded-lg bg-slate-800 px-4 py-2 text-sm shadow-lg" role="status">{toast}</div>}
    </div>
  );
}
