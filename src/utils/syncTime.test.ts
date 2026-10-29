import { describe, it, expect } from "vitest";
import { formatRelativeSyncTime } from "./syncTime";

describe("formatRelativeSyncTime helper", () => {
  it("handles null or undefined correctly", () => {
    expect(formatRelativeSyncTime(null)).toBe("Ready to sync");
    expect(formatRelativeSyncTime(undefined)).toBe("Ready to sync");
    expect(formatRelativeSyncTime("invalid-date")).toBe("Ready to sync");
  });

  it("formats recent syncs (< 45s) as 'Synced just now'", () => {
    const base = 1730190000000;
    const past = new Date(base - 20000).toISOString();
    expect(formatRelativeSyncTime(past, base)).toBe("Synced just now");
  });

  it("formats minutes ago correctly", () => {
    const base = 1730190000000;
    const past2m = new Date(base - 120000).toISOString();
    expect(formatRelativeSyncTime(past2m, base)).toBe("Synced 2 min ago");

    const past15m = new Date(base - 900000).toISOString();
    expect(formatRelativeSyncTime(past15m, base)).toBe("Synced 15 min ago");
  });

  it("formats same day (> 60m but < 12h) with hours:minutes", () => {
    const base = 1730190000000;
    const past3h = new Date(base - 3 * 3600 * 1000);
    const result = formatRelativeSyncTime(past3h.toISOString(), base);
    expect(result).toMatch(/^Last synced \d{2}:\d{2}$/);
  });
});
