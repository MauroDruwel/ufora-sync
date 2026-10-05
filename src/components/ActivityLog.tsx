import { useEffect, useRef, useState } from "react";
import { Terminal, Trash2, Pause, Play } from "lucide-react";

interface ActivityLogProps {
  logs: string[];
  isPaused: boolean;
  onClear: () => void;
  onTogglePause: () => void;
}

export const ActivityLog = ({
  logs,
  isPaused,
  onClear,
  onTogglePause,
}: ActivityLogProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const renderLine = (line: string, index: number) => {
    let colorClass = "text-slate-300";
    if (line.includes("✓") || line.includes("All ") || line.includes("up to date")) {
      colorClass = "text-emerald-400";
    } else if (line.includes("✗") || line.includes("Error") || line.includes("fail")) {
      colorClass = "text-rose-400 font-medium";
    } else if (line.includes("✎") || line.includes("Kept") || line.includes("Preserved")) {
      colorClass = "text-amber-300";
    } else if (line.includes("Inspecting") || line.includes("Scanning") || line.includes("Starting")) {
      colorClass = "text-sky-300";
    }

    return (
      <div key={index} className={`font-mono text-xs leading-relaxed ${colorClass}`}>
        {line}
      </div>
    );
  };

  return (
    <div className="flex flex-col h-full bg-[#0a0f1d] border border-slate-800/80 rounded-xl overflow-hidden shadow-inner">
      {/* Top bar */}
      <div className="h-9 px-3 bg-slate-900/90 border-b border-slate-800/80 flex items-center justify-between text-xs text-slate-400 select-none">
        <div className="flex items-center space-x-2">
          <Terminal className="w-3.5 h-3.5 text-ugent-blue" />
          <span className="font-semibold text-slate-300">Live Activity Log</span>
          <span className="text-[10px] px-1.5 py-0.2 rounded bg-slate-800 text-slate-400 font-mono">
            {logs.length} lines
          </span>
        </div>

        <div className="flex items-center space-x-1.5">
          <button
            onClick={onTogglePause}
            className="flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] bg-slate-800 hover:bg-slate-700 text-slate-300 transition"
          >
            {isPaused ? <Play className="w-3 h-3 text-emerald-400" /> : <Pause className="w-3 h-3 text-amber-400" />}
            <span>{isPaused ? "Resume" : "Pause"}</span>
          </button>

          <button
            onClick={() => setAutoScroll(!autoScroll)}
            className={`px-2 py-0.5 rounded text-[11px] transition ${
              autoScroll ? "bg-ugent-blue/20 text-ugent-blue font-medium" : "bg-slate-800 text-slate-400"
            }`}
          >
            Auto-scroll
          </button>

          <button
            onClick={onClear}
            className="p-1 rounded text-slate-400 hover:text-rose-400 hover:bg-slate-800 transition"
            title="Clear logs view"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Log scroll area */}
      <div ref={containerRef} className="flex-1 p-3.5 overflow-y-auto space-y-1 select-text">
        {logs.length === 0 ? (
          <div className="text-slate-500 font-mono text-xs italic">Waiting for sync activity…</div>
        ) : (
          logs.map((line, idx) => renderLine(line, idx))
        )}
      </div>
    </div>
  );
};
