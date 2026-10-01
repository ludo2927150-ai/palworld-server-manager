import LogAnalysis from "../components/LogAnalysis";
import LogViewer from "../components/LogViewer";
import AuditPanel from "../components/AuditPanel";

export default function Logs({ notify }: { notify: (m: string) => void }) {
  return (
    <div className="flex h-full flex-col gap-4">
      <LogAnalysis notify={notify} />
      <div className="min-h-[16rem] flex-1"><LogViewer notify={notify} /></div>
      <AuditPanel />
    </div>
  );
}
