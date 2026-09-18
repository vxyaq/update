#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{command, Manager, WebviewUrl};

const EXPIRY_BUFFER_SECS: i64 = 300;

const MC_CLIENT_ID: &str = "00000000402b5328";
const MC_SCOPE: &str = "XboxLive.signin offline_access";
const LIVE_AUTH: &str = "https://login.live.com/oauth20_authorize.srf";
const LIVE_TOKEN: &str = "https://login.live.com/oauth20_token.srf";
const DESKTOP_REDIRECT: &str = "https://login.live.com/oauth20_desktop.srf";
const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MinecraftVersion {
    id: String,
    r#type: String,
    release_time: String,
    installed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct LaunchOptions {
    version_id: String,
    java_path: String,
    max_memory: u32,
    min_memory: u32,
    width: u32,
    height: u32,
    fullscreen: bool,
    offline: bool,
    username: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MinecraftProfile {
    id: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredAuth {
    minecraft_token: String,
    minecraft_profile: MinecraftProfile,
    refresh_token: String,
    expires_at: String,
}

fn version_gte_1_8_8(id: &str) -> bool {
    let mut parts = id.split('.');

    let major = match parts.next().and_then(|v| v.parse::<u32>().ok()) {
        Some(v) => v,
        None => return false,
    };

    let minor = match parts.next().and_then(|v| v.parse::<u32>().ok()) {
        Some(v) => v,
        None => return false,
    };

    let patch = match parts.next() {
        Some(v) => match v.parse::<u32>() {
            Ok(value) => value,
            Err(_) => return false,
        },
        None => 0,
    };

    (major, minor, patch) >= (1, 8, 8)
}

fn minecraft_dir() -> Result<PathBuf, String> {
    let base = dirs::data_dir().ok_or_else(|| "Nie można znaleźć katalogu danych użytkownika".to_string())?;
    let dir = base.join("AmbadClient").join("minecraft");
    // Create own folder structure: versions / mods / assets / libraries / settings.json
    std::fs::create_dir_all(dir.join("versions")).ok();
    std::fs::create_dir_all(dir.join("mods")).ok();
    std::fs::create_dir_all(dir.join("assets")).ok();
    std::fs::create_dir_all(dir.join("libraries")).ok();
    Ok(dir)
}

fn mods_dir() -> Result<PathBuf, String> {
    Ok(minecraft_dir()?.join("mods"))
}

static MC_PROCESS: std::sync::OnceLock<Arc<Mutex<Option<std::process::Child>>>> = std::sync::OnceLock::new();

fn mc_process() -> Arc<Mutex<Option<std::process::Child>>> {
    MC_PROCESS.get_or_init(|| Arc::new(Mutex::new(None))).clone()
}

fn auth_file() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    let dir = base.join("AmbadClient");

    let _ = fs::create_dir_all(&dir);

    #[cfg(unix)]
    if let Ok(metadata) = fs::metadata(&dir) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = metadata.permissions();
        permissions.set_mode(0o700);
        let _ = fs::set_permissions(&dir, permissions);
    }

    dir.join("premium_auth.json")
}

fn save_auth(auth: &StoredAuth) -> Result<(), String> {
    let path = auth_file();
    let temp = path.with_extension("json.tmp");

    let data = serde_json::to_vec_pretty(auth)
        .map_err(|e| format!("Nie można serializować sesji: {e}"))?;

    fs::write(&temp, data).map_err(|e| format!("Nie można zapisać sesji: {e}"))?;

    #[cfg(unix)]
    if let Ok(metadata) = fs::metadata(&temp) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        let _ = fs::set_permissions(&temp, permissions);
    }

    fs::rename(&temp, &path)
        .map_err(|e| format!("Nie można zatwierdzić pliku sesji: {e}"))
}

fn load_auth() -> Option<StoredAuth> {
    let path = auth_file();
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn clear_auth() {
    let _ = fs::remove_file(auth_file());
}

fn token_is_valid(auth: &StoredAuth) -> bool {
    chrono::DateTime::parse_from_rfc3339(&auth.expires_at)
        .map(|expires| {
            expires.with_timezone(&chrono::Utc)
                > chrono::Utc::now()
                    + chrono::Duration::seconds(EXPIRY_BUFFER_SECS)
        })
        .unwrap_or(false)
}

fn validate_java_path(raw: &str) -> Result<String, String> {
    let raw = raw.trim();

    if raw.is_empty() {
        return Err("Ścieżka Javy jest pusta".to_string());
    }

    if raw.len() > 4096 {
        return Err("Ścieżka Javy jest zbyt długa".to_string());
    }

    if raw.contains('\0') || raw.contains('\n') || raw.contains('\r') {
        return Err("Nieprawidłowa ścieżka Javy".to_string());
    }

    let path = Path::new(raw);

    if path.is_absolute() && !path.is_file() {
        return Err("Nie znaleziono pliku Java".to_string());
    }

    let output = Command::new(path)
        .arg("-version")
        .output()
        .map_err(|e| format!("Nie można uruchomić Javy: {e}"))?;

    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_lowercase();

    if !output.status.success() {
        return Err("Java zakończyła się błędem podczas sprawdzania wersji".to_string());
    }

    if !text.contains("java")
        && !text.contains("openjdk")
        && !text.contains("jdk")
    {
        return Err("Wskazany plik nie wygląda na środowisko Java".to_string());
    }

    Ok(raw.to_string())
}

fn extract_param(url: &str, key: &str) -> Option<String> {
    let query = url
        .split_once('?')
        .map(|(_, value)| value)
        .unwrap_or(url)
        .split('#')
        .next()
        .unwrap_or_default();

    for pair in query.split('&') {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));

        if name == key {
            return urlencoding::decode(value)
                .ok()
                .map(|value| value.into_owned());
        }
    }

    None
}

