import { Cloud, RefreshCw, Settings, Folder, ExternalLink } from "lucide-react";

interface HeaderProps {
  status: "idle" | "syncing" | "paused" | "error";
  onSyncNow: () => void;
  onOpenSettings: () => void;
  onOpenSyncFolder: () => void;
  onOpenUforaWeb: () => void;
}

export const Header = ({
  status,
  onSyncNow,
  onOpenSettings,
  onOpenSyncFolder,
  onOpenUforaWeb,
}: HeaderProps) => {
  const isSyncing = status === "syncing";

  return (
    <header className="h-16 border-b border-slate-800/80 bg-[#0f172a]/95 backdrop-blur px-6 flex items-center justify-between select-none">
      <div className="flex items-center space-x-3">
        <div className="w-9 h-9 rounded-xl bg-ugent-blue/20 border border-ugent-blue/40 flex items-center justify-center text-ugent-blue shadow-lg shadow-ugent-blue/10">
          <Cloud className="w-5 h-5 fill-ugent-blue/20" />
        </div>
        <div>
          <div className="flex items-center space-x-2">
            <h1 className="font-semibold text-base text-slate-100 tracking-tight">Ufora Sync</h1>
            <span className="text-[10px] font-mono uppercase tracking-wider px-1.5 py-0.5 rounded bg-ugent-blue/15 text-ugent-blue border border-ugent-blue/30 font-semibold">
              Tauri v2
            </span>
          </div>
          <div className="flex items-center space-x-1.5 text-xs text-slate-400">
            <span
              className={`w-2 h-2 rounded-full ${
                isSyncing
                  ? "bg-blue-400 animate-pulse"
                  : status === "paused"
                  ? "bg-amber-400"
                  : status === "error"
                  ? "bg-rose-400"
                  : "bg-emerald-400"
              }`}
            />
            <span className="capitalize">
              {isSyncing ? "Syncing materials…" : status === "paused" ? "Sync paused" : status === "error" ? "Error" : "Up to date"}
            </span>
          </div>
        </div>
      </div>

      <div className="flex items-center space-x-2">
        <button
          onClick={onSyncNow}
          disabled={isSyncing}
          className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
            isSyncing
              ? "bg-slate-800 text-slate-500 cursor-not-allowed"
              : "bg-ugent-blue hover:bg-blue-600 text-white shadow-md shadow-blue-500/20 active:scale-95"
          }`}
          title="Trigger immediate sync pass"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${isSyncing ? "animate-spin" : ""}`} />
          <span>{isSyncing ? "Syncing…" : "Sync Now"}</span>
        </button>

        <button
          onClick={onOpenSyncFolder}
          className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-medium text-slate-300 bg-slate-800/80 hover:bg-slate-700 border border-slate-700/60 transition active:scale-95"
          title="Open root sync folder in file manager"
        >
          <Folder className="w-3.5 h-3.5" />
          <span>Folder</span>
        </button>

        <button
          onClick={onOpenUforaWeb}
          className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 bg-slate-800/40 hover:bg-slate-800 border border-slate-700/40 transition active:scale-95"
          title="Open UGent Ufora in browser"
        >
          <ExternalLink className="w-4 h-4" />
        </button>

        <button
          onClick={onOpenSettings}
          className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 bg-slate-800/40 hover:bg-slate-800 border border-slate-700/40 transition active:scale-95"
          title="Open Settings"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};
