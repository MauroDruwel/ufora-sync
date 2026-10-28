import { useState, FormEvent } from "react";
import { AppConfig, AuthStatus } from "../types";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { enable as enableAutostart, disable as disableAutostart } from "@tauri-apps/plugin-autostart";
import { X, Folder, Check, ShieldCheck, AlertCircle, LogIn } from "lucide-react";

interface SettingsModalProps {
  config: AppConfig;
  authStatus: AuthStatus;
  isOpen: boolean;
  onClose: () => void;
  onSave: (newConfig: AppConfig) => Promise<void>;
  onLogin?: () => void;
}

export const SettingsModal = ({
  config,
  authStatus,
  isOpen,
  onClose,
  onSave,
  onLogin,
}: SettingsModalProps) => {
  const [syncDir, setSyncDir] = useState(config.sync_dir);
  const [interval, setInterval] = useState(config.interval_minutes);
  const [conflictStrategy, setConflictStrategy] = useState(config.conflict_strategy);
  const [suffix, setSuffix] = useState(config.duplicate_suffix);
  const [syncDesc, setSyncDesc] = useState(config.sync_descriptions);
  const [syncLinks, setSyncLinks] = useState(config.sync_links);
  const [autostart, setAutostart] = useState(config.auto_start_tray);
  const [saving, setSaving] = useState(false);
  const [savedSuccess, setSavedSuccess] = useState(false);

  if (!isOpen) return null;

  const handlePickFolder = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: syncDir,
      });
      if (selected && typeof selected === "string") {
        setSyncDir(selected);
      }
    } catch (err) {
      console.error("Failed to open dialog:", err);
    }
  };

  const handleSave = async (e: FormEvent) => {
    e.preventDefault();
    setSaving(false);
    setSaving(true);
    try {
      if (autostart) {
        try {
          await enableAutostart();
        } catch (err) {
          console.warn("Failed to enable autostart:", err);
        }
      } else {
        try {
          await disableAutostart();
        } catch (err) {
          console.warn("Failed to disable autostart:", err);
        }
      }

      await onSave({
        ...config,
        sync_dir: syncDir.trim(),
        interval_minutes: interval,
        conflict_strategy: conflictStrategy,
        duplicate_suffix: suffix.trim() || "_edited",
        sync_descriptions: syncDesc,
        sync_links: syncLinks,
        auto_start_tray: autostart,
      });
      setSavedSuccess(true);
      setTimeout(() => {
        setSavedSuccess(false);
        onClose();
      }, 700);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4">
      <div className="bg-[#1e1f20] border border-white/[0.08] rounded-3xl w-full max-w-lg overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="px-6 py-4 border-b border-white/[0.08] flex items-center justify-between select-none">
          <h2 className="text-base font-medium text-[#e3e3e3]">Preferences & Settings</h2>
          <button
            onClick={onClose}
            className="w-8 h-8 rounded-full flex items-center justify-center text-[#8e918f] hover:text-[#e3e3e3] hover:bg-[#282a2c] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)]"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <form onSubmit={handleSave} className="p-6 space-y-5 overflow-y-auto flex-1 text-xs">
          {/* Auth status box */}
          <div className="p-4 rounded-2xl bg-[#131314] border border-white/[0.06] flex items-center justify-between">
            <div className="flex items-center space-x-3">
              {authStatus.is_authenticated ? (
                <ShieldCheck className="w-5 h-5 text-[#6dd58c]" />
              ) : (
                <AlertCircle className="w-5 h-5 text-[#ffb74d]" />
              )}
              <div>
                <p className="font-medium text-[#e3e3e3]">
                  {authStatus.is_authenticated ? "Authenticated with UGent" : "Authentication Required"}
                </p>
                <p className="text-[11px] text-[#8e918f]">
                  {authStatus.user_id ? `Student / User ID: ${authStatus.user_id}` : authStatus.message}
                </p>
              </div>
            </div>

            <div className="flex items-center space-x-2">
              <span
                className={`text-[11px] font-mono px-2.5 py-0.5 rounded-full font-medium ${
                  authStatus.is_authenticated
                    ? "bg-[#6dd58c]/15 text-[#6dd58c]"
                    : "bg-[#ffb74d]/15 text-[#ffb74d]"
                }`}
              >
                {authStatus.is_authenticated ? "Connected" : "Disconnected"}
              </span>
              {!authStatus.is_authenticated && onLogin && (
                <button
                  type="button"
                  onClick={onLogin}
                  className="flex items-center space-x-1 px-3 py-1 bg-[#a8c7fa] hover:bg-[#c2e7ff] text-[#062e6f] font-medium rounded-full text-xs transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
                >
                  <LogIn className="w-3 h-3" />
                  <span>Log In</span>
                </button>
              )}
            </div>
          </div>

          {/* Sync Folder */}
          <div className="space-y-2">
            <label className="font-medium text-[#c4c7c5]">Local Sync Directory</label>
            <div className="flex items-center space-x-2">
              <input
                type="text"
                value={syncDir}
                onChange={(e) => setSyncDir(e.target.value)}
                className="flex-1 bg-[#131314] border border-white/[0.08] focus:border-[#a8c7fa] focus:outline-none rounded-xl px-3.5 py-2.5 text-[#e3e3e3] font-mono text-xs transition duration-150"
                required
              />
              <button
                type="button"
                onClick={handlePickFolder}
                className="flex items-center space-x-1.5 px-4 py-2 bg-[#282a2c] hover:bg-[#37393b] border border-white/[0.08] rounded-full text-[#e3e3e3] font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
              >
                <Folder className="w-3.5 h-3.5 text-[#a8c7fa]" />
                <span>Browse</span>
              </button>
            </div>
          </div>

          {/* Interval */}
          <div className="space-y-2">
            <label className="font-medium text-[#c4c7c5]">Sync Interval</label>
            <select
              value={interval}
              onChange={(e) => setInterval(Number(e.target.value))}
              className="w-full bg-[#131314] border border-white/[0.08] focus:border-[#a8c7fa] focus:outline-none rounded-xl px-3.5 py-2.5 text-[#e3e3e3] text-xs transition duration-150"
            >
              <option value={15}>Every 15 minutes</option>
              <option value={30}>Every 30 minutes (recommended)</option>
              <option value={60}>Every 1 hour</option>
              <option value={120}>Every 2 hours</option>
              <option value={240}>Every 4 hours</option>
            </select>
          </div>

          {/* Conflict Strategy */}
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <label className="font-medium text-[#c4c7c5]">Conflict Strategy</label>
              <select
                value={conflictStrategy}
                onChange={(e) => setConflictStrategy(e.target.value)}
                className="w-full bg-[#131314] border border-white/[0.08] focus:border-[#a8c7fa] focus:outline-none rounded-xl px-3.5 py-2.5 text-[#e3e3e3] text-xs transition duration-150"
              >
                <option value="duplicate">Duplicate (stash edit as _edited)</option>
                <option value="skip">Skip (keep local edit untouched)</option>
                <option value="overwrite">Overwrite (replace with Ufora version)</option>
              </select>
            </div>

            <div className="space-y-2">
              <label className="font-medium text-[#c4c7c5]">Duplicate Suffix</label>
              <input
                type="text"
                value={suffix}
                onChange={(e) => setSuffix(e.target.value)}
                className="w-full bg-[#131314] border border-white/[0.08] focus:border-[#a8c7fa] focus:outline-none rounded-xl px-3.5 py-2.5 text-[#e3e3e3] font-mono text-xs transition duration-150"
                placeholder="_edited"
              />
            </div>
          </div>

          {/* Checkboxes */}
          <div className="space-y-3 pt-3 border-t border-white/[0.06]">
            <label className="flex items-center space-x-3 cursor-pointer select-none">
              <input
                type="checkbox"
                checked={syncDesc}
                onChange={(e) => setSyncDesc(e.target.checked)}
                className="w-4 h-4 rounded border-white/[0.12] bg-[#131314] text-[#a8c7fa] focus:ring-0 cursor-pointer"
              />
              <span className="text-[#c4c7c5]">Generate README.md summaries for folder & topic descriptions</span>
            </label>

            <label className="flex items-center space-x-3 cursor-pointer select-none">
              <input
                type="checkbox"
                checked={syncLinks}
                onChange={(e) => setSyncLinks(e.target.checked)}
                className="w-4 h-4 rounded border-white/[0.12] bg-[#131314] text-[#a8c7fa] focus:ring-0 cursor-pointer"
              />
              <span className="text-[#c4c7c5]">Generate clickable .html shortcut files for online activities</span>
            </label>

            <label className="flex items-center space-x-3 cursor-pointer select-none">
              <input
                type="checkbox"
                checked={autostart}
                onChange={(e) => setAutostart(e.target.checked)}
                className="w-4 h-4 rounded border-white/[0.12] bg-[#131314] text-[#a8c7fa] focus:ring-0 cursor-pointer"
              />
              <span className="text-[#c4c7c5]">Launch Ufora Sync automatically when logging into computer</span>
            </label>
          </div>

          {/* Footer */}
          <div className="pt-4 border-t border-white/[0.08] flex items-center justify-end space-x-2.5">
            <button
              type="button"
              onClick={onClose}
              className="px-5 py-2 rounded-full text-[#c4c7c5] hover:bg-[#282a2c] hover:text-[#e3e3e3] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] font-medium text-xs"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              className="flex items-center space-x-1.5 px-5 py-2 rounded-full bg-[#a8c7fa] hover:bg-[#c2e7ff] text-[#062e6f] font-medium text-xs transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98]"
            >
              {savedSuccess ? (
                <>
                  <Check className="w-3.5 h-3.5" />
                  <span>Saved</span>
                </>
              ) : saving ? (
                <span>Saving…</span>
              ) : (
                <span>Save preferences</span>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
