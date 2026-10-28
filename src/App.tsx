import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Course, AppConfig, AuthStatus, LogMessage, SyncSummary } from "./types";
import { Header } from "./components/Header";
import { CourseCard } from "./components/CourseCard";
import { ActivityLog } from "./components/ActivityLog";
import { SettingsModal } from "./components/SettingsModal";
import { BookOpen, CheckSquare, Square, RefreshCcw } from "lucide-react";

export default function App() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [courses, setCourses] = useState<Course[]>([]);
  const [authStatus, setAuthStatus] = useState<AuthStatus>({
    is_authenticated: false,
    message: "Checking authentication…",
  });
  const [logs, setLogs] = useState<string[]>([]);
  const [syncStatus, setSyncStatus] = useState<"idle" | "syncing" | "paused" | "error">("idle");
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isLoadingCourses, setIsLoadingCourses] = useState(true);
  const [activeTab, setActiveTab] = useState<"courses" | "logs">("courses");

  // Load initial data
  useEffect(() => {
    async function init() {
      try {
        const loadedConfig: AppConfig = await invoke("get_config");
        setConfig(loadedConfig);

        const loadedAuth: AuthStatus = await invoke("get_auth_status");
        setAuthStatus(loadedAuth);

        const initialLogs: string[] = await invoke("get_recent_logs");
        setLogs(initialLogs);

        // Load courses
        try {
          const loadedCourses: Course[] = await invoke("list_courses");
          setCourses(loadedCourses);
        } catch (err) {
          console.error("Failed to load courses:", err);
        } finally {
          setIsLoadingCourses(false);
        }
      } catch (err) {
        console.error("Failed to initialize:", err);
      }
    }
    init();

    // Event listeners
    const unlistenLog = listen<LogMessage>("log-message", (event) => {
      setLogs((prev) => [...prev, `[${event.payload.timestamp}] ${event.payload.message}`]);
    });

    const unlistenStatus = listen<string>("sync-status", (event) => {
      const s = event.payload.toLowerCase();
      if (s === "syncing") setSyncStatus("syncing");
      else if (s === "paused") setSyncStatus("paused");
      else if (s === "error") setSyncStatus("error");
      else setSyncStatus("idle");
    });

    const unlistenComplete = listen<SyncSummary>("sync-complete", (_event) => {
      setSyncStatus("idle");
    });

    return () => {
      unlistenLog.then((f) => f());
      unlistenStatus.then((f) => f());
      unlistenComplete.then((f) => f());
    };
  }, []);

  const handleToggleCourse = async (courseId: string, enabled: boolean) => {
    if (!config) return;
    const set = new Set(config.enabled_courses);
    if (enabled) {
      set.add(courseId);
    } else {
      set.delete(courseId);
    }
    const updated = { ...config, enabled_courses: Array.from(set) };
    setConfig(updated);
    await invoke("save_config", { newCfg: updated });
  };

  const handleSelectAll = async (enable: boolean) => {
    if (!config) return;
    const newEnabled = enable ? courses.map((c) => c.id) : [];
    const updated = { ...config, enabled_courses: newEnabled };
    setConfig(updated);
    await invoke("save_config", { newCfg: updated });
  };

  const handleSyncNow = async () => {
    try {
      await invoke("sync_now");
    } catch (err) {
      console.error("Failed to trigger sync:", err);
    }
  };

  const handleTogglePause = async () => {
    if (syncStatus === "paused") {
      await invoke("resume_sync");
      setSyncStatus("idle");
    } else {
      await invoke("pause_sync");
      setSyncStatus("paused");
    }
  };

  const handleSaveSettings = async (newConfig: AppConfig) => {
    setConfig(newConfig);
    await invoke("save_config", { newCfg: newConfig });
  };

  const handleOpenSyncFolder = async () => {
    if (config) {
      await invoke("open_folder", { path: config.sync_dir });
    }
  };

  const handleOpenCourseFolder = async (folderName: string) => {
    await invoke("open_course_folder", { folderName });
  };

  const handleOpenUrl = async (url: string) => {
    await invoke("open_url", { url });
  };

  if (!config) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#0b1120] text-slate-400 font-mono text-sm">
        Loading Ufora Sync…
      </div>
    );
  }

  const enabledCount = courses.filter((c) => config.enabled_courses.includes(c.id)).length;

  return (
    <div className="flex flex-col h-screen bg-[#0b1120] text-slate-100 select-none overflow-hidden">
      {/* Top Header */}
      <Header
        status={syncStatus}
        onSyncNow={handleSyncNow}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onOpenSyncFolder={handleOpenSyncFolder}
        onOpenUforaWeb={() => handleOpenUrl("https://ufora.ugent.be")}
      />

      {/* Tabs */}
      <div className="flex items-center justify-between px-6 pt-3 pb-2 border-b border-slate-800/60 bg-[#0d1424]">
        <div className="flex items-center space-x-2">
          <button
            onClick={() => setActiveTab("courses")}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition ${
              activeTab === "courses"
                ? "bg-ugent-blue text-white shadow-sm"
                : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/60"
            }`}
          >
            Courses ({courses.length})
          </button>
          <button
            onClick={() => setActiveTab("logs")}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition flex items-center space-x-1.5 ${
              activeTab === "logs"
                ? "bg-ugent-blue text-white shadow-sm"
                : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/60"
            }`}
          >
            <span>Activity Log</span>
            {syncStatus === "syncing" && (
              <span className="w-2 h-2 rounded-full bg-blue-300 animate-ping" />
            )}
          </button>
        </div>

        {activeTab === "courses" && (
          <div className="flex items-center space-x-3 text-xs text-slate-400">
            <span>
              <strong className="text-slate-200">{enabledCount}</strong> of {courses.length} active
            </span>
            <div className="flex items-center space-x-1 border-l border-slate-700/60 pl-3">
              <button
                onClick={() => handleSelectAll(true)}
                className="flex items-center space-x-1 hover:text-slate-200 px-1.5 py-0.5 rounded hover:bg-slate-800 transition"
                title="Enable all courses"
              >
                <CheckSquare className="w-3.5 h-3.5 text-ugent-blue" />
                <span>All</span>
              </button>
              <button
                onClick={() => handleSelectAll(false)}
                className="flex items-center space-x-1 hover:text-slate-200 px-1.5 py-0.5 rounded hover:bg-slate-800 transition"
                title="Disable all courses"
              >
                <Square className="w-3.5 h-3.5" />
                <span>None</span>
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Main Body */}
      <main className="flex-1 overflow-hidden p-6">
        {activeTab === "courses" ? (
          <div className="h-full overflow-y-auto space-y-2 pr-1">
            {isLoadingCourses ? (
              <div className="flex flex-col items-center justify-center h-64 space-y-3 text-slate-400">
                <RefreshCcw className="w-6 h-6 animate-spin text-ugent-blue" />
                <span className="text-xs">Fetching enrolled courses from UGent Ufora…</span>
              </div>
            ) : courses.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-64 space-y-2 text-slate-400">
                <BookOpen className="w-8 h-8 text-slate-600" />
                <span className="text-sm font-medium text-slate-300">No courses loaded</span>
                <span className="text-xs text-slate-500">
                  Make sure you are logged into Ufora and have active enrollments.
                </span>
              </div>
            ) : (
              courses.map((course) => (
                <CourseCard
                  key={course.id}
                  course={course}
                  isEnabled={config.enabled_courses.length === 0 || config.enabled_courses.includes(course.id)}
                  onToggle={handleToggleCourse}
                  onOpenFolder={handleOpenCourseFolder}
                  onOpenUrl={handleOpenUrl}
                />
              ))
            )}
          </div>
        ) : (
          <ActivityLog
            logs={logs}
            isPaused={syncStatus === "paused"}
            onClear={() => setLogs([])}
            onTogglePause={handleTogglePause}
          />
        )}
      </main>

      {/* Settings Modal */}
      <SettingsModal
        config={config}
        authStatus={authStatus}
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        onSave={handleSaveSettings}
      />
    </div>
  );
}