fn validate_version_id(version_id: &str) -> Result<(), String> {
    if version_id.is_empty() {
        return Err("ID wersji jest puste".to_string());
    }

    if version_id.len() > 64 {
        return Err("ID wersji jest zbyt długie".to_string());
    }

    if !version_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    {
        return Err("Nieprawidłowe ID wersji".to_string());
    }

    Ok(())
}

fn validate_username(username: &str) -> Result<(), String> {
    if username.is_empty() || username.len() > 16 {
        return Err("Nieprawidłowa nazwa gracza".to_string());
    }

    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Nieprawidłowa nazwa gracza".to_string());
    }

    Ok(())
}

async fn exchange_xbox_to_minecraft(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<(String, MinecraftProfile), String> {
    let response = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={access_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| format!("Xbox Live request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Xbox Live auth failed (HTTP {})",
            response.status().as_u16()
        ));
    }

    let xbox_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("XBL parse error: {e}"))?;

    let xbl_token = xbox_data["Token"]
        .as_str()
        .ok_or_else(|| "Brak tokenu Xbox Live".to_string())?;

    let uhs = xbox_data["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or_else(|| "Brak identyfikatora użytkownika Xbox".to_string())?;

    let response = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "Properties": {
                "SandboxId": "RETAIL",
                "UserTokens": [xbl_token]
            },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| format!("XSTS request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();

        if body.contains("2148916233") {
            return Err("Konto nie ma powiązanego Xbox Live".to_string());
        }

        if body.contains("2148916238") {
            return Err("Konto dziecięce wymaga zgody rodzica".to_string());
        }

        if body.contains("2148916235") {
            return Err("Xbox Live jest niedostępne w tym kraju".to_string());
        }

        return Err(format!("XSTS failed (HTTP {status})"));
    }

    let xsts_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("XSTS parse error: {e}"))?;

    let xsts_token = xsts_data["Token"]
        .as_str()
        .ok_or_else(|| "Brak tokenu XSTS".to_string())?;

    let response = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({
            "identityToken": format!("XBL3.0 x={uhs};{xsts_token}")
        }))
        .send()
        .await
        .map_err(|e| format!("Minecraft login request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        let detail: String = body.chars().take(200).collect();

        return Err(format!(
            "Minecraft authentication failed (HTTP {status}): {detail}"
        ));
    }

    let mc_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("MC parse error: {e}"))?;

    let minecraft_token = mc_data["access_token"]
        .as_str()
        .ok_or_else(|| "Brak tokenu Minecraft".to_string())?
        .to_string();

    let response = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&minecraft_token)
        .send()
        .await
        .map_err(|e| format!("Profile request failed: {e}"))?;

    if response.status().as_u16() == 404 {
        return Err("Konto nie posiada Minecraft Java Edition".to_string());
    }

    if !response.status().is_success() {
        return Err(format!(
            "Profile fetch failed (HTTP {})",
            response.status().as_u16()
        ));
    }

    let profile_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Profile parse error: {e}"))?;

    let id = profile_data["id"]
        .as_str()
        .ok_or_else(|| "Brak identyfikatora profilu".to_string())?;

    let name = profile_data["name"]
        .as_str()
        .ok_or_else(|| "Brak nazwy profilu".to_string())?;

    Ok((
        minecraft_token,
        MinecraftProfile {
            id: id.to_string(),
            name: name.to_string(),
        },
    ))
}

