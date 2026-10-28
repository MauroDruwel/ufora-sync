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
    let colorClass = "text-[#c4c7c5]";
    if (line.includes("✓") || line.includes("All ") || line.includes("up to date")) {
      colorClass = "text-[#6dd58c]";
    } else if (line.includes("✗") || line.includes("Error") || line.includes("fail")) {
      colorClass = "text-[#f28b82] font-medium";
    } else if (line.includes("✎") || line.includes("Kept") || line.includes("Preserved")) {
      colorClass = "text-[#ffb74d]";
    } else if (line.includes("Inspecting") || line.includes("Scanning") || line.includes("Starting")) {
      colorClass = "text-[#a8c7fa]";
    }

    return (
      <div key={index} className={`font-mono text-xs leading-relaxed ${colorClass}`}>
        {line}
      </div>
    );
  };

  return (
    <div className="flex flex-col h-full bg-[#1e1f20] border border-white/[0.08] rounded-2xl overflow-hidden">
      {/* Top bar */}
      <div className="h-10 px-4 bg-[#1e1f20] border-b border-white/[0.06] flex items-center justify-between text-xs text-[#8e918f] select-none">
        <div className="flex items-center space-x-2.5">
          <Terminal className="w-3.5 h-3.5 text-[#a8c7fa]" />
          <span className="font-medium text-[#e3e3e3]">Activity Log</span>
          <span className="text-[11px] px-2 py-0.5 rounded-full bg-[#282a2c] text-[#c4c7c5] font-mono">
            {logs.length} lines
          </span>
        </div>

        <div className="flex items-center space-x-2">
          <button
            onClick={onTogglePause}
            className="flex items-center space-x-1.5 px-3 py-1 rounded-full text-[11px] font-medium bg-[#282a2c] hover:bg-[#37393b] text-[#c4c7c5] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          >
            {isPaused ? <Play className="w-3 h-3 text-[#6dd58c]" /> : <Pause className="w-3 h-3 text-[#ffb74d]" />}
            <span>{isPaused ? "Resume" : "Pause"}</span>
          </button>

          <button
            onClick={() => setAutoScroll(!autoScroll)}
            className={`px-3 py-1 rounded-full text-[11px] font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] ${
              autoScroll
                ? "bg-[#a8c7fa]/15 text-[#a8c7fa] border border-[#a8c7fa]/20"
                : "bg-[#282a2c] text-[#8e918f] hover:text-[#c4c7c5]"
            }`}
          >
            Auto-scroll
          </button>

          <button
            onClick={onClear}
            className="w-7 h-7 rounded-full flex items-center justify-center text-[#8e918f] hover:text-[#f28b82] hover:bg-[#282a2c] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)]"
            title="Clear logs"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Log scroll area */}
      <div ref={containerRef} className="flex-1 p-4 overflow-y-auto space-y-1 select-text">
        {logs.length === 0 ? (
          <div className="text-[#8e918f] font-mono text-xs italic">Waiting for sync activity…</div>
        ) : (
          logs.map((line, idx) => renderLine(line, idx))
        )}
      </div>
    </div>
  );
};
