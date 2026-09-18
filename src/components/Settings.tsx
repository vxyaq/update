import { useEffect, useState } from "react"

interface SettingsData {
  ram: number
  width: number
  height: number
}

interface Props {
  javaPath: string
  onSelectJava: () => void
  onClose: () => void
  onSave?: (settings: SettingsData) => void
}

const DEFAULT_SETTINGS: SettingsData = {
  ram: 2048,
  width: 1280,
  height: 720,
}

const MIN_RAM = 512
const MAX_RAM = 16384
const RAM_STEP = 512

const MIN_WIDTH = 640
const MAX_WIDTH = 3840

const MIN_HEIGHT = 480
const MAX_HEIGHT = 2160

const STORAGE_KEY = "ambad-client-settings"

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max)
}

function loadSettings(): SettingsData {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)

    if (!saved) {
      return DEFAULT_SETTINGS
    }

    const parsed = JSON.parse(saved) as Partial<SettingsData>

    return {
      ram: clamp(
        Number.isFinite(parsed.ram) ? Number(parsed.ram) : DEFAULT_SETTINGS.ram,
        MIN_RAM,
        MAX_RAM
      ),
      width: clamp(
        Number.isFinite(parsed.width)
          ? Number(parsed.width)
          : DEFAULT_SETTINGS.width,
        MIN_WIDTH,
        MAX_WIDTH
      ),
      height: clamp(
        Number.isFinite(parsed.height)
          ? Number(parsed.height)
          : DEFAULT_SETTINGS.height,
        MIN_HEIGHT,
        MAX_HEIGHT
      ),
    }
  } catch {
    return DEFAULT_SETTINGS
  }
}

export default function Settings({
  javaPath,
  onSelectJava,
  onClose,
  onSave,
}: Props) {
  const [settings, setSettings] = useState<SettingsData>(loadSettings)
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    setSaved(false)
  }, [settings])

  const updateRam = (value: string) => {
    const parsed = Number(value)

    if (!Number.isFinite(parsed)) {
      return
    }

    setSettings(current => ({
      ...current,
      ram: clamp(parsed, MIN_RAM, MAX_RAM),
    }))
  }

  const updateWidth = (value: string) => {
    const parsed = Number(value)

    if (!Number.isFinite(parsed)) {
      return
    }

    setSettings(current => ({
      ...current,
      width: clamp(parsed, MIN_WIDTH, MAX_WIDTH),
    }))
  }

  const updateHeight = (value: string) => {
    const parsed = Number(value)

    if (!Number.isFinite(parsed)) {
      return
    }

    setSettings(current => ({
      ...current,
      height: clamp(parsed, MIN_HEIGHT, MAX_HEIGHT),
    }))
  }

  const handleSave = () => {
    const normalized: SettingsData = {
      ram: clamp(
        Math.round(settings.ram / RAM_STEP) * RAM_STEP,
        MIN_RAM,
        MAX_RAM
      ),
      width: clamp(Math.round(settings.width), MIN_WIDTH, MAX_WIDTH),
      height: clamp(Math.round(settings.height), MIN_HEIGHT, MAX_HEIGHT),
    }

    setSettings(normalized)

    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized))
    } catch {
    }

    onSave?.(normalized)
    setSaved(true)
  }

  return (
    <div className="fullscreen-view">
      <div className="background" />
      <div className="background-overlay" />

      <div className="fullscreen-content">
        <div className="page-header">
          <button
            type="button"
            className="btn-back"
            onClick={onClose}
          >
            ← Wróć
          </button>

          <h1 className="page-title">Ustawienia</h1>
        </div>

        <div className="page-body">
          <div className="settings-section">
            <div className="settings-section-title">Java</div>

            <div className="settings-row">
              <div className="settings-row-label">
                <span>Ścieżka Java</span>

                <small title={javaPath}>
                  {javaPath.trim() || "Java nie została wybrana"}
                </small>
              </div>

              <button
                type="button"
                className="btn-ghost"
                onClick={onSelectJava}
              >
                Przeglądaj
              </button>
            </div>
          </div>

          <div className="settings-section">
            <div className="settings-section-title">Pamięć RAM</div>

            <div className="settings-row">
              <div className="settings-row-label">
                <span>Maksymalna pamięć</span>
                <small>Zalecane: 2048–4096 MB</small>
              </div>

              <div className="settings-input-group">
                <input
                  type="number"
                  className="settings-input"
                  value={settings.ram}
                  min={MIN_RAM}
                  max={MAX_RAM}
                  step={RAM_STEP}
                  inputMode="numeric"
                  onChange={event => updateRam(event.target.value)}
                />

                <span className="settings-unit">MB</span>
              </div>
            </div>
          </div>

          <div className="settings-section">
            <div className="settings-section-title">
              Rozdzielczość okna
            </div>

            <div className="settings-row">
              <div className="settings-row-label">
                <span>Szerokość</span>
              </div>

              <div className="settings-input-group">
                <input
                  type="number"
                  className="settings-input"
                  value={settings.width}
                  min={MIN_WIDTH}
                  max={MAX_WIDTH}
                  step={1}
                  inputMode="numeric"
                  onChange={event => updateWidth(event.target.value)}
                />

                <span className="settings-unit">px</span>
              </div>
            </div>

            <div className="settings-row">
              <div className="settings-row-label">
                <span>Wysokość</span>
              </div>

              <div className="settings-input-group">
                <input
                  type="number"
                  className="settings-input"
                  value={settings.height}
                  min={MIN_HEIGHT}
                  max={MAX_HEIGHT}
                  step={1}
                  inputMode="numeric"
                  onChange={event => updateHeight(event.target.value)}
                />

                <span className="settings-unit">px</span>
              </div>
            </div>
          </div>

          <button
            type="button"
            className="btn-primary"
            style={{ marginTop: 8 }}
            onClick={handleSave}
          >
            {saved ? "Zapisano" : "Zapisz ustawienia"}
          </button>
        </div>
      </div>
    </div>
  )
}
