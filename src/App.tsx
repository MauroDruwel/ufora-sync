import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Course, AppConfig, AuthStatus, LogMessage, SyncSummary } from "./types";
import { Header } from "./components/Header";
import { CourseCard } from "./components/CourseCard";
import { ActivityLog } from "./components/ActivityLog";
import { SettingsModal } from "./components/SettingsModal";
import { BookOpen, CheckSquare, Square, RefreshCcw, LogIn } from "lucide-react";

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

  // Dynamic OS Dark/Light theme sync
  useEffect(() => {
    if (typeof window !== "undefined" && window.matchMedia) {
      const media = window.matchMedia("(prefers-color-scheme: dark)");
      const updateTheme = (matches: boolean) => {
        if (matches) {
          document.documentElement.classList.add("dark");
        } else {
          document.documentElement.classList.remove("dark");
        }
      };
      updateTheme(media.matches);
      const handler = (e: MediaQueryListEvent) => updateTheme(e.matches);
      media.addEventListener("change", handler);
      return () => media.removeEventListener("change", handler);
    }
  }, []);

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

    const unlistenAuth = listen<boolean>("auth-status-changed", async (_event) => {
      try {
        const loadedAuth: AuthStatus = await invoke("get_auth_status");
        setAuthStatus(loadedAuth);
        if (loadedAuth.is_authenticated) {
          setIsLoadingCourses(true);
          const loadedCourses: Course[] = await invoke("list_courses");
          setCourses(loadedCourses);
          await invoke("sync_now");
        }
      } catch (err) {
        console.error("Failed to update auth status:", err);
      } finally {
        setIsLoadingCourses(false);
      }
    });

    return () => {
      unlistenLog.then((f) => f());
      unlistenStatus.then((f) => f());
      unlistenComplete.then((f) => f());
      unlistenAuth.then((f) => f());
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
    const updated = {
      ...config,
      enabled_courses: enable ? courses.map((c) => c.id) : [],
    };
    setConfig(updated);
    await invoke("save_config", { newCfg: updated });
  };

  const handleSaveSettings = async (newCfg: AppConfig) => {
    setConfig(newCfg);
    await invoke("save_config", { newCfg });
  };

  const handleSyncNow = async () => {
    try {
      await invoke("sync_now");
    } catch (err) {
      console.error("Failed to sync now:", err);
    }
  };

  const handleTogglePause = async () => {
    try {
      if (syncStatus === "paused") {
        await invoke("resume_sync");
        setSyncStatus("idle");
      } else {
        await invoke("pause_sync");
        setSyncStatus("paused");
      }
    } catch (err) {
      console.error("Failed to toggle pause:", err);
    }
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

  const handleOpenLogin = async () => {
    try {
      await invoke("open_login");
    } catch (err) {
      console.error("Failed to open login window:", err);
    }
  };

  if (!config) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#ffffff] dark:bg-[#131314] text-[#747775] dark:text-[#8e918f] font-sans text-xs">
        Loading Ufora Sync…
      </div>
    );
  }

  const enabledCount = courses.filter((c) => config.enabled_courses.includes(c.id)).length;

  return (
    <div className="flex flex-col h-screen bg-[#ffffff] dark:bg-[#131314] text-[#1f1f1f] dark:text-[#e3e3e3] font-sans select-none overflow-hidden transition-colors duration-150">
      {/* Top Header with traffic light safe area */}
      <Header
        status={syncStatus}
        isAuthenticated={authStatus.is_authenticated}
        onSyncNow={handleSyncNow}
        onLogin={handleOpenLogin}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onOpenSyncFolder={handleOpenSyncFolder}
        onOpenUforaWeb={() => handleOpenUrl("https://ufora.ugent.be")}
      />

      {/* Tabs & Controls */}
      <div className="flex items-center justify-between px-6 py-2.5 border-b border-black/[0.08] dark:border-white/[0.08] bg-[#ffffff] dark:bg-[#131314] transition-colors duration-150">
        <div className="flex items-center space-x-1.5">
          <button
            onClick={() => setActiveTab("courses")}
            className={`px-3.5 py-1.5 rounded-full text-xs font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] ${
              activeTab === "courses"
                ? "bg-[#e9eef6] dark:bg-[#282a2c] text-[#1f1f1f] dark:text-[#e3e3e3]"
                : "text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#c4c7c5] hover:bg-[#f0f4f9] dark:hover:bg-[#1e1f20]"
            }`}
          >
            Courses ({courses.length})
          </button>
          <button
            onClick={() => setActiveTab("logs")}
            className={`px-3.5 py-1.5 rounded-full text-xs font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] flex items-center space-x-2 ${
              activeTab === "logs"
                ? "bg-[#e9eef6] dark:bg-[#282a2c] text-[#1f1f1f] dark:text-[#e3e3e3]"
                : "text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#c4c7c5] hover:bg-[#f0f4f9] dark:hover:bg-[#1e1f20]"
            }`}
          >
            <span>Activity Log</span>
            {syncStatus === "syncing" && (
              <span className="w-1.5 h-1.5 rounded-full bg-[#1a73e8] dark:bg-[#a8c7fa] animate-ping" />
            )}
          </button>
        </div>

        {activeTab === "courses" && (
          <div className="flex items-center space-x-3 text-xs text-[#747775] dark:text-[#8e918f]">
            <span>
              <strong className="text-[#1f1f1f] dark:text-[#c4c7c5] font-medium">{enabledCount}</strong> of{" "}
              {courses.length} active
            </span>
            <div className="flex items-center space-x-1.5 border-l border-black/[0.08] dark:border-white/[0.08] pl-3">
              <button
                onClick={() => handleSelectAll(true)}
                className="flex items-center space-x-1 px-2.5 py-1 rounded-full text-xs font-medium bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.06] dark:border-white/[0.06] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
                title="Enable all courses"
              >
                <CheckSquare className="w-3.5 h-3.5 text-[#1a73e8] dark:text-[#a8c7fa]" />
                <span>All</span>
              </button>
              <button
                onClick={() => handleSelectAll(false)}
                className="flex items-center space-x-1 px-2.5 py-1 rounded-full text-xs font-medium bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.06] dark:border-white/[0.06] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
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
          <div className="h-full overflow-y-auto space-y-2.5 pr-1">
            {isLoadingCourses ? (
              <div className="flex flex-col items-center justify-center h-64 space-y-3 text-[#747775] dark:text-[#8e918f]">
                <RefreshCcw className="w-5 h-5 animate-spin text-[#1a73e8] dark:text-[#a8c7fa]" />
                <span className="text-xs">Fetching enrolled courses from UGent Ufora…</span>
              </div>
            ) : courses.length === 0 ? (
              !authStatus.is_authenticated ? (
                <div className="flex flex-col items-center justify-center h-80 space-y-4 text-center max-w-sm mx-auto p-6 rounded-3xl bg-[#f0f4f9] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08]">
                  <div className="w-12 h-12 rounded-full bg-[#ffffff] dark:bg-[#282a2c] flex items-center justify-center text-[#1a73e8] dark:text-[#a8c7fa] shadow-sm">
                    <LogIn className="w-5 h-5" />
                  </div>
                  <div className="space-y-1">
                    <h3 className="font-medium text-sm text-[#1f1f1f] dark:text-[#e3e3e3]">
                      Authentication Required
                    </h3>
                    <p className="text-xs text-[#747775] dark:text-[#8e918f] leading-relaxed">
                      {authStatus.message || "Sign in with your UGent account to fetch and sync course materials."}
                    </p>
                  </div>
                  <button
                    onClick={handleOpenLogin}
                    className="flex items-center space-x-2 px-5 py-2.5 bg-[#1a73e8] dark:bg-[#a8c7fa] hover:bg-[#1765cc] dark:hover:bg-[#c2e7ff] text-[#ffffff] dark:text-[#062e6f] font-medium rounded-full text-xs transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.98]"
                  >
                    <LogIn className="w-4 h-4" />
                    <span>Log in with UGent Ufora</span>
                  </button>
                </div>
              ) : (
                <div className="flex flex-col items-center justify-center h-64 space-y-3 text-[#747775] dark:text-[#8e918f] max-w-sm mx-auto p-6 rounded-3xl bg-[#f0f4f9] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08] text-center">
                  <BookOpen className="w-6 h-6 text-[#747775] dark:text-[#8e918f]" />
                  <div className="space-y-1">
                    <span className="text-sm font-medium text-[#1f1f1f] dark:text-[#e3e3e3] block">
                      No courses found
                    </span>
                    <span className="text-xs text-[#747775] dark:text-[#8e918f] block">
                      Make sure you have active course enrollments on Ufora.
                    </span>
                  </div>
                </div>
              )
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
        onLogin={handleOpenLogin}
      />
    </div>
  );
}
