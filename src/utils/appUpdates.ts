import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UpdateInfo } from "../types";

let inFlight: Promise<UpdateInfo> | undefined;
let startup: Promise<UpdateInfo | null> | undefined;

export function checkForUpdates(): Promise<UpdateInfo> {

  return inFlight ??= invoke<UpdateInfo>("check_for_updates").finally(() => {
    inFlight = undefined;
  });
}

export function useStartupUpdate(enabled = true) {
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  useEffect(() => {
    if (!enabled || !("__TAURI_INTERNALS__" in window)) return;
    let active = true;
    startup ??= checkForUpdates().catch(() => null);
    void startup.then((result) => {
      if (active && result?.has_update) setUpdate(result);
    });
    return () => { active = false; };
  }, [enabled]);

  return { update, dismissUpdate: () => setUpdate(null) };
}
