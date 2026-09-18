/**
 * Generuje latest.json dla Tauri Updater v2 ze zbudowanych artefaktów.
 *
 * Użycie:
 *   1. Ustaw klucze (jednorazowo):  npm run tauri signer generate -w ~/.tauri/ambad.key
 *      -> pubkey wklej do src-tauri/tauri.conf.json > plugins.updater.pubkey
 *      -> private key ustaw jako env TAURI_SIGNING_PRIVATE_KEY przy buildzie
 *   2. Podbij wersję w package.json + src-tauri/tauri.conf.json + src-tauri/Cargo.toml
 *   3. Zbuduj: TAURI_SIGNING_PRIVATE_KEY="..." npx tauri build
 *      (albo na CI — artefakty .zip/.tar.gz + .sig lądują w src-tauri/target/release/bundle/)
 *   4. Wygeneruj latest.json: node scripts/publish-update.mjs --base https://updates.ambad.pl --notes "Poprawki"
 *      -> pliki lądują w dist-updates/
 *   5. Wgraj zawartość dist-updates/ na hosting tak żeby
 *      https://updates.ambad.pl/latest.json działał.
 *
 * Wspiera: windows (nsis, msi), linux (appimage, deb), macos (app).
 */
import { readdirSync, readFileSync, writeFileSync, copyFileSync, mkdirSync, statSync } from "node:fs";
import { join, basename } from "node:path";

const args = process.argv.slice(2);
const getArg = (name, fallback = "") => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : fallback;
};

const BASE_URL = (getArg("--base", process.env.UPDATE_BASE_URL || "https://updates.ambad.pl")).replace(/\/$/, "");
const NOTES = getArg("--notes", process.env.UPDATE_NOTES || "");
const OUT_DIR = getArg("--out", "dist-updates");
const BUNDLE_DIR = "src-tauri/target/release/bundle";

const PLATFORM_MAP = [
  { match: /nsis.*\.zip$/i, platform: "windows-x86_64" },
  { match: /msi.*\.zip$/i, platform: "windows-x86_64" },
  { match: /\.AppImage\.tar\.gz$/i, platform: "linux-x86_64" },
  { match: /amd64.*\.deb\.tar\.gz$/i, platform: "linux-x86_64" },
  { match: /aarch64.*\.deb\.tar\.gz$/i, platform: "linux-aarch64" },
  { match: /x86_64.*\.app\.tar\.gz$/i, platform: "darwin-x86_64" },
  { match: /aarch64.*\.app\.tar\.gz$/i, platform: "darwin-aarch64" },
];

function walk(dir, out = []) {
  if (!safeExists(dir)) return out;
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    const st = statSync(p);
    if (st.isDirectory()) walk(p, out);
    else out.push(p);
  }
  return out;
}
function safeExists(p) {
  try { statSync(p); return true; } catch { return false; }
}

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const version = pkg.version;
const pubDate = new Date().toISOString();
const files = walk(BUNDLE_DIR).filter((f) => /\.zip$|\.tar\.gz$/i.test(f) && !/\.sig$/i.test(f));

if (files.length === 0) {
  console.error(`Nie znaleziono artefaktów w ${BUNDLE_DIR}. Najpierw uruchom 'npx tauri build' z TAURI_SIGNING_PRIVATE_KEY.`);
  process.exit(1);
}

mkdirSync(OUT_DIR, { recursive: true });
const platforms = {};

for (const file of files) {
  const name = basename(file);
  const sigFile = file + ".sig";
  if (!safeExists(sigFile)) {
    console.warn(`[skip] brak podpisu dla ${name} (oczekiwano ${basename(sigFile)}). Upewnij się że build był z kluczem podpisującym.`);
    continue;
  }
  const entry = PLATFORM_MAP.find((m) => m.match.test(name));
  if (!entry) {
    console.warn(`[skip] nierozpoznany artefakt: ${name}`);
    continue;
  }
  if (platforms[entry.platform]) {
    console.warn(`[skip] duplikat platformy ${entry.platform}: ${name} (już jest ${platforms[entry.platform].url})`);
    continue;
  }
  const signature = readFileSync(sigFile, "utf8").trim();
  copyFileSync(file, join(OUT_DIR, name));
  copyFileSync(sigFile, join(OUT_DIR, name + ".sig"));
  platforms[entry.platform] = { signature, url: `${BASE_URL}/${name}` };
  console.log(`[ok] ${entry.platform} -> ${name}`);
}

if (Object.keys(platforms).length === 0) {
  console.error("Brak platform do publikacji — przerwano (sprawdź .sig i nazwy plików).");
  process.exit(1);
}

const latest = { version, notes: NOTES, pub_date: pubDate, platforms };
writeFileSync(join(OUT_DIR, "latest.json"), JSON.stringify(latest, null, 2));
console.log(`\nGotowe: ${OUT_DIR}/latest.json v${version} (${Object.keys(platforms).join(", ")})`);
console.log(`Wgraj ${OUT_DIR}/* na hosting pod ${BASE_URL}/latest.json`);
