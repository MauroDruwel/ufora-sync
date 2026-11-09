import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import type { AppConfig, Course } from "../src/types";

const courses: Course[] = [
  { id: "1", name: "Algorithms", code: "CS101", home_url: "/d2l/home/1", folder_name: "Algorithms" },
  { id: "2", name: "Databases", code: "CS102", home_url: "/d2l/home/2", folder_name: "Databases" },
];
const initialConfig: AppConfig = {
  sync_dir: "/tmp/Ufora",
  interval_minutes: 30,
  enabled_courses: [],
  conflict_strategy: "duplicate",
  duplicate_suffix: "_edited",
  sync_links: true,
  auto_start_tray: false,
};
const flags = new URLSearchParams(location.search);
const storageKey = "ufora-e2e-config";
let config: AppConfig = JSON.parse(sessionStorage.getItem(storageKey) ?? "null") ?? initialConfig;
const signedIn = !flags.has("signedOut");

mockWindows("main");
mockIPC((command, args) => {
  const payload = args && "newCfg" in args ? args.newCfg as AppConfig : undefined;
  switch (command) {
    case "get_config": return config;
    case "get_auth_status": return { is_authenticated: signedIn, message: "Sign in to Ufora" };
    case "get_recent_logs": return [];
    case "list_courses": return signedIn ? courses : [];
    case "get_autostart_status": return config.auto_start_tray;
    case "set_autostart": return args && "enabled" in args ? args.enabled : undefined;
    case "open_login": return emit("log-message", { timestamp: "12:00:00", message: "Login requested" });
    case "sync_now":
      return Promise.all([
        emit("sync-status", "syncing"),
        emit("log-message", { timestamp: "12:00:00", message: "Scanning Algorithms" }),
      ]);
    case "save_config":
      if (flags.has("failSave")) throw new Error("Fixture: disk write failed");
      if (!payload) throw new Error("Missing config in save_config");
      config = payload;
      sessionStorage.setItem(storageKey, JSON.stringify(config));
      return;
    default: throw new Error(`Unexpected Tauri command: ${command}`);
  }
}, { shouldMockEvents: true });

void import("../src/main");
