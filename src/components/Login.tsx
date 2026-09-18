import { useState } from "react"
import { invoke, isTauri } from "@tauri-apps/api/core"

type MinecraftProfile = {
  id: string
  name: string
}

type LoginProps = {
  onLogin: (name: string) => void
}

export default function Login({ onLogin }: LoginProps) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState("")

  const login = async () => {
    if (busy) return

    setError("")
    setBusy(true)

    try {
      const profile = await invoke<MinecraftProfile>("microsoft_login")
      onLogin(profile.name)
    } catch (error) {
      const message =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : "Logowanie przez Microsoft nie powiodło się."

      setError(message)
    } finally {
      setBusy(false)
    }
  }

  const handleLogin = () => {
    if (!isTauri()) {
      setError(
        "Launcher musi być uruchomiony jako aplikacja Tauri. Tryb przeglądarkowy nie jest obsługiwany."
      )
      return
    }

    void login()
  }

  return (
    <div className="fullscreen-view">
      <div className="background" />
      <div className="background-overlay" />

      <div className="fullscreen-center">
        <div className="login-card">
          <div className="login-divider" />

          {busy ? (
            <div className="login-waiting" aria-live="polite">
              <div className="login-spinner-big" />

              <p className="login-waiting-title">
                Autoryzacja w toku
              </p>

              <p className="login-waiting-desc">
                Otworzyliśmy okno logowania Microsoft.
                <br />
                Po pomyślnym uwierzytelnieniu wrócisz automatycznie do launchera.
              </p>

              {error && (
                <div className="login-error" role="alert">
                  {error}
                </div>
              )}
            </div>
          ) : (
            <div>
              <p className="login-desc">
                Zaloguj się kontem Microsoft posiadającym licencję
                <br />
                <strong>Minecraft: Java Edition</strong>
              </p>

              <button
                type="button"
                className="btn-primary login-btn"
                onClick={handleLogin}
                disabled={busy}
              >
                Kontynuuj z Microsoft
              </button>

              {error && (
                <div className="login-error" role="alert">
                  {error}
                </div>
              )}

              <div className="login-note">
                Dane logowania nie są przechowywane na naszych serwerach.
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}