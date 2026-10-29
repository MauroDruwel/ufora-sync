import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Course, AppConfig, AuthStatus, LogMessage, SyncSummary } from "./types";
import { Header } from "./components/Header";
import { CourseCard } from "./components/CourseCard";
import { ActivityLog } from "./components/ActivityLog";
import { SettingsModal } from "./components/SettingsModal";
import {
  BookOpen,
  CheckSquare,
  Square,
  RefreshCcw,
  LogIn,
  LayoutGrid,
  List,
  Search,
  X,
  SlidersHorizontal,
  ChevronDown,
  ArrowLeftRight,
  AlertTriangle,
} from "lucide-react";

interface ConfirmDialogState {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText: string;
  variant: "primary" | "danger";
  action: () => Promise<void>;
}

type CourseCategory = "current" | "all_courses" | "groups" | "all";

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
  const [courseCategory, setCourseCategory] = useState<CourseCategory>("current");
  const [searchQuery, setSearchQuery] = useState("");
  const [isSelectionMenuOpen, setIsSelectionMenuOpen] = useState(false);
  const [confirmDialog, setConfirmDialog] = useState<ConfirmDialogState | null>(null);

  const [viewMode, setViewMode] = useState<"grid" | "list">(() => {
    if (typeof window !== "undefined") {
      const saved = localStorage.getItem("ufora_view_mode");
      if (saved === "list" || saved === "grid") return saved;
    }
    return "grid";
  });

  const toggleViewMode = (mode: "grid" | "list") => {
    setViewMode(mode);
    if (typeof window !== "undefined") {
      localStorage.setItem("ufora_view_mode", mode);
    }
  };

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

        // Load courses if authenticated
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

    const unlistenStatus = listen<string>("sync-status", async (event) => {
      const s = event.payload.toLowerCase();
      if (s === "syncing") setSyncStatus("syncing");
      else if (s === "paused") setSyncStatus("paused");
      else if (s === "error") setSyncStatus("error");
      else setSyncStatus("idle");

      try {
        const updatedConfig: AppConfig = await invoke("get_config");
        setConfig(updatedConfig);
      } catch (e) {
        console.error("Failed to reload config on status update:", e);
      }
    });

    const unlistenComplete = listen<SyncSummary>("sync-complete", async (_event) => {
      setSyncStatus("idle");
      try {
        const updatedConfig: AppConfig = await invoke("get_config");
        setConfig(updatedConfig);
      } catch (e) {
        console.error("Failed to reload config on sync complete:", e);
      }
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
        } else {
          setCourses([]);
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

  const currentAcademicYear = (() => {
    const now = new Date();
    // UGent academic year starts in September (month 8, 0-indexed)
    return now.getMonth() >= 8 ? now.getFullYear() : now.getFullYear() - 1;
  })();

  const currentCourses = courses.filter(
    (c) => c.is_course_offering !== false && (c.academic_year === currentAcademicYear || !c.academic_year)
  );
  const allCourseOfferings = courses.filter((c) => c.is_course_offering !== false);
  const groupCourses = courses.filter((c) => c.is_course_offering === false);

  const categoryFilteredCourses = courses.filter((c) => {
    if (courseCategory === "current") {
      return c.is_course_offering !== false && (c.academic_year === currentAcademicYear || !c.academic_year);
    }
    if (courseCategory === "all_courses") {
      return c.is_course_offering !== false;
    }
    if (courseCategory === "groups") {
      return c.is_course_offering === false;
    }
    return true;
  });

  const filteredCourses = categoryFilteredCourses.filter((c) => {
    const q = searchQuery.toLowerCase().trim();
    if (!q) return true;
    return (
      c.name.toLowerCase().includes(q) ||
      c.code.toLowerCase().includes(q) ||
      c.id.toLowerCase().includes(q)
    );
  });

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

  const handleSelectVisible = async (enable: boolean) => {
    if (!config) return;
    const set = new Set(config.enabled_courses);
    for (const c of filteredCourses) {
      if (enable) {
        set.add(c.id);
      } else {
        set.delete(c.id);
      }
    }
    const updated = { ...config, enabled_courses: Array.from(set) };
    setConfig(updated);
    await invoke("save_config", { newCfg: updated });
  };

  const handleInvertVisible = async () => {
    if (!config) return;
    const currentSet = new Set(config.enabled_courses);
    for (const c of filteredCourses) {
      if (currentSet.has(c.id)) {
        currentSet.delete(c.id);
      } else {
        currentSet.add(c.id);
      }
    }
    const updated = { ...config, enabled_courses: Array.from(currentSet) };
    setConfig(updated);
    await invoke("save_config", { newCfg: updated });
  };

  const promptSelectVisible = () => {
    const count = filteredCourses.length;
    setConfirmDialog({
      isOpen: true,
      title: "Enable visible items?",
      message: `Are you sure you want to enable background synchronization for ${count} visible course(s)?`,
      confirmText: `Enable ${count} Items`,
      variant: "primary",
      action: async () => {
        await handleSelectVisible(true);
      },
    });
  };

  const promptDeselectVisible = () => {
    const count = filteredCourses.length;
    setConfirmDialog({
      isOpen: true,
      title: "Disable visible items?",
      message: `Are you sure you want to disable synchronization for ${count} visible course(s)? No local files will be deleted.`,
      confirmText: `Disable ${count} Items`,
      variant: "danger",
      action: async () => {
        await handleSelectVisible(false);
      },
    });
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

  const handleLogout = async () => {
    try {
      await invoke("logout");
      setAuthStatus({
        is_authenticated: false,
        message: "Logged out.",
      });
      setCourses([]);
    } catch (err) {
      console.error("Failed to log out:", err);
    }
  };

  if (!config) {
    return (
      <div className="flex h-screen items-center justify-center bg-[#ffffff] dark:bg-[#131314] text-[#747775] dark:text-[#8e918f] font-sans text-xs">
        Loading Ufora Sync…
      </div>
    );
  }

  const enabledVisibleCount = filteredCourses.filter((c) =>
    config.enabled_courses.length === 0 || config.enabled_courses.includes(c.id)
  ).length;

  return (
    <div className="flex flex-col h-screen bg-[#ffffff] dark:bg-[#131314] text-[#1f1f1f] dark:text-[#e3e3e3] font-sans select-none overflow-hidden transition-colors duration-150">
      {/* Top Header with traffic light safe area & dynamic status */}
      <Header
        status={syncStatus}
        lastSyncTime={config.last_sync_time}
        lastSyncStatus={config.last_sync_status}
        isAuthenticated={authStatus.is_authenticated}
        onSyncNow={handleSyncNow}
        onLogin={handleOpenLogin}
        onOpenSettings={() => setIsSettingsOpen(true)}
        onOpenSyncFolder={handleOpenSyncFolder}
        onOpenUforaWeb={() => handleOpenUrl("https://ufora.ugent.be")}
      />

      {/* Tabs & Controls */}
      <div className="flex items-center justify-between px-6 py-2.5 border-b border-black/[0.08] dark:border-white/[0.08] bg-[#ffffff] dark:bg-[#131314] transition-colors duration-150">
        <div className="flex items-center space-x-3">
          {/* Main View Tabs */}
          <div className="flex items-center space-x-1">
            <button
              onClick={() => setActiveTab("courses")}
              className={`px-3 py-1.5 rounded-full text-xs font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] ${
                activeTab === "courses"
                  ? "bg-[#e9eef6] dark:bg-[#282a2c] text-[#1f1f1f] dark:text-[#e3e3e3]"
                  : "text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#c4c7c5] hover:bg-[#f0f4f9] dark:hover:bg-[#1e1f20]"
              }`}
            >
              Courses
            </button>
            <button
              onClick={() => setActiveTab("logs")}
              className={`px-3 py-1.5 rounded-full text-xs font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] flex items-center space-x-2 ${
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

          {/* Academic Year & Category Filter Pills (Filters out 60 past years and group placeholders) */}
          {activeTab === "courses" && courses.length > 0 && (
            <div className="flex items-center p-0.5 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08] text-xs">
              <button
                onClick={() => setCourseCategory("current")}
                className={`px-3 py-1 rounded-full font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] ${
                  courseCategory === "current"
                    ? "bg-[#ffffff] dark:bg-[#282a2c] text-[#1a73e8] dark:text-[#a8c7fa] shadow-sm"
                    : "text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3]"
                }`}
                title="Active courses for current academic year (2026-2027)"
              >
                Current ({currentCourses.length})
              </button>
              <button
                onClick={() => setCourseCategory("all_courses")}
                className={`px-3 py-1 rounded-full font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] ${
                  courseCategory === "all_courses"
                    ? "bg-[#ffffff] dark:bg-[#282a2c] text-[#1a73e8] dark:text-[#a8c7fa] shadow-sm"
                    : "text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3]"
                }`}
                title="All course offerings including past academic years"
              >
                All Courses ({allCourseOfferings.length})
              </button>
              {groupCourses.length > 0 && (
                <button
                  onClick={() => setCourseCategory("groups")}
                  className={`px-3 py-1 rounded-full font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] ${
                    courseCategory === "groups"
                      ? "bg-[#ffffff] dark:bg-[#282a2c] text-[#1a73e8] dark:text-[#a8c7fa] shadow-sm"
                      : "text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3]"
                  }`}
                  title="Project groups and organizational units"
                >
                  Groups ({groupCourses.length})
                </button>
              )}
            </div>
          )}
        </div>

        {activeTab === "courses" && courses.length > 0 && (
          <div className="flex items-center space-x-3 text-xs text-[#747775] dark:text-[#8e918f]">
            {/* Search input */}
            <div className="relative flex items-center">
              <Search className="w-3.5 h-3.5 absolute left-2.5 text-[#747775] dark:text-[#8e918f] pointer-events-none" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Filter courses…"
                className="pl-8 pr-7 py-1 text-xs rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] focus:bg-[#ffffff] dark:focus:bg-[#131314] border border-black/[0.08] dark:border-white/[0.08] focus:border-[#1a73e8] dark:focus:border-[#a8c7fa] focus:outline-none text-[#1f1f1f] dark:text-[#e3e3e3] placeholder-[#747775] dark:placeholder-[#8e918f] w-36 sm:w-44 transition duration-150"
              />
              {searchQuery && (
                <button
                  onClick={() => setSearchQuery("")}
                  className="absolute right-2 text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3]"
                  title="Clear filter"
                >
                  <X className="w-3 h-3" />
                </button>
              )}
            </div>

            {/* View Mode Toggle */}
            <div className="flex items-center p-0.5 rounded-full bg-[#f0f4f9] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08]">
              <button
                onClick={() => toggleViewMode("grid")}
                className={`p-1.5 rounded-full transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] ${
                  viewMode === "grid"
                    ? "bg-[#ffffff] dark:bg-[#282a2c] text-[#1a73e8] dark:text-[#a8c7fa] shadow-sm"
                    : "text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3]"
                }`}
                title="Grid view"
              >
                <LayoutGrid className="w-3.5 h-3.5" />
              </button>
              <button
                onClick={() => toggleViewMode("list")}
                className={`p-1.5 rounded-full transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] ${
                  viewMode === "list"
                    ? "bg-[#ffffff] dark:bg-[#282a2c] text-[#1a73e8] dark:text-[#a8c7fa] shadow-sm"
                    : "text-[#747775] dark:text-[#8e918f] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3]"
                }`}
                title="List view"
              >
                <List className="w-3.5 h-3.5" />
              </button>
            </div>

            {/* Safe Batch Selection Menu */}
            <div className="relative border-l border-black/[0.08] dark:border-white/[0.08] pl-3">
              <button
                onClick={() => setIsSelectionMenuOpen((prev) => !prev)}
                className="flex items-center space-x-1.5 px-3 py-1 rounded-full text-xs font-medium bg-[#f0f4f9] dark:bg-[#1e1f20] hover:bg-[#e9eef6] dark:hover:bg-[#282a2c] text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] border border-black/[0.08] dark:border-white/[0.08] transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96]"
                title="Manage course synchronization selection"
              >
                <SlidersHorizontal className="w-3.5 h-3.5 text-[#1a73e8] dark:text-[#a8c7fa]" />
                <span>
                  <strong className="text-[#1f1f1f] dark:text-[#e3e3e3] font-medium">{enabledVisibleCount}</strong>/
                  {filteredCourses.length} active
                </span>
                <ChevronDown className="w-3 h-3 text-[#747775] dark:text-[#8e918f]" />
              </button>

              {/* Selection Dropdown */}
              {isSelectionMenuOpen && (
                <>
                  <div
                    className="fixed inset-0 z-40"
                    onClick={() => setIsSelectionMenuOpen(false)}
                  />
                  <div className="absolute right-0 mt-1.5 w-52 bg-[#ffffff] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08] rounded-2xl shadow-xl z-50 p-1.5 text-xs animate-in fade-in zoom-in-95 duration-100">
                    <button
                      onClick={() => {
                        setIsSelectionMenuOpen(false);
                        promptSelectVisible();
                      }}
                      className="w-full flex items-center space-x-2.5 px-3 py-2 rounded-xl text-left text-[#1f1f1f] dark:text-[#e3e3e3] hover:bg-[#f0f4f9] dark:hover:bg-[#282a2c] transition duration-150"
                    >
                      <CheckSquare className="w-4 h-4 text-[#1a73e8] dark:text-[#a8c7fa]" />
                      <span>Enable Visible ({filteredCourses.length})</span>
                    </button>
                    <button
                      onClick={() => {
                        setIsSelectionMenuOpen(false);
                        promptDeselectVisible();
                      }}
                      className="w-full flex items-center space-x-2.5 px-3 py-2 rounded-xl text-left text-[#b3261e] dark:text-[#f2b8b5] hover:bg-[#b3261e]/10 dark:hover:bg-[#b3261e]/15 transition duration-150"
                    >
                      <Square className="w-4 h-4" />
                      <span>Disable Visible</span>
                    </button>
                    <div className="my-1 border-t border-black/[0.06] dark:border-white/[0.06]" />
                    <button
                      onClick={() => {
                        setIsSelectionMenuOpen(false);
                        handleInvertVisible();
                      }}
                      className="w-full flex items-center space-x-2.5 px-3 py-2 rounded-xl text-left text-[#444746] dark:text-[#c4c7c5] hover:bg-[#f0f4f9] dark:hover:bg-[#282a2c] transition duration-150"
                    >
                      <ArrowLeftRight className="w-4 h-4 text-[#747775] dark:text-[#8e918f]" />
                      <span>Invert Visible</span>
                    </button>
                  </div>
                </>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Main Body */}
      <main className="flex-1 overflow-hidden p-6">
        {activeTab === "courses" ? (
          <div className="h-full overflow-y-auto pr-1">
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
            ) : filteredCourses.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-64 space-y-3 text-[#747775] dark:text-[#8e918f] max-w-sm mx-auto p-6 rounded-3xl bg-[#f0f4f9] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08] text-center">
                <Search className="w-6 h-6 text-[#747775] dark:text-[#8e918f]" />
                <div className="space-y-1">
                  <span className="text-sm font-medium text-[#1f1f1f] dark:text-[#e3e3e3] block">
                    No matching courses
                  </span>
                  <span className="text-xs text-[#747775] dark:text-[#8e918f] block">
                    No courses match "{searchQuery}" in this view.
                  </span>
                </div>
                <div className="flex items-center space-x-2">
                  {searchQuery && (
                    <button
                      onClick={() => setSearchQuery("")}
                      className="px-3.5 py-1.5 rounded-full text-xs font-medium bg-[#ffffff] dark:bg-[#282a2c] hover:bg-[#e9eef6] dark:hover:bg-[#37393b] text-[#1a73e8] dark:text-[#a8c7fa] border border-black/[0.08] dark:border-white/[0.08] transition duration-150 active:scale-[0.96]"
                    >
                      Clear search
                    </button>
                  )}
                  {courseCategory !== "all_courses" && (
                    <button
                      onClick={() => setCourseCategory("all_courses")}
                      className="px-3.5 py-1.5 rounded-full text-xs font-medium bg-[#ffffff] dark:bg-[#282a2c] hover:bg-[#e9eef6] dark:hover:bg-[#37393b] text-[#444746] dark:text-[#c4c7c5] border border-black/[0.08] dark:border-white/[0.08] transition duration-150 active:scale-[0.96]"
                    >
                      View All Courses
                    </button>
                  )}
                </div>
              </div>
            ) : (
              <div
                className={
                  viewMode === "grid"
                    ? "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3"
                    : "space-y-2.5"
                }
              >
                {filteredCourses.map((course) => (
                  <CourseCard
                    key={course.id}
                    course={course}
                    viewMode={viewMode}
                    isEnabled={config.enabled_courses.length === 0 || config.enabled_courses.includes(course.id)}
                    onToggle={handleToggleCourse}
                    onOpenFolder={handleOpenCourseFolder}
                    onOpenUrl={handleOpenUrl}
                  />
                ))}
              </div>
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

      {/* Confirmation Modal */}
      {confirmDialog && confirmDialog.isOpen && (
        <div className="fixed inset-0 z-50 bg-black/40 dark:bg-black/60 flex items-center justify-center p-4 backdrop-blur-sm animate-in fade-in duration-150">
          <div className="bg-[#ffffff] dark:bg-[#1e1f20] border border-black/[0.08] dark:border-white/[0.08] rounded-3xl p-6 max-w-sm w-full shadow-2xl space-y-4">
            <div className="flex items-center space-x-3">
              <div
                className={`w-10 h-10 rounded-full flex items-center justify-center shrink-0 ${
                  confirmDialog.variant === "danger"
                    ? "bg-[#b3261e]/10 text-[#b3261e] dark:text-[#f2b8b5]"
                    : "bg-[#1a73e8]/10 text-[#1a73e8] dark:text-[#a8c7fa]"
                }`}
              >
                {confirmDialog.variant === "danger" ? (
                  <AlertTriangle className="w-5 h-5" />
                ) : (
                  <CheckSquare className="w-5 h-5" />
                )}
              </div>
              <div>
                <h3 className="font-medium text-sm text-[#1f1f1f] dark:text-[#e3e3e3]">
                  {confirmDialog.title}
                </h3>
                <p className="text-[11px] text-[#747775] dark:text-[#8e918f]">
                  Course Selection
                </p>
              </div>
            </div>

            <p className="text-xs text-[#444746] dark:text-[#c4c7c5] leading-relaxed">
              {confirmDialog.message}
            </p>

            <div className="flex items-center justify-end space-x-2 pt-2">
              <button
                type="button"
                onClick={() => setConfirmDialog(null)}
                className="px-4 py-2 rounded-full text-xs font-medium text-[#444746] dark:text-[#c4c7c5] hover:text-[#1f1f1f] dark:hover:text-[#e3e3e3] hover:bg-[#f0f4f9] dark:hover:bg-[#282a2c] transition duration-150"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={async () => {
                  const action = confirmDialog.action;
                  setConfirmDialog(null);
                  await action();
                }}
                className={`px-4 py-2 rounded-full text-xs font-medium transition duration-150 ease-[cubic-bezier(0.2,0,0,1)] active:scale-[0.96] ${
                  confirmDialog.variant === "danger"
                    ? "bg-[#b3261e] hover:bg-[#8c1d18] text-white"
                    : "bg-[#1a73e8] dark:bg-[#a8c7fa] hover:bg-[#1765cc] dark:hover:bg-[#c2e7ff] text-white dark:text-[#062e6f]"
                }`}
              >
                {confirmDialog.confirmText}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Settings Modal */}
      <SettingsModal
        config={config}
        authStatus={authStatus}
        isOpen={isSettingsOpen}
        onClose={() => setIsSettingsOpen(false)}
        onSave={handleSaveSettings}
        onLogin={handleOpenLogin}
        onLogout={handleLogout}
      />
    </div>
  );
}