async fn refresh_token(stored: &StoredAuth) -> Result<StoredAuth, String> {
    if stored.refresh_token.trim().is_empty() {
        return Err("Brak refresh tokenu; wymagane ponowne logowanie".to_string());
    }

    let client = reqwest::Client::new();

    let response = client
        .post(LIVE_TOKEN)
        .form(&[
            ("client_id", MC_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", stored.refresh_token.as_str()),
            ("scope", MC_SCOPE),
        ])
        .send()
        .await
        .map_err(|e| format!("Refresh request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(
            "Odświeżenie tokenu nie powiodło się; zaloguj się ponownie".to_string()
        );
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Refresh parse error: {e}"))?;

    let access_token = data["access_token"]
        .as_str()
        .ok_or_else(|| "Brak access_token".to_string())?;

    let refresh_token = data["refresh_token"]
        .as_str()
        .unwrap_or(&stored.refresh_token)
        .to_string();

    let expires_in = data["expires_in"]
        .as_i64()
        .unwrap_or(86400)
        .clamp(60, 7 * 24 * 60 * 60);

    let (minecraft_token, profile) =
        exchange_xbox_to_minecraft(&client, access_token).await?;

    let auth = StoredAuth {
        minecraft_token,
        minecraft_profile: profile,
        refresh_token,
        expires_at: (chrono::Utc::now()
            + chrono::Duration::seconds(expires_in))
        .to_rfc3339(),
    };

    save_auth(&auth)?;

    Ok(auth)
}

#[command]
async fn get_versions() -> Result<Vec<MinecraftVersion>, String> {
    let client = reqwest::Client::new();

    let response = client
        .get(VERSION_MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("Nie można pobrać listy wersji: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Manifest wersji zwrócił HTTP {}",
            response.status().as_u16()
        ));
    }

    let manifest: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Nie można odczytać manifestu wersji: {e}"))?;

    let versions = manifest["versions"]
        .as_array()
        .ok_or_else(|| "Nieprawidłowy format manifestu".to_string())?;

    let versions_dir = minecraft_dir()?.join("versions");

    let installed: HashSet<String> = match fs::read_dir(&versions_dir) {
        Ok(entries) => entries
            .flatten()
            .filter_map(|entry| {
                let file_type = entry.file_type().ok()?;

                if !file_type.is_dir() {
                    return None;
                }

                let id = entry.file_name().to_str()?.to_string();

                let json = entry.path().join(format!("{id}.json"));
                let jar = entry.path().join(format!("{id}.jar"));

                if json.is_file() && jar.is_file() {
                    Some(id)
                } else {
                    None
                }
            })
            .collect(),
        Err(_) => HashSet::new(),
    };

    let result = versions
        .iter()
        .filter_map(|version| {
            let id = version["id"].as_str()?.to_string();
            let version_type = version["type"].as_str()?;

            if version_type != "release" || !version_gte_1_8_8(&id) {
                return None;
            }

            let release_time = version["releaseTime"].as_str()?.to_string();

            Some(MinecraftVersion {
                id: id.clone(),
                r#type: version_type.to_string(),
                release_time,
                installed: installed.contains(&id),
            })
        })
        .collect();

    Ok(result)
}

#[command]
async fn detect_java() -> Result<String, String> {
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            r"C:\Program Files\Java\jre-1.8\bin\java.exe",
            r"C:\Program Files\Java\jdk-1.8\bin\java.exe",
            r"C:\Program Files\Java\jre-17\bin\java.exe",
            r"C:\Program Files\Java\jdk-17\bin\java.exe",
            r"C:\Program Files\Java\jre-21\bin\java.exe",
            r"C:\Program Files\Java\jdk-21\bin\java.exe",
            r"C:\Program Files\Java\jre-25\bin\java.exe",
            r"C:\Program Files\Java\jdk-25\bin\java.exe",
            r"C:\Program Files\Eclipse Adoptium\jdk-17\bin\java.exe",
            r"C:\Program Files\Eclipse Adoptium\jdk-21\bin\java.exe",
            r"C:\Program Files\Eclipse Adoptium\jdk-25\bin\java.exe",
        ]
    } else {
        &[
            "/usr/lib/jvm/default-java/bin/java",
            "/usr/lib/jvm/java-8-openjdk/bin/java",
            "/usr/lib/jvm/java-17-openjdk/bin/java",
            "/usr/lib/jvm/java-21-openjdk/bin/java",
            "/usr/lib/jvm/java-25-openjdk/bin/java",
            "/usr/lib/jvm/java-17-oracle/bin/java",
            "/usr/lib/jvm/java-21-oracle/bin/java",
            "/usr/lib/jvm/java-25-oracle/bin/java",
            "/opt/java/bin/java",
            "/usr/bin/java",
        ]
    };

    for candidate in candidates {
        if Path::new(candidate).is_file() {
            if validate_java_path(candidate).is_ok() {
                return Ok(candidate.to_string());
            }
        }
    }

    if let Ok(path_env) = std::env::var("PATH") {
        let separator = if cfg!(target_os = "windows") {
            ';'
        } else {
            ':'
        };

        let executable = if cfg!(target_os = "windows") {
            "java.exe"
        } else {
            "java"
        };

        for directory in path_env
            .split(separator)
            .filter(|directory| !directory.is_empty())
        {
            let candidate = Path::new(directory).join(executable);

            if candidate.is_file() {
                let candidate_string = candidate.to_string_lossy().into_owned();

                if validate_java_path(&candidate_string).is_ok() {
                    return Ok(candidate_string);
                }
            }
        }
    }

    Err("Nie znaleziono poprawnej instalacji Java".to_string())
}

