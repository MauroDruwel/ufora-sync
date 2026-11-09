import { defineConfig, devices } from "@playwright/test";

const baseURL = "http://127.0.0.1:1425";

export default defineConfig({
  testDir: "./e2e",
  forbidOnly: Boolean(process.env.CI),
  use: { baseURL, trace: "retain-on-failure" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "pnpm exec vite --host 127.0.0.1 --port 1425 --strictPort",
    url: `${baseURL}/e2e/index.html`,
  },
});
