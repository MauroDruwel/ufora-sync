import { useEffect, useState } from "react";
import { RefreshCw, Settings, Folder, ExternalLink, LogIn } from "lucide-react";
import { formatRelativeSyncTime } from "../utils/syncTime";

export interface HeaderProps {
  status: "idle" | "syncing" | "paused" | "error";
  lastSyncTime?: string | null;
  lastSyncStatus?: string | null;
  isAuthenticated?: boolean;
  onSyncNow: () => void;
  onLogin?: () => void;
  onOpenSettings: () => void;
  onOpenSyncFolder: () => void;
  onOpenUforaWeb: () => void;
}

export const Header = ({
  status,
  lastSyncTime,
  lastSyncStatus,
  isAuthenticated = true,
  onSyncNow,
  onLogin,
  onOpenSettings,
  onOpenSyncFolder,
  onOpenUforaWeb,
}: HeaderProps) => {
  const isSyncing = status === "syncing";
  const [isMac, setIsMac] = useState(true);
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    if (typeof navigator !== "undefined") {
      setIsMac(/Mac|iPhone|iPod|iPad/i.test(navigator.userAgent));
    }
  }, []);

  // Refresh relative time every 15 seconds
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 15000);
    return () => clearInterval(timer);
  }, []);

  let dotColor = "bg-[#6dd58c]";
  let statusLabel = "";

  if (isSyncing) {
    dotColor = "bg-[#1a73e8] dark:bg-[#a8c7fa] animate-pulse";
    statusLabel = "Syncing materials…";
  } else if (status === "paused") {
    dotColor = "bg-[#ffb74d]";
    statusLabel = "Sync paused";
  } else if (!isAuthenticated) {
    dotColor = "bg-[#f28b82]";
    statusLabel = "Authentication required";
  } else if (status === "error" || lastSyncStatus === "error") {
    dotColor = "bg-[#f28b82]";
    statusLabel = "Sync error";
  } else if (lastSyncStatus === "warning") {
    dotColor = "bg-[#ffb74d]";
    statusLabel = `Warning · ${formatRelativeSyncTime(lastSyncTime, now)}`;
  } else {
    dotColor = "bg-[#6dd58c]";
    statusLabel = formatRelativeSyncTime(lastSyncTime, now);
  }

  return (
    <header
      data-tauri-drag-region
      className={`h-14 border-b border-black/[0.08] dark:border-white/[0.08] bg-[#ffffff] dark:bg-[#131314] pr-6 flex items-center justify-between select-none transition-colors duration-150 ${
        isMac ? "pl-20" : "pl-6"
      }`}
    >
      {/* Left side: Clean macOS window drag area (no traffic light clash) */}
      <div data-tauri-drag-region className="flex-1 h-full" />

      {/* Right side: Dynamic status pill + Action buttons */}
      <div className="flex items-center space-x-2.5">
        {/* Dynamic Status Pill */}
        <div
          className="flex items-center space-x-2 px-3 py-1 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08] text-xs font-medium text-[#444746] dark:text-[#c4c7c5] transition-colors duration-150"
          title={statusLabel}
        >
          <span className={`w-2 h-2 rounded-full shrink-0 transition-colors duration-200 ${dotColor}`} />
          <span className="tracking-tight">{statusLabel}</span>
        </div>

        {!isAuthenticated && onLogin && (
          <button
            onClick={onLogin}
            className="flex items-center space-x-1.5 px-3.5 py-1.5 rounded-full text-xs font-medium bg-[#1a73e8] dark:bg-[#a8c7fa] hover:bg-[#1765cc] dark:hover:bg-[#c2e7ff] text-[#ffffff] dark:text-[#062e6f] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98]"
            title="Log in to UGent Ufora"
          >
            <LogIn className="w-3.5 h-3.5" />
            <span>Log in with UGent</span>
          </button>
        )}

        <button
          onClick={onSyncNow}
          disabled={isSyncing}
          className={`flex items-center space-x-1.5 px-3.5 py-1.5 rounded-full text-xs font-medium border border-black/[0.08] dark:border-white/[0.08] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98] ${
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
          className="w-8 h-8 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open root sync folder in file manager"
        >
          <Folder className="w-3.5 h-3.5" />
        </button>

        <button
          onClick={onOpenUforaWeb}
          className="w-8 h-8 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open UGent Ufora in browser"
        >
          <ExternalLink className="w-3.5 h-3.5" />
        </button>

        <button
          onClick={onOpenSettings}
          className="w-8 h-8 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] flex items-center justify-center transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
          title="Open Preferences & Settings"
        >
          <Settings className="w-3.5 h-3.5" />
        </button>
      </div>
    </header>
  );
};