/// Jaka major-Java jest wymagana dla danej wersji Minecraft:
/// <=1.16 -> 8, 1.17-1.20.4 -> 17, >=1.20.5 -> 21.
fn mc_java_major(version_id: &str) -> u8 {
    let mut parts = version_id.split('.');
    let major: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(1);
    let minor: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let patch: u32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);

    if major == 1 && minor <= 16 {
        8
    } else if major == 1 && (minor < 20 || (minor == 20 && patch < 5)) {
        17
    } else {
        21
    }
}

/// Major wersji Javy spod danej ścieżki (np. 8 / 17 / 21), None gdy nie da się ustalić.
fn java_version_major(java_path: &str) -> Option<u32> {
    let output = Command::new(java_path).arg("-version").output().ok()?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // format: openjdk version "21.0.8" ... albo: java version "1.8.0_451"
    let quoted = text.split('"').nth(1)?;
    let mut nums = quoted.split('.');
    let first: u32 = nums.next()?.parse().ok()?;
    if first == 1 {
        nums.next()?.parse().ok()
    } else {
        Some(first)
    }
}

fn managed_java_dir(major: u8) -> Result<PathBuf, String> {
    let base = dirs::data_dir()
        .ok_or_else(|| "Nie można znaleźć katalogu danych użytkownika".to_string())?;
    Ok(base.join("AmbadClient").join("java").join(major.to_string()))
}

fn java_exe_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    }
}

/// Szuka bin/java wprost w katalogu albo w jednym podkatalogu (tak pakuje Adoptium).
fn find_java_in(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join("bin").join(java_exe_name());
    if direct.is_file() {
        return Some(direct);
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let candidate = entry.path().join("bin").join(java_exe_name());
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn adoptium_target() -> Result<(&'static str, &'static str, bool), String> {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return Err("Automatyczna instalacja Javy nie obsługuje tego systemu".to_string());
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => {
            return Err(format!(
                "Automatyczna instalacja Javy nie obsługuje architektury {other}"
            ))
        }
    };
    Ok((os, arch, os == "windows"))
}

fn adoptium_url(major: u8, os: &str, arch: &str) -> String {
    format!("https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{arch}/jre/hotspot/normal/eclipse")
}

fn extract_archive(archive: &Path, dest: &Path, is_zip: bool) -> Result<(), String> {
    if is_zip {
        let file = fs::File::open(archive)
            .map_err(|e| format!("Nie można otworzyć paczki Javy: {e}"))?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|e| format!("Uszkodzona paczka Javy: {e}"))?;
        zip.extract(dest)
            .map_err(|e| format!("Nie można wypakować Javy: {e}"))?;
    } else {
        let file = fs::File::open(archive)
            .map_err(|e| format!("Nie można otworzyć paczki Javy: {e}"))?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(gz);
        tar.unpack(dest)
            .map_err(|e| format!("Nie można wypakować Javy: {e}"))?;
    }
    Ok(())
}

async fn download_adoptium_java(major: u8) -> Result<PathBuf, String> {
    let (os, arch, is_zip) = adoptium_target()?;
    let url = adoptium_url(major, os, arch);
    let dest = managed_java_dir(major)?;

    let _ = fs::remove_dir_all(&dest);
    fs::create_dir_all(&dest).map_err(|e| format!("Nie można utworzyć katalogu Javy: {e}"))?;

    let tmp = dest.with_extension(if is_zip { "zip.part" } else { "tar.gz.part" });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| format!("Nie można przygotować pobierania Javy: {e}"))?;
    let mut response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Nie można pobrać Javy {major} (sprawdź połączenie z internetem): {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Serwer Javy zwrócił HTTP {}",
            response.status().as_u16()
        ));
    }
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("Nie można zapisać paczki Javy: {e}"))?;
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Pobieranie Javy przerwane: {e}"))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Zapis paczki Javy nieudany: {e}"))?;
    }
    file.flush().await.ok();
    drop(file);

    let extract_to = dest.join("_new");
    let _ = fs::remove_dir_all(&extract_to);
    fs::create_dir_all(&extract_to)
        .map_err(|e| format!("Nie można utworzyć katalogu Javy: {e}"))?;
    if let Err(e) = extract_archive(&tmp, &extract_to, is_zip) {
        let _ = fs::remove_dir_all(&extract_to);
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    let _ = fs::remove_file(&tmp);

    // Adoptium pakuje jeden katalog jdk-...-jre — przenieś jego zawartość wprost do dest.
    let entries: Vec<_> = fs::read_dir(&extract_to)
        .map_err(|e| format!("Nie można odczytać paczki Javy: {e}"))?
        .flatten()
        .collect();
    if entries.len() == 1 && entries[0].path().is_dir() {
        let inner = entries[0].path();
        fs::remove_dir_all(&dest).ok();
        fs::rename(&inner, &dest).map_err(|e| format!("Nie można zainstalować Javy: {e}"))?;
        let _ = fs::remove_dir_all(&extract_to);
    } else {
        fs::remove_dir_all(&dest).ok();
        fs::rename(&extract_to, &dest).map_err(|e| format!("Nie można zainstalować Javy: {e}"))?;
    }

    #[cfg(unix)]
    if let Some(java) = find_java_in(&dest) {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&java) {
            let mut perm = meta.permissions();
            perm.set_mode(0o755);
            let _ = fs::set_permissions(&java, perm);
        }
    }

    find_java_in(&dest).ok_or_else(|| "Paczka Javy nie zawiera bin/java".to_string())
}

