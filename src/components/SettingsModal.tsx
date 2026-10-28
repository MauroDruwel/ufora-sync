import { useState, type FormEvent } from "react";
import { AppConfig, AuthStatus } from "../types";
import { X, Folder, Save, CheckCircle2, ShieldCheck, AlertCircle } from "lucide-react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { enable as enableAutostart, disable as disableAutostart } from "@tauri-apps/plugin-autostart";

interface SettingsModalProps {
  config: AppConfig;
  authStatus: AuthStatus;
  isOpen: boolean;
  onClose: () => void;
  onSave: (newConfig: AppConfig) => Promise<void>;
}

export const SettingsModal = ({
  config,
  authStatus,
  isOpen,
  onClose,
  onSave,
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
    setSaving(true);
    try {
      // Autostart plugin toggle
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
    <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-[#101726] border border-slate-700/80 rounded-2xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
        {/* Header */}
        <div className="px-6 py-4 border-b border-slate-800 flex items-center justify-between">
          <h2 className="text-base font-semibold text-slate-100">Preferences & Settings</h2>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <form onSubmit={handleSave} className="p-6 space-y-5 overflow-y-auto flex-1 text-xs">
          {/* Auth status box */}
          <div className="p-3 rounded-xl bg-slate-900/80 border border-slate-800 flex items-center justify-between">
            <div className="flex items-center space-x-2.5">
              {authStatus.is_authenticated ? (
                <ShieldCheck className="w-5 h-5 text-emerald-400" />
              ) : (
                <AlertCircle className="w-5 h-5 text-rose-400" />
              )}
              <div>
                <p className="font-semibold text-slate-200">
                  {authStatus.is_authenticated ? "Authenticated with UGent" : "Authentication Required"}
                </p>
                <p className="text-[11px] text-slate-400">
                  {authStatus.user_id ? `Student / User ID: ${authStatus.user_id}` : authStatus.message}
                </p>
              </div>
            </div>
            <span
              className={`text-[10px] uppercase font-mono px-2 py-0.5 rounded font-semibold ${
                authStatus.is_authenticated
                  ? "bg-emerald-500/10 text-emerald-400 border border-emerald-500/30"
                  : "bg-rose-500/10 text-rose-400 border border-rose-500/30"
              }`}
            >
              {authStatus.is_authenticated ? "Connected" : "Disconnected"}
            </span>
          </div>

          {/* Sync Folder */}
          <div className="space-y-1.5">
            <label className="font-medium text-slate-300">Local Sync Directory</label>
            <div className="flex items-center space-x-2">
              <input
                type="text"
                value={syncDir}
                onChange={(e) => setSyncDir(e.target.value)}
                className="flex-1 bg-slate-900 border border-slate-700/80 rounded-lg px-3 py-2 text-slate-200 focus:outline-none focus:border-ugent-blue font-mono text-[11px]"
                required
              />
              <button
                type="button"
                onClick={handlePickFolder}
                className="flex items-center space-x-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-slate-200 font-medium transition"
              >
                <Folder className="w-3.5 h-3.5 text-ugent-blue" />
                <span>Browse</span>
              </button>
            </div>
          </div>

          {/* Interval */}
          <div className="space-y-1.5">
            <label className="font-medium text-slate-300">Sync Interval</label>
            <select
              value={interval}
              onChange={(e) => setInterval(Number(e.target.value))}
              className="w-full bg-slate-900 border border-slate-700/80 rounded-lg px-3 py-2 text-slate-200 focus:outline-none focus:border-ugent-blue text-xs"
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
            <div className="space-y-1.5">
              <label className="font-medium text-slate-300">Conflict Strategy</label>
              <select
                value={conflictStrategy}
                onChange={(e) => setConflictStrategy(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700/80 rounded-lg px-3 py-2 text-slate-200 focus:outline-none focus:border-ugent-blue text-xs"
              >
                <option value="duplicate">Duplicate (stash edit as _edited)</option>
                <option value="skip">Skip (keep local edit untouched)</option>
                <option value="overwrite">Overwrite (replace with Ufora version)</option>
              </select>
            </div>

            <div className="space-y-1.5">
              <label className="font-medium text-slate-300">Duplicate Suffix</label>
              <input
                type="text"
                value={suffix}
                onChange={(e) => setSuffix(e.target.value)}
                className="w-full bg-slate-900 border border-slate-700/80 rounded-lg px-3 py-2 text-slate-200 focus:outline-none focus:border-ugent-blue font-mono text-xs"
                placeholder="_edited"
              />
            </div>
          </div>

          {/* Checkboxes */}
          <div className="space-y-2.5 pt-2 border-t border-slate-800">
            <label className="flex items-center space-x-2.5 cursor-pointer">
              <input
                type="checkbox"
                checked={syncDesc}
                onChange={(e) => setSyncDesc(e.target.checked)}
                className="w-4 h-4 rounded border-slate-700 text-ugent-blue focus:ring-0 bg-slate-900"
              />
              <span className="text-slate-300">Generate README.md summaries for folder & topic descriptions</span>
            </label>

            <label className="flex items-center space-x-2.5 cursor-pointer">
              <input
                type="checkbox"
                checked={syncLinks}
                onChange={(e) => setSyncLinks(e.target.checked)}
                className="w-4 h-4 rounded border-slate-700 text-ugent-blue focus:ring-0 bg-slate-900"
              />
              <span className="text-slate-300">Generate clickable .html shortcut files for online activities</span>
            </label>

            <label className="flex items-center space-x-2.5 cursor-pointer">
              <input
                type="checkbox"
                checked={autostart}
                onChange={(e) => setAutostart(e.target.checked)}
                className="w-4 h-4 rounded border-slate-700 text-ugent-blue focus:ring-0 bg-slate-900"
              />
              <span className="text-slate-300">Launch Ufora Sync automatically when logging into computer</span>
            </label>
          </div>

          {/* Footer */}
          <div className="pt-4 border-t border-slate-800 flex items-center justify-end space-x-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition font-medium"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              className="flex items-center space-x-1.5 px-4 py-2 bg-ugent-blue hover:bg-blue-600 text-white font-medium rounded-lg shadow-lg shadow-blue-500/20 transition active:scale-95 disabled:opacity-50"
            >
              {savedSuccess ? (
                <>
                  <CheckCircle2 className="w-4 h-4 text-emerald-300" />
                  <span>Saved!</span>
                </>
              ) : (
                <>
                  <Save className="w-4 h-4" />
                  <span>{saving ? "Saving…" : "Save Settings"}</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
