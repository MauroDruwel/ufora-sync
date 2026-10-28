import { useEffect, useState } from "react";
import { Sparkles, RefreshCw, Settings, Folder, ExternalLink, LogIn, LogOut } from "lucide-react";

interface HeaderProps {
  status: "idle" | "syncing" | "paused" | "error";
  isAuthenticated?: boolean;
  onSyncNow: () => void;
  onLogin?: () => void;
  onLogout?: () => void;
  onOpenSettings: () => void;
  onOpenSyncFolder: () => void;
  onOpenUforaWeb: () => void;
}

export const Header = ({
  status,
  isAuthenticated = true,
  onSyncNow,
  onLogin,
  onLogout,
  onOpenSettings,
  onOpenSyncFolder,
  onOpenUforaWeb,
}: HeaderProps) => {
  const isSyncing = status === "syncing";
  const [isMac, setIsMac] = useState(true);

  useEffect(() => {
    if (typeof navigator !== "undefined") {
      setIsMac(/Mac|iPhone|iPod|iPad/i.test(navigator.userAgent));
    }
  }, []);

  return (
    <header
      data-tauri-drag-region
      className={`h-16 border-b border-black/[0.08] dark:border-white/[0.08] bg-[#ffffff] dark:bg-[#131314] pr-6 flex items-center justify-between select-none transition-colors duration-150 ${
        isMac ? "pl-20" : "pl-6"
      }`}
    >
      {/* Title & Status */}
      <div data-tauri-drag-region className="flex items-center space-x-3.5">
        <div className="w-9 h-9 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08] flex items-center justify-center text-[#1a73e8] dark:text-[#a8c7fa] shrink-0">
          <Sparkles className="w-4 h-4" />
        </div>
        <div>
          <div className="flex items-center space-x-2">
            <h1 className="font-medium text-[15px] text-[#1f1f1f] dark:text-[#e3e3e3] tracking-normal">
              Ufora Sync
            </h1>
            <span className="text-[11px] font-mono px-2 py-0.5 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] text-[#444746] dark:text-[#c4c7c5] border border-black/[0.08] dark:border-white/[0.08]">
              v1.0
            </span>
          </div>
          <div className="flex items-center space-x-1.5 text-xs text-[#747775] dark:text-[#8e918f]">
            <span
              className={`w-2 h-2 rounded-full ${
                isSyncing
                  ? "bg-[#1a73e8] dark:bg-[#a8c7fa] animate-pulse"
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

      {/* Action buttons */}
      <div className="flex items-center space-x-2">
        {!isAuthenticated && onLogin && (
          <button
            onClick={onLogin}
            className="flex items-center space-x-1.5 px-4 py-1.5 rounded-full text-xs font-medium bg-[#1a73e8] dark:bg-[#a8c7fa] hover:bg-[#1765cc] dark:hover:bg-[#c2e7ff] text-[#ffffff] dark:text-[#062e6f] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98]"
            title="Log in to UGent Ufora"
          >
            <LogIn className="w-3.5 h-3.5" />
            <span>Log in with UGent</span>
          </button>
        )}

        <button
          onClick={onSyncNow}
          disabled={isSyncing}
          className={`flex items-center space-x-1.5 px-4 py-1.5 rounded-full text-xs font-medium border border-black/[0.08] dark:border-white/[0.08] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98] ${
            isSyncing
              ? "bg-[#f0f4f9] dark:bg-[#1e1f20] text-[#747775] dark:text-[#8e918f] cursor-not-allowed"
              : "bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#1f1f1f] dark:text-[#e3e3e3]"
          }`}
          title="Trigger immediate sync pass"
        >
          <RefreshCw
            className={`w-3.5 h-3.5 ${
              isSyncing ? "animate-spin text-[#1a73e8] dark:text-[#a8c7fa]" : "text-[#444746] dark:text-[#c4c7c5]"
            }`}
          />
          <span>{isSyncing ? "Syncing…" : "Sync Now"}</span>
        </button>

        <button
          onClick={onOpenSyncFolder}
          className="w-9 h-9 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open root sync folder in file manager"
        >
          <Folder className="w-4 h-4" />
        </button>

        <button
          onClick={onOpenUforaWeb}
          className="w-9 h-9 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open UGent Ufora in browser"
        >
          <ExternalLink className="w-4 h-4" />
        </button>

        <button
          onClick={onOpenSettings}
          className="w-9 h-9 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open Settings"
        >
          <Settings className="w-4 h-4" />
        </button>

        {isAuthenticated && onLogout && (
          <button
            onClick={onLogout}
            className="w-9 h-9 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#b3261e]/10 dark:hover:bg-[#b3261e]/20 text-[#747775] dark:text-[#8e918f] hover:text-[#b3261e] dark:hover:text-[#f2b8b5] border border-black/[0.08] dark:border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
            title="Log out from UGent Ufora"
          >
            <LogOut className="w-4 h-4" />
          </button>
        )}
      </div>
    </header>
  );
};
