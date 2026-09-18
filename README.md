# ambad

Ambad Client — launcher Minecraft Java Edition (Tauri v2 + React).

## Build lokalny

```bash
npm install
npm run tauri:dev      # dev
npm run tauri:build    # build instalki (Linux: .deb/.rpm/.AppImage)
```

## Release (Windows .exe + Linux + macOS + auto-update)

1. Podbij wersję w `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` (muszą być równe).
2. Commit + tag:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```
3. GitHub Actions (`.github/workflows/publish.yml`) zbuduje paczki na Windows (.exe NSIS + .msi),
   Linux (.AppImage + .deb) i macOS (.dmg) i dopnie je do Releases razem z `latest.json`.
4. Apki na PC same pobiorą update przy starcie (cichy auto-update + restart).

Sekrety wymagane w repo (Settings → Secrets → Actions):
- `TAURI_SIGNING_PRIVATE_KEY` — klucz prywatny do podpisywania updatów
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — hasło klucza (puste, jeśli brak)
