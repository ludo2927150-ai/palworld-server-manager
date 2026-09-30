import LogViewer from "../components/LogViewer";

export default function Logs({ notify }: { notify: (m: string) => void }) {
  return <LogViewer notify={notify} />;
}