/// Zwraca działającą Javę dla danej wersji MC: własna (pobrana) > systemowa, pobiera gdy brak.
async fn ensure_java_inner(version_id: &str) -> Result<String, String> {
    let required = mc_java_major(version_id);

    // 1. własna, wcześniej pobrana
    if let Ok(dir) = managed_java_dir(required) {
        if let Some(java) = find_java_in(&dir) {
            let path = java.to_string_lossy().into_owned();
            if validate_java_path(&path).is_ok() {
                return Ok(path);
            }
        }
    }

    // 2. systemowa, o ile wystarczająco nowa dla tej wersji MC
    if let Ok(system) = detect_java().await {
        let fresh_enough = java_version_major(&system)
            .map(|v| v >= required as u32)
            .unwrap_or(true);
        if fresh_enough {
            return Ok(system);
        }
    }

    // 3. pobierz przenośną (bez uprawnień admina, do folderu launchera)
    let java = download_adoptium_java(required).await?;
    let path = java.to_string_lossy().into_owned();
    validate_java_path(&path)?;
    Ok(path)
}

#[command]
async fn ensure_java(version_id: String) -> Result<String, String> {
    validate_version_id(&version_id)?;
    ensure_java_inner(&version_id).await
}

#[cfg(test)]
mod java_tests {
    use super::*;

    #[test]
    fn java_major_mapping() {
        assert_eq!(mc_java_major("1.8.9"), 8);
        assert_eq!(mc_java_major("1.12.2"), 8);
        assert_eq!(mc_java_major("1.16.5"), 8);
        assert_eq!(mc_java_major("1.17.1"), 17);
        assert_eq!(mc_java_major("1.20.4"), 17);
        assert_eq!(mc_java_major("1.20.5"), 21);
        assert_eq!(mc_java_major("1.21"), 21);
        assert_eq!(mc_java_major("1.21.4"), 21);
    }

    #[test]
    fn adoptium_url_shape() {
        let url = adoptium_url(21, "windows", "x64");
        assert!(url.starts_with(
            "https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jre/"
        ));
    }
}

#[command]
async fn get_minecraft_profile() -> Result<Option<MinecraftProfile>, String> {
    match load_auth() {
        None => Ok(None),
        Some(auth) if token_is_valid(&auth) => Ok(Some(auth.minecraft_profile)),
        Some(auth) => match refresh_token(&auth).await {
            Ok(new_auth) => Ok(Some(new_auth.minecraft_profile)),
            Err(_) => {
                clear_auth();
                Ok(None)
            }
        },
    }
}

#[command]
async fn is_authenticated() -> Result<bool, String> {
    match load_auth() {
        None => Ok(false),
        Some(auth) if token_is_valid(&auth) => Ok(true),
        Some(auth) => match refresh_token(&auth).await {
            Ok(_) => Ok(true),
            Err(_) => {
                clear_auth();
                Ok(false)
            }
        },
    }
}

#[command]
async fn microsoft_logout() -> Result<(), String> {
    clear_auth();
    Ok(())
}

