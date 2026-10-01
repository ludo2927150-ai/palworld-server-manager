import LogAnalysis from "../components/LogAnalysis";
import LogViewer from "../components/LogViewer";

export default function Logs({ notify }: { notify: (m: string) => void }) {
  return (
    <div className="flex h-full flex-col gap-4">
      <LogAnalysis notify={notify} />
      <div className="min-h-0 flex-1"><LogViewer notify={notify} /></div>
    </div>
  );
}
