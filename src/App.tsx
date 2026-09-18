import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import Profile from "./components/Profile";
import Settings from "./components/Settings";
import Login from "./components/Login";
import { checkForUpdatesSilently, type UpdateStatus } from "./updater";
import "./App.css";

interface MinecraftVersion {
  id: string;
  type: string;
  release_time: string;
  installed: boolean;
}

interface DisplayUser {
  name: string;
  type: "Premium";
}

export default function App() {
  const [versions, setVersions] = useState<MinecraftVersion[]>([]);
  const [selected, setSelected] = useState("");
  const [javaPath, setJavaPath] = useState("java");
  const [status, setStatus] = useState("");
  const [versionOpen, setVersionOpen] = useState(false);
  const [accountOpen, setAccountOpen] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [running, setRunning] = useState(false);
  const [view, setView] = useState<"home" | "profile" | "settings">("home");
  const [user, setUser] = useState<DisplayUser | null>(null);
  const [authChecked, setAuthChecked] = useState(false);
  const [appVersion, setAppVersion] = useState("");
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>({ phase: "idle" });

  useEffect(() => {
    // Cichy auto-update w tle — nie blokuje startu apki
    void checkForUpdatesSilently(setUpdateStatus);
    // Numer wersji apki do rogu (żeby było widać, czy update się zainstalował)
    void invoke<string>("get_app_version").then(setAppVersion).catch(() => {});
  }, []);

  useEffect(() => {
    localStorage.removeItem("embad_user");

    const isTauri = () => !!(window as any).__TAURI_INTERNALS__;

    const load = async () => {
      if (!isTauri()) {
        setAuthChecked(true);
        return;
      }
      try {
        const prof = await invoke<{ id: string; name: string } | null>("get_minecraft_profile");
        setUser(prof ? { name: prof.name, type: "Premium" } : null);
      } catch {
        setUser(null);
      } finally {
        setAuthChecked(true);
      }
      let defaultVersion = "";
      try {
        const result = await invoke<MinecraftVersion[]>("get_versions");
        setVersions(result);
        const first = result.find((v) => v.installed) ?? result[0];
        if (first) {
          setSelected(first.id);
          defaultVersion = first.id;
        }
      } catch {
        setStatus("Błąd ładowania wersji");
      }
      try {
        const java = await invoke<string>("detect_java");
        if (java) setJavaPath(java);
      } catch {
        // Brak Javy na PC — dociągnij przenośną w tle (jednorazowo, bez admina).
        try {
          if (defaultVersion) {
            const java = await invoke<string>("ensure_java", { versionId: defaultVersion });
            if (java) setJavaPath(java);
          }
        } catch { /* non-fatal — launch i tak spróbuje przy Graj */ }
      }
    };

    void load();

    // poll czy własny Minecraft z AmbadClient/minecraft jest uruchomiony — blokuje Graj i pokazuje STOP
    const interval = setInterval(async () => {
      try {
        const r = await invoke<boolean>("is_minecraft_running");
        setRunning(r);
        if (r) setLaunching(false);
      } catch {}
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  const launch = async () => {
    if (!isLogged || !selected || launching || running) return;
    setLaunching(true);
    setStatus("Uruchamianie własnego Minecraft z AmbadClient/minecraft...");
    try {
      await invoke("launch_minecraft", {
        options: {
          version_id: selected,
          java_path: javaPath,
          max_memory: 2048,
          min_memory: 512,
          width: 1280,
          height: 720,
          fullscreen: false,
          offline: false,
          username: user?.name ?? "",
        },
      });
      setStatus("Minecraft uruchomiony");
      setRunning(true);
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : String(error);
      setStatus(msg);
    } finally {
      setLaunching(false);
    }
  };

  const stop = async () => {
    try {
      await invoke("stop_minecraft");
      setStatus("Minecraft zatrzymany");
      setRunning(false);
    } catch (error: unknown) {
      const msg = error instanceof Error ? error.message : String(error);
      setStatus(msg);
    }
  };

  const selectJava = async () => {
    try {
      const path = await open({ title: "Wybierz Java", directory: false, multiple: false });
      if (typeof path === "string" && path.length > 0) {
        setJavaPath(path);
      }
    } catch { /* cancelled */ }
  };

  const handlePremiumLogin = (name: string) => {
    setUser({ name, type: "Premium" });
  };

  const saveUser = (name: string) => {
    setUser({ name, type: "Premium" });
    setView("home");
    setAccountOpen(false);
  };

  const logout = async () => {
    try { await invoke("microsoft_logout"); } catch { /* ignore */ }
    setUser(null);
    setAccountOpen(false);
    setView("home");
  };

  const isLogged = !!user;

  if (!authChecked) {
    return (
      <div className="launcher">
        <div className="background" />
        <div className="background-overlay" />
      </div>
    );
  }

  return (
    <div className="launcher">
      <div className="background" />
      <div className="background-overlay" />

      {!isLogged && <Login onLogin={handlePremiumLogin} />}

      {/* topbar */}
      <header className="topbar" data-tauri-drag-region>
        <div className="topbar-drag" data-tauri-drag-region />

        <div className="account-wrap">
          <button className="account" type="button" onClick={() => setAccountOpen((v) => !v)}>
            <span className="avatar">{(user?.name?.[0] ?? "?").toUpperCase()}</span>
            <span className="account-name">{user?.name ?? "—"}</span>
            <span className={`account-arrow ${accountOpen ? "open" : ""}`}>‹</span>
          </button>

          {accountOpen && (
            <div className="account-menu">
              <button type="button" onClick={() => { setView("profile"); setAccountOpen(false); }}>
                Profil
              </button>
              <button type="button" onClick={() => { setView("settings"); setAccountOpen(false); }}>
                Ustawienia
              </button>
              <div className="menu-sep" />
              <button type="button" className="menu-logout" onClick={logout}>
                Wyloguj
              </button>
            </div>
          )}
        </div>
      </header>

      {/* fullscreen views */}
      {view === "profile" && (
        <Profile user={user} onSave={saveUser} onClose={() => setView("home")} onLogout={logout} />
      )}
      {view === "settings" && (
        <Settings javaPath={javaPath} onSelectJava={() => void selectJava()} onClose={() => setView("home")} />
      )}

      {/* main */}
      <main className="main">
        {/* lewa strona — info o wersji */}
        <div className="side-info">
          {appVersion && <div className="side-label">Ambad v{appVersion}</div>}
          {selected && (
            <>
              <div className="side-label">Minecraft Java</div>
              <div className="side-version">{selected}</div>
            </>
          )}
        </div>

        {/* środek — play / stop */}
        <div className="play-area">
          {(updateStatus.phase === "checking" ||
            updateStatus.phase === "downloading" ||
            updateStatus.phase === "installing" ||
            updateStatus.phase === "ready-to-restart") && (
            <div className="update-banner">
              <span className="play-spinner" style={{ width: 12, height: 12 }} />
              <span>
                {updateStatus.phase === "checking" && "Sprawdzanie aktualizacji…"}
                {updateStatus.phase === "downloading" &&
                  (updateStatus.total
                    ? `Pobieranie aktualizacji… ${(updateStatus.downloaded / 1024 / 1024).toFixed(1)} / ${(updateStatus.total / 1024 / 1024).toFixed(1)} MB`
                    : `Pobieranie aktualizacji… ${(updateStatus.downloaded / 1024 / 1024).toFixed(1)} MB`)}
                {updateStatus.phase === "installing" && "Instalowanie aktualizacji…"}
                {updateStatus.phase === "ready-to-restart" && "Restartowanie…"}
              </span>
            </div>
          )}
          {status && <div className="error-msg">{status}</div>}

          {!running ? (
            <button
              className="play-button"
              type="button"
              disabled={!isLogged || !selected || launching}
              onClick={() => void launch()}
            >
              {launching ? (
                <span className="play-spinner" />
              ) : (
                <span className="play-icon">▶</span>
              )}
              <span>{launching ? "Uruchamianie..." : isLogged ? "Graj" : "Zaloguj się"}</span>
            </button>
          ) : (
            <button
              className="play-button"
              type="button"
              style={{ background: "linear-gradient(180deg, #ff6b6b 0%, #ef4444 100%)", borderColor: "rgba(239,68,68,0.3)", color: "#fff", boxShadow: "0 8px 32px rgba(239,68,68,0.28)" }}
              onClick={() => void stop()}
            >
              <span>■</span>
              <span>STOP</span>
            </button>
          )}
        </div>

        {/* prawa strona — wybór wersji (auto-instalacja przy Graj) */}
        <div className="side-version-wrap">
          <div className="version-selector">
            <button
              className={`version-button ${versionOpen ? "open" : ""}`}
              type="button"
              onClick={() => setVersionOpen((v) => !v)}
            >
              <span className="version-text">
                <small>WERSJA</small>
                <strong>{selected || "Wybierz"}</strong>
              </span>
              <span className="version-arrow">{versionOpen ? "↑" : "↓"}</span>
            </button>

            {versionOpen && (
              <div className="version-menu">
                {versions.length === 0 ? (
                  <div className="version-empty">Brak wersji</div>
                ) : (
                  versions.map((v) => (
                    <button
                      key={v.id}
                      type="button"
                      className={v.id === selected ? "version-option selected" : "version-option"}
                      onClick={() => { setSelected(v.id); setVersionOpen(false); }}
                    >
                      <span className="vo-id">{v.id}</span>
                      {v.installed && <span className="vo-dot" />}
                    </button>
                  ))
                )}
                <div className="menu-divider" />
                <button type="button" className="java-option" onClick={() => void selectJava()}>
                  <span>
                    <strong>Java</strong>
                    <small title={javaPath}>{javaPath}</small>
                  </span>
                  <span>›</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
