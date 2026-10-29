import { describe, it, expect } from "vitest";
import { Course, AppConfig, AuthStatus, UpdateInfo } from "./types";

describe("Type definitions and domain contracts", () => {
  it("validates Course structure", () => {
    const course: Course = {
      id: "12345",
      name: "Wiskunde I",
      code: "E610004A_2026",
      home_url: "https://ufora.ugent.be/d2l/home/12345",
      folder_name: "E610004A - Wiskunde I",
      is_course_offering: true,
      academic_year: 2026,
    };

    expect(course.id).toBe("12345");
    expect(course.code).toBe("E610004A_2026");
    expect(course.folder_name).toContain("Wiskunde I");
    expect(course.is_course_offering).toBe(true);
    expect(course.academic_year).toBe(2026);
  });

  it("validates AppConfig defaults and conflict strategy types", () => {
    const validStrategies = ["duplicate", "skip", "overwrite"];
    const cfg: AppConfig = {
      sync_dir: "/Users/test/Documents/Ufora",
      interval_minutes: 30,
      enabled_courses: ["12345"],
      conflict_strategy: "duplicate",
      duplicate_suffix: "_edited",
      sync_descriptions: true,
      sync_links: true,
      auto_start_tray: true,
    };

    expect(validStrategies).toContain(cfg.conflict_strategy);
    expect(cfg.interval_minutes).toBeGreaterThanOrEqual(15);
    expect(cfg.duplicate_suffix).toBe("_edited");
  });

  it("validates AuthStatus behavior", () => {
    const auth: AuthStatus = {
      is_authenticated: true,
      user_id: "user_ugent_123",
      message: "Authenticated",
    };

    expect(auth.is_authenticated).toBe(true);
    expect(auth.user_id).toBeDefined();

    const loggedOut: AuthStatus = {
      is_authenticated: false,
      message: "Token expired",
    };
    expect(loggedOut.is_authenticated).toBe(false);
    expect(loggedOut.user_id).toBeUndefined();
  });

  it("validates UpdateInfo contract", () => {
    const update: UpdateInfo = {
      current_version: "1.0.1",
      latest_version: "1.1.0",
      has_update: true,
      release_notes: "Added automatic sync",
      html_url: "https://github.com/MauroDruwel/ufora-sync/releases/tag/v1.1.0",
      published_at: "2026-10-29T10:00:00Z",
    };

    expect(update.has_update).toBe(true);
    expect(update.current_version).toBe("1.0.1");
    expect(update.latest_version).toBe("1.1.0");
    expect(update.html_url).toContain("MauroDruwel/ufora-sync");
  });
});
