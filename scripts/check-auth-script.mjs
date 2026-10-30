import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const authPath = fileURLToPath(new URL("../src-tauri/src/auth.rs", import.meta.url));
const source = readFileSync(authPath, "utf8");
const match = source.match(/const AUTO_AUTH_AND_TOKEN_SCRIPT: &str = r##"([\s\S]*?)"##;/);

if (!match) {
  throw new Error("Could not locate AUTO_AUTH_AND_TOKEN_SCRIPT in src-tauri/src/auth.rs");
}

new Function(match[1]);
console.log("Embedded authentication script syntax is valid.");