#[command]
async fn microsoft_login(app: tauri::AppHandle) -> Result<MinecraftProfile, String> {
    use base64::Engine;
    use rand::RngCore;
    use sha2::{Digest, Sha256};

    if let Some(window) = app.get_webview_window("ms-login") {
        let _ = window.close();
    }

    let mut verifier_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);

    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(verifier_bytes);

    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));

    let auth_url = format!(
        "{LIVE_AUTH}?client_id={MC_CLIENT_ID}&response_type=code&redirect_uri={}&scope={}&code_challenge={challenge}&code_challenge_method=S256",
        urlencoding::encode(DESKTOP_REDIRECT),
        urlencoding::encode(MC_SCOPE)
    );

    let (sender, receiver) =
        tokio::sync::oneshot::channel::<Result<String, String>>();

    let sender = Arc::new(Mutex::new(Some(sender)));
    let navigation_sender = Arc::clone(&sender);

    let url = auth_url
        .parse()
        .map_err(|e| format!("Nieprawidłowy URL logowania: {e}"))?;

    let window = tauri::WebviewWindowBuilder::new(
        &app,
        "ms-login",
        WebviewUrl::External(url),
    )
    .title("Zaloguj się przez Microsoft")
    .inner_size(480.0, 640.0)
    .center()
    .resizable(false)
    .on_navigation(move |url| {
        let value = url.to_string();

        let is_redirect = value.starts_with(DESKTOP_REDIRECT)
            || value.starts_with("ms-appx-web://Microsoft.AAD");

        if !is_redirect {
            return true;
        }

        let result = if let Some(code) = extract_param(&value, "code") {
            Ok(code)
        } else if let Some(error) = extract_param(&value, "error") {
            Err(format!(
                "Microsoft odrzucił logowanie: {error}"
            ))
        } else if let Some(description) =
            extract_param(&value, "error_description")
        {
            Err(format!(
                "Microsoft odrzucił logowanie: {description}"
            ))
        } else {
            Err("Brak kodu autoryzacji".to_string())
        };

        if let Ok(mut guard) = navigation_sender.lock() {
            if let Some(sender) = guard.take() {
                let _ = sender.send(result);
            }
        }

        false
    })
    .build()
    .map_err(|e| format!("Nie można otworzyć okna logowania: {e}"))?;

    let result =
        tokio::time::timeout(Duration::from_secs(300), receiver).await;

    let _ = window.close();

    let code = result
        .map_err(|_| "Logowanie wygasło; spróbuj ponownie".to_string())?
        .map_err(|_| "Błąd kanału autoryzacji".to_string())??;

    let client = reqwest::Client::new();

    let response = client
        .post(LIVE_TOKEN)
        .form(&[
            ("client_id", MC_CLIENT_ID),
            ("code", code.as_str()),
            ("redirect_uri", DESKTOP_REDIRECT),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier.as_str()),
            ("scope", MC_SCOPE),
        ])
        .send()
        .await
        .map_err(|e| format!("Błąd połączenia z Microsoft: {e}"))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        let detail: String = body.chars().take(200).collect();

        return Err(format!(
            "Wymiana tokenu nieudana (HTTP {status}): {detail}"
        ));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Token parse error: {e}"))?;

    let access_token = data["access_token"]
        .as_str()
        .ok_or_else(|| "Brak access_token".to_string())?;

    let refresh_token = data["refresh_token"]
        .as_str()
        .unwrap_or_default();

    if refresh_token.is_empty() {
        return Err("Microsoft nie zwrócił refresh tokenu".to_string());
    }

    let expires_in = data["expires_in"]
        .as_i64()
        .unwrap_or(86400)
        .clamp(60, 7 * 24 * 60 * 60);

    let (minecraft_token, profile) =
        exchange_xbox_to_minecraft(&client, access_token).await?;

    save_auth(&StoredAuth {
        minecraft_token,
        minecraft_profile: profile.clone(),
        refresh_token: refresh_token.to_string(),
        expires_at: (chrono::Utc::now()
            + chrono::Duration::seconds(expires_in))
        .to_rfc3339(),
    })?;

    Ok(profile)
}

async fn download_file(client: &reqwest::Client, url: &str, dest: &Path) -> Result<(), String> {
    if dest.is_file() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Nie można utworzyć katalogu {}: {e}", parent.display()))?;
    }
    let bytes = client.get(url).send().await.map_err(|e| format!("Pobieranie {url} nieudane: {e}"))?
        .bytes().await.map_err(|e| format!("Odczyt {url} nieudany: {e}"))?;
    fs::write(dest, &bytes).map_err(|e| format!("Zapis {} nieudany: {e}", dest.display()))?;
    Ok(())
}

