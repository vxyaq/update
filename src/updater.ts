import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "downloading"; downloaded: number; total: number | null }
  | { phase: "installing" }
  | { phase: "ready-to-restart" }
  | { phase: "up-to-date" }
  | { phase: "error"; message: string };

const isTauri = () => !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;

/**
 * Cichy auto-update: sprawdza przy starcie, pobiera w tle i restartuje apkę.
 * Wywoływać raz po starcie App. onStatus służy do pokazania paska postępu.
 */
export async function checkForUpdatesSilently(
  onStatus?: (s: UpdateStatus) => void,
): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    onStatus?.({ phase: "checking" });
    const update = await check();
    if (!update) {
      onStatus?.({ phase: "up-to-date" });
      return false;
    }

    let total = 0;
    let downloaded = 0;
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          total = event.data.contentLength ?? 0;
          onStatus?.({ phase: "downloading", downloaded: 0, total: total || null });
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          onStatus?.({ phase: "downloading", downloaded, total: total || null });
          break;
        case "Finished":
          onStatus?.({ phase: "installing" });
          break;
      }
    });

    onStatus?.({ phase: "ready-to-restart" });
    // Krótka pauza żeby user zobaczył "instalowanie", potem restart
    await new Promise((r) => setTimeout(r, 800));
    await relaunch();
    return true;
  } catch (e) {
    // Brak pubkey / brak netu / brak hostingu — nie blokuj apki
    const message = e instanceof Error ? e.message : String(e);
    console.warn("[updater] check failed:", message);
    onStatus?.({ phase: "error", message });
    return false;
  }
}
