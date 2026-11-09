import { expect, test } from "@playwright/test";

test("bulk selection respects the filter and survives reload", async ({ page }) => {
  await page.goto("/e2e/index.html");
  await page.getByRole("textbox", { name: "Filter courses" }).fill("Algorithms");
  await page.getByRole("button", { name: "Manage course synchronization selection" }).click();
  await page.getByRole("button", { name: "Disable Visible", exact: true }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Disable 1 Items" }).click();
  await expect(page.getByRole("checkbox", { name: "Enable synchronization for Algorithms" })).not.toBeChecked();
  await expect.poll(() => page.evaluate(() => sessionStorage.getItem("ufora-e2e-config"))).toContain('"enabled_courses":["2"]');
  await page.reload();
  await expect(page.getByRole("checkbox", { name: "Enable synchronization for Algorithms" })).not.toBeChecked();
  await expect(page.getByRole("checkbox", { name: "Disable synchronization for Databases" })).toBeChecked();
});

test("failed course save restores the previous selection", async ({ page }) => {
  await page.goto("/e2e/index.html?failSave");
  const course = page.getByRole("checkbox", { name: /synchronization for Algorithms/ });
  await course.focus();
  await course.press("Space");
  await expect(page.getByRole("alert")).toContainText("previous selection was restored");
  await expect(course).toBeChecked();
});

test("settings discard drafts and persist saved changes", async ({ page }) => {
  await page.goto("/e2e/index.html");
  const open = page.getByRole("button", { name: "Open preferences and settings" });
  const settings = page.getByRole("dialog");
  const interval = settings.getByRole("combobox", { name: "Sync interval" });
  await open.click();
  await interval.selectOption("60");
  await page.keyboard.press("Escape");
  await expect(settings).toBeHidden();
  await open.click();
  await expect(interval).toHaveValue("30");
  await interval.selectOption("60");
  await settings.getByRole("button", { name: "Save preferences" }).click();
  await expect(settings).toBeHidden();
  await page.reload();
  await open.click();
  await expect(interval).toHaveValue("60");
});

test("a sync event updates the status and activity log", async ({ page }) => {
  await page.goto("/e2e/index.html");
  await expect(page.getByRole("heading", { name: "Algorithms" })).toBeVisible();
  await page.getByRole("button", { name: "Sync Now" }).click();
  await expect(page.getByText("Syncing materials…")).toBeVisible();
  const courses = page.getByRole("tab", { name: /Courses/ });
  await courses.focus();
  await courses.press("ArrowRight");
  await expect(page.getByRole("tab", { name: "Activity Log" })).toBeFocused();
  await expect(page.getByRole("tabpanel")).toContainText("Scanning Algorithms");
});

test("signed-out users can start sign-in", async ({ page }) => {
  await page.goto("/e2e/index.html?signedOut");
  const panel = page.getByRole("tabpanel");
  await expect(panel.getByRole("heading", { name: "Authentication Required" })).toBeVisible();
  await panel.getByRole("button", { name: "Log in with UGent Ufora" }).click();
  await page.getByRole("tab", { name: "Activity Log" }).click();
  await expect(page.getByRole("tabpanel")).toContainText("Login requested");
});