async fn ensure_version_installed(version_id: &str) -> Result<(), String> {
    let mc_dir = minecraft_dir()?;
    let version_dir = mc_dir.join("versions").join(version_id);
    let version_json = version_dir.join(format!("{version_id}.json"));
    let client_jar = version_dir.join(format!("{version_id}.jar"));

    if version_json.is_file() && client_jar.is_file() {
        // quick check libs?
        return Ok(());
    }

    let client = reqwest::Client::builder().timeout(Duration::from_secs(60)).build().map_err(|e| e.to_string())?;

    // 1. manifest -> znajdź url wersji
    let manifest: serde_json::Value = client.get(VERSION_MANIFEST_URL).send().await.map_err(|e| format!("Manifest pobieranie: {e}"))?
        .json().await.map_err(|e| format!("Manifest parse: {e}"))?;
    let entry = manifest["versions"].as_array().ok_or("Zły manifest")?
        .iter().find(|v| v["id"].as_str()==Some(version_id)).ok_or(format!("Wersja {version_id} nie znaleziona"))?;
    let url = entry["url"].as_str().ok_or("Brak url wersji")?;

    // 2. pobierz version json
    let vjson: serde_json::Value = client.get(url).send().await.map_err(|e| format!("Version json pobieranie: {e}"))?
        .json().await.map_err(|e| format!("Version json parse: {e}"))?;
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;
    fs::write(&version_json, serde_json::to_string_pretty(&vjson).unwrap()).map_err(|e| e.to_string())?;

    // 3. client jar
    let jar_url = vjson["downloads"]["client"]["url"].as_str().ok_or("Brak client url")?;
    download_file(&client, jar_url, &client_jar).await?;

    // 4. libraries
    if let Some(libs) = vjson["libraries"].as_array() {
        for lib in libs {
            if let Some(artifact) = lib["downloads"].get("artifact") {
                if let (Some(path), Some(url)) = (artifact["path"].as_str(), artifact["url"].as_str()) {
                    let dest = mc_dir.join("libraries").join(path);
                    // skip if exists
                    if dest.is_file() { continue; }
                    // rules — uprośc: pobieraj wszystko
                    let _ = download_file(&client, url, &dest).await;
                }
            }
        }
    }

    // 5. assetIndex
    if let (Some(ai_url), Some(ai_id)) = (vjson["assetIndex"]["url"].as_str(), vjson["assetIndex"]["id"].as_str()) {
        let idx_path = mc_dir.join("assets").join("indexes").join(format!("{ai_id}.json"));
        download_file(&client, ai_url, &idx_path).await?;
        // 6. assets objects (pobierz wszystkie — może potrwać)
        if let Ok(idx_text) = fs::read_to_string(&idx_path) {
            if let Ok(idx) = serde_json::from_str::<serde_json::Value>(&idx_text) {
                if let Some(objects) = idx["objects"].as_object() {
                    for (_, obj) in objects.iter() {
                        if let Some(hash) = obj["hash"].as_str() {
                            let sub = &hash[0..2];
                            let dest = mc_dir.join("assets").join("objects").join(sub).join(hash);
                            if dest.is_file() { continue; }
                            let url = format!("https://resources.download.minecraft.net/{sub}/{hash}");
                            let _ = download_file(&client, &url, &dest).await;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[command]
async fn launch_minecraft(options: LaunchOptions) -> Result<(), String> {
    validate_version_id(&options.version_id)?;

    // Brak Javy na PC albo za stara? — dociągnij przenośną automatycznie zamiast rzucać błędem.
    let java_path = match validate_java_path(&options.java_path) {
        Ok(path) => path,
        Err(_) => ensure_java_inner(&options.version_id).await?,
    };

    let max_memory = options.max_memory.clamp(512, 32768);
    let min_memory = options.min_memory.clamp(512, max_memory);

    let minecraft_dir = minecraft_dir()?;
    // auto-install jeśli brak
    ensure_version_installed(&options.version_id).await?;

    let version_dir = minecraft_dir.join("versions").join(&options.version_id);

    let version_json = version_dir.join(format!("{}.json", options.version_id));
    let client_jar = version_dir.join(format!("{}.jar", options.version_id));
    let natives_dir = version_dir.join("natives");

    if !version_json.is_file() {
        return Err(format!(
            "Brak pliku {}",
            version_json.display()
        ));
    }

    if !client_jar.is_file() {
        return Err(format!(
            "Brak pliku {}",
            client_jar.display()
        ));
    }

    let version_data = fs::read_to_string(&version_json)
        .map_err(|e| format!("Nie można odczytać pliku wersji: {e}"))?;

    let info: serde_json::Value = serde_json::from_str(&version_data)
        .map_err(|e| format!("Nie można sparsować pliku wersji: {e}"))?;

    let main_class = info["mainClass"]
        .as_str()
        .unwrap_or("net.minecraft.client.main.Main");

    if !main_class.split('.').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    }) {
        return Err("Nieprawidłowa mainClass".to_string());
    }

    let mut classpath = Vec::new();

    if let Some(libraries) = info["libraries"].as_array() {
        for library in libraries {
            let path = match library["downloads"]["artifact"]["path"].as_str() {
                Some(path) => path,
                None => continue,
            };

            let relative_path = Path::new(path);

            if relative_path.is_absolute()
                || relative_path
                    .components()
                    .any(|component| {
                        matches!(
                            component,
                            std::path::Component::ParentDir
                        )
                    })
            {
                return Err(format!(
                    "Nieprawidłowa ścieżka biblioteki: {path}"
                ));
            }

            let library_path = minecraft_dir.join("libraries").join(relative_path);

            if !library_path.is_file() {
                return Err(format!(
                    "Brak wymaganej biblioteki: {}",
                    library_path.display()
                ));
            }

            classpath.push(library_path.to_string_lossy().into_owned());
        }
    }

    classpath.push(client_jar.to_string_lossy().into_owned());

    let classpath_separator = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };

    let username = options
        .username
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(str::to_string);

    let auth = if options.offline {
        None
    } else {
        Some(match load_auth() {
            None => {
                return Err(
                    "Nie zalogowano przez Microsoft".to_string()
                )
            }
            Some(auth) if token_is_valid(&auth) => auth,
            Some(auth) => refresh_token(&auth)
                .await
                .map_err(|_| {
                    "Sesja wygasła; zaloguj się ponownie".to_string()
                })?,
        })
    };

    let username = username
        .or_else(|| {
            auth.as_ref()
                .map(|auth| auth.minecraft_profile.name.clone())
        })
        .unwrap_or_else(|| "Player".to_string());

    validate_username(&username)?;

    let uuid = auth
        .as_ref()
        .map(|auth| auth.minecraft_profile.id.clone())
        .unwrap_or_else(|| {
            "00000000-0000-0000-0000-000000000000".to_string()
        });

    let access_token = auth
        .as_ref()
        .map(|auth| auth.minecraft_token.clone())
        .unwrap_or_else(|| "0".to_string());

    let assets_dir = minecraft_dir.join("assets");

    let asset_index = info["assetIndex"]["id"]
        .as_str()
        .unwrap_or(&options.version_id);

    // Brand widoczny w F3 jak u Lunara — zamiast "release" pokaże "AMBAD"
    let _vanilla_type = info["type"].as_str().unwrap_or("release");
    let version_type = "AMBAD";

    let natives_path = natives_dir.to_string_lossy().into_owned();

    let mut args = vec![
        format!("-Xms{min_memory}M"),
        format!("-Xmx{max_memory}M"),
        "-XX:+UseG1GC".to_string(),
        "-XX:MaxGCPauseMillis=200".to_string(),
        "-XX:+DisableExplicitGC".to_string(),
        "-Dfile.encoding=UTF-8".to_string(),
        "-Dminecraft.launcher.brand=AMBAD".to_string(),
        "-Dminecraft.launcher.version=1.0.0".to_string(),
        format!("-Djava.library.path={natives_path}"),
        "-cp".to_string(),
        classpath.join(classpath_separator),
        main_class.to_string(),
        "--username".to_string(),
        username,
        "--version".to_string(),
        options.version_id.clone(),
        "--gameDir".to_string(),
        minecraft_dir.to_string_lossy().into_owned(),
        "--assetsDir".to_string(),
        assets_dir.to_string_lossy().into_owned(),
        "--assetIndex".to_string(),
        asset_index.to_string(),
        "--uuid".to_string(),
        uuid,
        "--accessToken".to_string(),
        access_token,
        "--userType".to_string(),
        if options.offline {
            "legacy".to_string()
        } else {
            "msa".to_string()
        },
        "--versionType".to_string(),
        version_type.to_string(),
    ];

    if options.fullscreen {
        args.push("--fullscreen".to_string());
    } else if options.width > 0 && options.height > 0 {
        args.push("--width".to_string());
        args.push(options.width.to_string());
        args.push("--height".to_string());
        args.push(options.height.to_string());
    }

    // — Własny Minecraft: gameDir = AmbadClient/minecraft, nie systemowy .minecraft
    // Blokada wielokrotnego odpalania — jeśli już działa, zwróć błąd
    {
        let proc = mc_process();
        let mut guard = proc.lock().unwrap();
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(None) => return Err("Minecraft już jest uruchomiony — użyj STOP".to_string()),
                _ => { *guard = None; } // proces zakończony, wyczyść
            }
        }
    }

    let child = Command::new(&java_path)
        .args(&args)
        .current_dir(&minecraft_dir)
        .spawn()
        .map_err(|e| format!("Nie można uruchomić Minecraft: {e}"))?;

    // zapisz child żeby umożliwić STOP i blokadę Graj (własny Minecraft z AmbadClient/minecraft)
    {
        let proc = mc_process();
        let mut guard = proc.lock().unwrap();
        *guard = Some(child);
    }

    Ok(())
}

#[command]
fn get_mods() -> Result<Vec<String>, String> {
    let dir = mods_dir()?;
    let entries = fs::read_dir(&dir).map_err(|e| format!("Nie można odczytać mods: {e}"))?;
    let mut mods: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let ft = e.file_type().ok()?;
            if !ft.is_file() {
                return None;
            }
            let name = e.file_name().to_str()?.to_string();
            if name.ends_with(".jar") || name.ends_with(".disabled") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    mods.sort();
    Ok(mods)
}

#[command]
fn open_mods_folder() -> Result<(), String> {
    let dir = mods_dir()?;
    std::fs::create_dir_all(&dir).ok();
    open::that(&dir).map_err(|e| format!("Nie można otworzyć folderu mods: {e}"))?;
    Ok(())
}

#[command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[command]
fn get_minecraft_dir() -> Result<String, String> {
    Ok(minecraft_dir()?.to_string_lossy().into_owned())
}

#[command]
fn stop_minecraft() -> Result<(), String> {
    let proc = mc_process();
    let mut guard = proc.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(None) => {
                child.kill().map_err(|e| format!("Nie można zatrzymać Minecraft: {e}"))?;
                *guard = None;
                Ok(())
            },
            _ => {
                *guard = None;
                Err("Minecraft już nie działa".to_string())
            }
        }
    } else {
        Err("Minecraft nie jest uruchomiony".to_string())
    }
}

#[command]
fn is_minecraft_running() -> Result<bool, String> {
    let proc = mc_process();
    let mut guard = proc.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(None) => Ok(true),
            Ok(Some(_)) => { *guard = None; Ok(false) },
            Err(_) => { *guard = None; Ok(false) }
        }
    } else {
        Ok(false)
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_versions,
            detect_java,
            ensure_java,
            launch_minecraft,
            microsoft_login,
            microsoft_logout,
            get_minecraft_profile,
            is_authenticated,
            get_mods,
            open_mods_folder,
            get_minecraft_dir,
            get_app_version,
            stop_minecraft,
            is_minecraft_running
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}