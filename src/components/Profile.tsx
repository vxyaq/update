interface Props {
  user: {
    name: string
    type: string
  } | null
  onSave: (name: string) => void
  onClose: () => void
  onLogout: () => void
}

export default function Profile({ user, onClose, onLogout }: Props) {
  const displayName = user?.name?.trim() || "Nieznany"
  const accountType = user?.type?.trim() || "—"
  const avatarLetter = displayName.charAt(0).toUpperCase() || "?"

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

          <h1 className="page-title">Profil</h1>
        </div>

        <div className="page-body">
          <div className="profile-avatar-big" aria-hidden="true">
            {avatarLetter}
          </div>

          <div className="profile-name">
            {displayName}
          </div>

          <div className="profile-badge">
            {accountType}
          </div>

          <div className="profile-info-card">
            <div className="info-row">
              <span className="info-label">Status</span>
              <span
                className="info-value profile-status"
                aria-label="Zalogowany"
              >
                Zalogowany
              </span>
            </div>

            <div className="info-row">
              <span className="info-label">Typ konta</span>
              <span className="info-value">
                {accountType}
              </span>
            </div>

            <div className="info-row">
              <span className="info-label">Nick</span>
              <span className="info-value">
                {displayName}
              </span>
            </div>
          </div>

          <button
            type="button"
            className="btn-danger"
            onClick={onLogout}
          >
            Wyloguj się
          </button>
        </div>
      </div>
    </div>
  )
}
