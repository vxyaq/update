@echo off
REM Dash Client - build instalki Windows (.exe + .msi) + pliki do auto-update
REM Wymaga: Node.js LTS (https://nodejs.org) i Rust (https://rustup.rs)
REM Klucz prywatny (dash.key) poloz obok tego pliku LUB ustaw zmienna srodowiskowa
REM   set TAURI_SIGNING_PRIVATE_KEY_PATH=C:\sciezka\do\dash.key
REM Bez klucza powstanie zwykly instalator (do recznej instalacji, bez auto-update).

cd /d "%~dp0"

where node >nul 2>nul || (echo [BLAD] Brak Node.js - zainstaluj z https://nodejs.org && pause && exit /b 1)
where cargo >nul 2>nul || (echo [BLAD] Brak Rust - zainstaluj z https://rustup.rs && pause && exit /b 1)

if exist "dash.key" (
  set TAURI_SIGNING_PRIVATE_KEY_PATH=%~dp0dash.key
  set TAURI_SIGNING_PRIVATE_KEY_PASSWORD=
  echo [OK] Znaleziono dash.key - build bedzie podpisany (auto-update zadziala).
) else (
  echo [INFO] Brak dash.key obok pliku - build bez podpisu (tylko reczna instalacja).
)

call npm install || (echo [BLAD] npm install sie nie powiodlo && pause && exit /b 1)
call npx tauri build || (echo [BLAD] Build sie nie powiodl && pause && exit /b 1)

echo.
echo [GOTOWE] Instalki w: src-tauri\target\release\bundle\
echo   - NSIS (.exe): bundle\nsis\
echo   - MSI:         bundle\msi\
echo   - pliki updatera (*.zip + *.sig): dolacz do Release na GitHubie + dopisz do latest.json
echo.
pause
