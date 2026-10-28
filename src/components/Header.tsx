import { Sparkles, RefreshCw, Settings, Folder, ExternalLink, LogIn } from "lucide-react";

interface HeaderProps {
  status: "idle" | "syncing" | "paused" | "error";
  isAuthenticated?: boolean;
  onSyncNow: () => void;
  onLogin?: () => void;
  onOpenSettings: () => void;
  onOpenSyncFolder: () => void;
  onOpenUforaWeb: () => void;
}

export const Header = ({
  status,
  isAuthenticated = true,
  onSyncNow,
  onLogin,
  onOpenSettings,
  onOpenSyncFolder,
  onOpenUforaWeb,
}: HeaderProps) => {
  const isSyncing = status === "syncing";

  return (
    <header className="h-16 border-b border-white/[0.08] bg-[#131314] px-6 flex items-center justify-between select-none">
      <div className="flex items-center space-x-3.5">
        <div className="w-9 h-9 rounded-full bg-[#1e1f20] border border-white/[0.08] flex items-center justify-center text-[#a8c7fa]">
          <Sparkles className="w-4 h-4" />
        </div>
        <div>
          <div className="flex items-center space-x-2">
            <h1 className="font-medium text-[15px] text-[#e3e3e3] tracking-normal">Ufora Sync</h1>
            <span className="text-[11px] font-mono px-2 py-0.5 rounded-full bg-[#1e1f20] text-[#c4c7c5] border border-white/[0.08]">
              v1.0
            </span>
          </div>
          <div className="flex items-center space-x-1.5 text-xs text-[#8e918f]">
            <span
              className={`w-2 h-2 rounded-full ${
                isSyncing
                  ? "bg-[#a8c7fa] animate-pulse"
                  : status === "paused"
                  ? "bg-[#ffb74d]"
                  : status === "error"
                  ? "bg-[#f28b82]"
                  : "bg-[#6dd58c]"
              }`}
            />
            <span>
              {isSyncing
                ? "Syncing materials…"
                : status === "paused"
                ? "Sync paused"
                : status === "error"
                ? "Sync error"
                : "Up to date"}
            </span>
          </div>
        </div>
      </div>

      <div className="flex items-center space-x-2">
        {!isAuthenticated && onLogin && (
          <button
            onClick={onLogin}
            className="flex items-center space-x-1.5 px-4 py-1.5 rounded-full text-xs font-medium bg-[#a8c7fa] hover:bg-[#c2e7ff] text-[#062e6f] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98]"
            title="Log in to UGent Ufora"
          >
            <LogIn className="w-3.5 h-3.5" />
            <span>Log in with UGent</span>
          </button>
        )}

        <button
          onClick={onSyncNow}
          disabled={isSyncing}
          className={`flex items-center space-x-1.5 px-4 py-1.5 rounded-full text-xs font-medium border border-white/[0.08] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98] ${
            isSyncing
              ? "bg-[#1e1f20] text-[#8e918f] cursor-not-allowed"
              : "bg-[#1e1f20] hover:bg-[#282a2c] text-[#e3e3e3]"
          }`}
          title="Trigger immediate sync pass"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${isSyncing ? "animate-spin text-[#a8c7fa]" : "text-[#c4c7c5]"}`} />
          <span>{isSyncing ? "Syncing…" : "Sync Now"}</span>
        </button>

        <button
          onClick={onOpenSyncFolder}
          className="w-9 h-9 rounded-full bg-[#1e1f20] hover:bg-[#282a2c] text-[#c4c7c5] hover:text-[#e3e3e3] border border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open root sync folder in file manager"
        >
          <Folder className="w-4 h-4" />
        </button>

        <button
          onClick={onOpenUforaWeb}
          className="w-9 h-9 rounded-full bg-[#1e1f20] hover:bg-[#282a2c] text-[#c4c7c5] hover:text-[#e3e3e3] border border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open UGent Ufora in browser"
        >
          <ExternalLink className="w-4 h-4" />
        </button>

        <button
          onClick={onOpenSettings}
          className="w-9 h-9 rounded-full bg-[#1e1f20] hover:bg-[#282a2c] text-[#c4c7c5] hover:text-[#e3e3e3] border border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open Settings"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};
