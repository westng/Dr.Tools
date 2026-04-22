use chrono::Utc;
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tar::Archive;
use tauri::{AppHandle, Manager, State};

use crate::application::AppState;
use crate::domain::{
  AppSettings, AppSettingsPatch, ManagedEnvironmentStatus, TokenValidationPayload, TokenValidationResult,
  UpdateCheckResult,
};
use crate::error::AppError;
use crate::services::configure_background_command;
use crate::services::python::{
  managed_ffmpeg_bin_path, managed_ffmpeg_root, managed_ffmpeg_version, managed_runtime_bin_path,
  managed_runtime_root, MANAGED_FFMPEG_ARM64_MACOS_VERSION, MANAGED_FFMPEG_SOURCE_BASE_URL,
  MANAGED_FFMPEG_SOURCE_LABEL, MANAGED_FFMPEG_VERSION, MANAGED_PYTHON_RELEASE,
  MANAGED_PYTHON_SOURCE_BASE_URL, MANAGED_PYTHON_SOURCE_LABEL, MANAGED_PYTHON_VERSION,
};

const KEY_THEME_MODE: &str = "settings.theme_mode";
const KEY_LIQUID_GLASS_STYLE: &str = "settings.liquid_glass_style";
const KEY_ACCENT_COLOR: &str = "settings.accent_color";
const KEY_LOCALE: &str = "settings.locale";
const KEY_EXPORT_DIR: &str = "settings.export_dir";
const KEY_MAX_CONCURRENT_DOWNLOADS: &str = "settings.max_concurrent_downloads";
const KEY_DOWNLOAD_NOTIFICATIONS_ENABLED: &str = "settings.download_notifications_enabled";
const KEY_DOUYIN_COOKIE: &str = "settings.douyin_cookie";
const KEY_DOUYIN_COOKIE_UPDATED_AT: &str = "settings.douyin_cookie_updated_at";
const KEY_DOUYIN_LAST_CHECKED_AT: &str = "settings.douyin_last_checked_at";
const KEY_DOUYIN_LAST_CHECK_STATUS: &str = "settings.douyin_last_check_status";
const KEY_DOUYIN_LAST_CHECK_MESSAGE: &str = "settings.douyin_last_check_message";
const KEY_TIKTOK_COOKIE: &str = "settings.tiktok_cookie";
const KEY_TIKTOK_COOKIE_UPDATED_AT: &str = "settings.tiktok_cookie_updated_at";
const KEY_TIKTOK_LAST_CHECKED_AT: &str = "settings.tiktok_last_checked_at";
const KEY_TIKTOK_LAST_CHECK_STATUS: &str = "settings.tiktok_last_check_status";
const KEY_TIKTOK_LAST_CHECK_MESSAGE: &str = "settings.tiktok_last_check_message";
const KEY_AUTO_CHECK_UPDATES: &str = "settings.auto_check_updates";
const KEY_LAST_UPDATE_CHECK_AT: &str = "settings.last_update_check_at";
const KEY_LAST_UPDATE_STATUS: &str = "settings.last_update_status";
const PYPI_MIRROR_URL: &str = "https://pypi.tuna.tsinghua.edu.cn/simple";

const DEFAULT_THEME_MODE: &str = "auto";
const DEFAULT_LIQUID_GLASS_STYLE: &str = "transparent";
const DEFAULT_ACCENT_COLOR: &str = "#2f6dff";
const DEFAULT_LOCALE: &str = "zh-CN";
const DEFAULT_MAX_CONCURRENT_DOWNLOADS: u32 = 3;
const STATUS_NOT_CONFIGURED: &str = "not_configured";
const STATUS_UNCHECKED: &str = "unchecked";

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>, app: AppHandle) -> Result<AppSettings, AppError> {
  load_settings(&state, &app)
}

#[tauri::command]
pub fn settings_update(
  payload: AppSettingsPatch,
  state: State<'_, AppState>,
  app: AppHandle,
) -> Result<AppSettings, AppError> {
  if let Some(theme_mode) = payload.theme_mode.as_ref() {
    validate_theme_mode(theme_mode)?;
    state.db.set_meta(KEY_THEME_MODE, theme_mode.trim())?;
  }

  if let Some(style) = payload.liquid_glass_style.as_ref() {
    validate_liquid_glass_style(style)?;
    state.db.set_meta(KEY_LIQUID_GLASS_STYLE, style.trim())?;
  }

  if let Some(color) = payload.accent_color.as_ref() {
    validate_accent_color(color)?;
    state.db.set_meta(KEY_ACCENT_COLOR, color.trim())?;
  }

  if let Some(locale) = payload.locale.as_ref() {
    validate_locale(locale)?;
    state.db.set_meta(KEY_LOCALE, locale.trim())?;
  }

  if let Some(export_dir) = payload.export_dir.as_ref() {
    state.db.set_meta(KEY_EXPORT_DIR, export_dir.trim())?;
  }

  if let Some(value) = payload.max_concurrent_downloads {
    validate_max_concurrent_downloads(value)?;
    state
      .db
      .set_meta(KEY_MAX_CONCURRENT_DOWNLOADS, &value.to_string())?;
  }

  if let Some(enabled) = payload.download_notifications_enabled {
    state.db.set_meta(
      KEY_DOWNLOAD_NOTIFICATIONS_ENABLED,
      if enabled { "true" } else { "false" },
    )?;
  }

  if let Some(cookie) = payload.douyin_cookie.as_ref() {
    let normalized = normalize_download_cookie(cookie);
    state.db.set_meta(KEY_DOUYIN_COOKIE, &normalized)?;
    if normalized.is_empty() {
      clear_token_state(&state, KEY_DOUYIN_COOKIE_UPDATED_AT, KEY_DOUYIN_LAST_CHECKED_AT, KEY_DOUYIN_LAST_CHECK_STATUS, KEY_DOUYIN_LAST_CHECK_MESSAGE)?;
    } else {
      let now = Utc::now().to_rfc3339();
      state.db.set_meta(KEY_DOUYIN_COOKIE_UPDATED_AT, &now)?;
      state.db.set_meta(KEY_DOUYIN_LAST_CHECKED_AT, "")?;
      state.db.set_meta(KEY_DOUYIN_LAST_CHECK_STATUS, STATUS_UNCHECKED)?;
      state.db.set_meta(KEY_DOUYIN_LAST_CHECK_MESSAGE, "")?;
    }
  }

  if let Some(cookie) = payload.tiktok_cookie.as_ref() {
    let normalized = normalize_download_cookie(cookie);
    state.db.set_meta(KEY_TIKTOK_COOKIE, &normalized)?;
    if normalized.is_empty() {
      clear_token_state(&state, KEY_TIKTOK_COOKIE_UPDATED_AT, KEY_TIKTOK_LAST_CHECKED_AT, KEY_TIKTOK_LAST_CHECK_STATUS, KEY_TIKTOK_LAST_CHECK_MESSAGE)?;
    } else {
      let now = Utc::now().to_rfc3339();
      state.db.set_meta(KEY_TIKTOK_COOKIE_UPDATED_AT, &now)?;
      state.db.set_meta(KEY_TIKTOK_LAST_CHECKED_AT, "")?;
      state.db.set_meta(KEY_TIKTOK_LAST_CHECK_STATUS, STATUS_UNCHECKED)?;
      state.db.set_meta(KEY_TIKTOK_LAST_CHECK_MESSAGE, "")?;
    }
  }

  if let Some(enabled) = payload.auto_check_updates {
    state.db.set_meta(KEY_AUTO_CHECK_UPDATES, if enabled { "true" } else { "false" })?;
  }

  load_settings(&state, &app)
}

#[tauri::command]
pub fn token_validate(
  payload: TokenValidationPayload,
  state: State<'_, AppState>,
  app: AppHandle,
) -> Result<TokenValidationResult, AppError> {
  validate_download_platform(&payload.platform)?;

  let normalized = normalize_download_cookie(&payload.cookie);
  if normalized.is_empty() {
    return Err(AppError::Validation("cookie is required".to_string()));
  }

  let result = state.python.request_isolated(
    &app,
    "run_task",
    serde_json::json!({
      "task_type": "token.validate",
      "payload": {
        "platform": payload.platform,
        "cookie": normalized,
      },
    }),
  )?;

  let validation: TokenValidationResult = serde_json::from_value(result)?;
  persist_token_validation(&state, &validation)?;
  Ok(validation)
}

#[tauri::command]
pub fn settings_check_update(state: State<'_, AppState>) -> Result<UpdateCheckResult, AppError> {
  let checked_at = Utc::now().to_rfc3339();
  let current_version = env!("CARGO_PKG_VERSION").to_string();
  let latest_version = current_version.clone();
  let status = "up-to-date".to_string();
  let message = "当前已是最新版本".to_string();

  state.db.set_meta(KEY_LAST_UPDATE_CHECK_AT, &checked_at)?;
  state.db.set_meta(KEY_LAST_UPDATE_STATUS, &status)?;

  Ok(UpdateCheckResult {
    checked_at,
    status,
    message,
    current_version,
    latest_version,
  })
}

#[tauri::command]
pub fn environment_status(app: AppHandle) -> Result<ManagedEnvironmentStatus, AppError> {
  read_environment_status(&app)
}

#[tauri::command]
pub async fn environment_download(app: AppHandle) -> Result<ManagedEnvironmentStatus, AppError> {
  tauri::async_runtime::spawn_blocking(move || install_managed_environment(&app))
    .await
    .map_err(|error| AppError::TaskExec(format!("environment install task failed: {}", error)))?
}

#[tauri::command]
pub fn select_export_directory(current: Option<String>) -> Option<String> {
  let mut dialog = rfd::FileDialog::new();
  if let Some(path) = current {
    let trimmed = path.trim();
    if !trimmed.is_empty() {
      dialog = dialog.set_directory(trimmed);
    }
  }

  dialog
    .pick_folder()
    .map(|path| path.to_string_lossy().to_string())
}

fn load_settings(state: &AppState, app: &AppHandle) -> Result<AppSettings, AppError> {
  let mut settings = AppSettings {
    theme_mode: DEFAULT_THEME_MODE.to_string(),
    liquid_glass_style: DEFAULT_LIQUID_GLASS_STYLE.to_string(),
    accent_color: DEFAULT_ACCENT_COLOR.to_string(),
    locale: DEFAULT_LOCALE.to_string(),
    export_dir: default_export_dir(app),
    max_concurrent_downloads: DEFAULT_MAX_CONCURRENT_DOWNLOADS,
    download_notifications_enabled: true,
    douyin_cookie: String::new(),
    douyin_cookie_updated_at: None,
    douyin_last_checked_at: None,
    douyin_last_check_status: STATUS_NOT_CONFIGURED.to_string(),
    douyin_last_check_message: None,
    tiktok_cookie: String::new(),
    tiktok_cookie_updated_at: None,
    tiktok_last_checked_at: None,
    tiktok_last_check_status: STATUS_NOT_CONFIGURED.to_string(),
    tiktok_last_check_message: None,
    auto_check_updates: true,
    last_update_check_at: None,
    last_update_status: None,
  };

  if let Some(theme_mode) = state.db.get_meta(KEY_THEME_MODE)? {
    if validate_theme_mode(&theme_mode).is_ok() {
      settings.theme_mode = theme_mode;
    }
  }

  if let Some(style) = state.db.get_meta(KEY_LIQUID_GLASS_STYLE)? {
    if validate_liquid_glass_style(&style).is_ok() {
      settings.liquid_glass_style = style;
    }
  }

  if let Some(color) = state.db.get_meta(KEY_ACCENT_COLOR)? {
    if validate_accent_color(&color).is_ok() {
      settings.accent_color = color;
    }
  }

  if let Some(locale) = state.db.get_meta(KEY_LOCALE)? {
    if validate_locale(&locale).is_ok() {
      settings.locale = locale;
    }
  }

  if let Some(export_dir) = state.db.get_meta(KEY_EXPORT_DIR)? {
    settings.export_dir = export_dir;
  }

  if let Some(raw) = state.db.get_meta(KEY_MAX_CONCURRENT_DOWNLOADS)? {
    if let Ok(value) = raw.trim().parse::<u32>() {
      if validate_max_concurrent_downloads(value).is_ok() {
        settings.max_concurrent_downloads = value;
      }
    }
  }

  if let Some(raw) = state.db.get_meta(KEY_DOWNLOAD_NOTIFICATIONS_ENABLED)? {
    settings.download_notifications_enabled = parse_bool(&raw).unwrap_or(true);
  }

  if let Some(cookie) = state.db.get_meta(KEY_DOUYIN_COOKIE)? {
    settings.douyin_cookie = normalize_download_cookie(&cookie);
  }
  settings.douyin_cookie_updated_at = read_optional_meta(state, KEY_DOUYIN_COOKIE_UPDATED_AT)?;
  settings.douyin_last_checked_at = read_optional_meta(state, KEY_DOUYIN_LAST_CHECKED_AT)?;
  settings.douyin_last_check_status = read_optional_meta(state, KEY_DOUYIN_LAST_CHECK_STATUS)?
    .unwrap_or_else(|| {
      if settings.douyin_cookie.is_empty() {
        STATUS_NOT_CONFIGURED.to_string()
      } else {
        STATUS_UNCHECKED.to_string()
      }
    });
  settings.douyin_last_check_message = read_optional_meta(state, KEY_DOUYIN_LAST_CHECK_MESSAGE)?;

  if let Some(cookie) = state.db.get_meta(KEY_TIKTOK_COOKIE)? {
    settings.tiktok_cookie = normalize_download_cookie(&cookie);
  }
  settings.tiktok_cookie_updated_at = read_optional_meta(state, KEY_TIKTOK_COOKIE_UPDATED_AT)?;
  settings.tiktok_last_checked_at = read_optional_meta(state, KEY_TIKTOK_LAST_CHECKED_AT)?;
  settings.tiktok_last_check_status = read_optional_meta(state, KEY_TIKTOK_LAST_CHECK_STATUS)?
    .unwrap_or_else(|| {
      if settings.tiktok_cookie.is_empty() {
        STATUS_NOT_CONFIGURED.to_string()
      } else {
        STATUS_UNCHECKED.to_string()
      }
    });
  settings.tiktok_last_check_message = read_optional_meta(state, KEY_TIKTOK_LAST_CHECK_MESSAGE)?;

  if let Some(raw) = state.db.get_meta(KEY_AUTO_CHECK_UPDATES)? {
    settings.auto_check_updates = parse_bool(&raw).unwrap_or(true);
  }

  settings.last_update_check_at = state.db.get_meta(KEY_LAST_UPDATE_CHECK_AT)?;
  settings.last_update_status = state.db.get_meta(KEY_LAST_UPDATE_STATUS)?;

  Ok(settings)
}

fn read_optional_meta(state: &AppState, key: &str) -> Result<Option<String>, AppError> {
  Ok(state.db.get_meta(key)?.and_then(|value| {
    let trimmed = value.trim();
    if trimmed.is_empty() {
      None
    } else {
      Some(trimmed.to_string())
    }
  }))
}

fn parse_bool(raw: &str) -> Option<bool> {
  match raw.trim().to_ascii_lowercase().as_str() {
    "true" | "1" | "yes" | "on" => Some(true),
    "false" | "0" | "no" | "off" => Some(false),
    _ => None,
  }
}

fn default_export_dir(app: &AppHandle) -> String {
  app
    .path()
    .download_dir()
    .ok()
    .or_else(|| app.path().home_dir().ok())
    .map(|path| path.to_string_lossy().to_string())
    .unwrap_or_default()
}

fn validate_theme_mode(value: &str) -> Result<(), AppError> {
  match value.trim() {
    "auto" | "light" | "dark" => Ok(()),
    _ => Err(AppError::Validation(
      "themeMode must be one of auto/light/dark".to_string(),
    )),
  }
}

fn validate_liquid_glass_style(value: &str) -> Result<(), AppError> {
  match value.trim() {
    "transparent" | "tinted" => Ok(()),
    _ => Err(AppError::Validation(
      "liquidGlassStyle must be one of transparent/tinted".to_string(),
    )),
  }
}

fn validate_locale(value: &str) -> Result<(), AppError> {
  match value.trim() {
    "zh-CN" | "en-US" => Ok(()),
    _ => Err(AppError::Validation(
      "locale must be one of zh-CN/en-US".to_string(),
    )),
  }
}

fn validate_accent_color(value: &str) -> Result<(), AppError> {
  let trimmed = value.trim();
  let valid = trimmed.len() == 7
    && trimmed.starts_with('#')
    && trimmed
      .chars()
      .skip(1)
      .all(|ch| ch.is_ascii_hexdigit());
  if valid {
    Ok(())
  } else {
    Err(AppError::Validation(
      "accentColor must be a hex color like #2f6dff".to_string(),
    ))
  }
}

fn validate_max_concurrent_downloads(value: u32) -> Result<(), AppError> {
  if (1..=8).contains(&value) {
    Ok(())
  } else {
    Err(AppError::Validation(
      "maxConcurrentDownloads must be between 1 and 8".to_string(),
    ))
  }
}

fn normalize_download_cookie(value: &str) -> String {
  value
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

fn clear_token_state(
  state: &AppState,
  updated_at_key: &str,
  checked_at_key: &str,
  status_key: &str,
  message_key: &str,
) -> Result<(), AppError> {
  state.db.set_meta(updated_at_key, "")?;
  state.db.set_meta(checked_at_key, "")?;
  state.db.set_meta(status_key, STATUS_NOT_CONFIGURED)?;
  state.db.set_meta(message_key, "")?;
  Ok(())
}

fn persist_token_validation(state: &AppState, result: &TokenValidationResult) -> Result<(), AppError> {
  let (checked_at_key, status_key, message_key) = match result.platform.trim() {
    "douyin" => (
      KEY_DOUYIN_LAST_CHECKED_AT,
      KEY_DOUYIN_LAST_CHECK_STATUS,
      KEY_DOUYIN_LAST_CHECK_MESSAGE,
    ),
    "tiktok" => (
      KEY_TIKTOK_LAST_CHECKED_AT,
      KEY_TIKTOK_LAST_CHECK_STATUS,
      KEY_TIKTOK_LAST_CHECK_MESSAGE,
    ),
    _ => {
      return Err(AppError::Validation(
        "platform must be one of douyin/tiktok".to_string(),
      ))
    }
  };

  state.db.set_meta(checked_at_key, &result.checked_at)?;
  state.db.set_meta(status_key, &result.status)?;
  state.db.set_meta(message_key, &result.message)?;
  Ok(())
}

fn validate_download_platform(value: &str) -> Result<(), AppError> {
  match value.trim() {
    "douyin" | "tiktok" => Ok(()),
    _ => Err(AppError::Validation(
      "platform must be one of douyin/tiktok".to_string(),
    )),
  }
}

fn read_environment_status(app: &AppHandle) -> Result<ManagedEnvironmentStatus, AppError> {
  let install_dir = managed_runtime_root(app)
    .ok_or_else(|| AppError::Io("failed to resolve app data directory".to_string()))?;
  let source_url = resolve_ffmpeg_archive_url()?;
  let python_bin = managed_runtime_bin_path(app);
  let ffmpeg_bin = managed_ffmpeg_bin_path(app);
  let python_ready = python_bin
    .as_ref()
    .map(|bin| validate_managed_runtime(bin).is_ok())
    .unwrap_or(false);
  let ffmpeg_ready = ffmpeg_bin
    .as_ref()
    .map(|bin| validate_managed_ffmpeg(bin).is_ok())
    .unwrap_or(false);

  let (installed, status, message) = match (python_ready, ffmpeg_ready) {
    (true, true) => (
      true,
      "ready".to_string(),
      "Python 与 FFmpeg 环境已就绪，可用于 Token 校验、下载和直播录制。".to_string(),
    ),
    _ if python_bin.is_none() && ffmpeg_bin.is_none() => (
      false,
      "missing".to_string(),
      "当前尚未下载内置 Python 与 FFmpeg 环境。".to_string(),
    ),
    _ => (
      true,
      "invalid".to_string(),
      "检测到环境目录，但 Python 或 FFmpeg 未完整安装，请重新下载环境。".to_string(),
    ),
  };

  Ok(ManagedEnvironmentStatus {
    python_version: MANAGED_PYTHON_VERSION.to_string(),
    ffmpeg_version: managed_ffmpeg_version().to_string(),
    source_label: format!("{} + {}", MANAGED_PYTHON_SOURCE_LABEL, MANAGED_FFMPEG_SOURCE_LABEL),
    source_url,
    install_dir: install_dir.to_string_lossy().to_string(),
    python_bin: python_bin.map(|path| path.to_string_lossy().to_string()),
    ffmpeg_bin: ffmpeg_bin.map(|path| path.to_string_lossy().to_string()),
    installed,
    status,
    message,
  })
}

fn install_managed_environment(app: &AppHandle) -> Result<ManagedEnvironmentStatus, AppError> {
  let install_dir = managed_runtime_root(app)
    .ok_or_else(|| AppError::Io("failed to resolve app data directory".to_string()))?;
  let base_dir = install_dir
    .parent()
    .ok_or_else(|| AppError::Io("failed to resolve managed runtime parent directory".to_string()))?;
  fs::create_dir_all(base_dir)?;

  let archive_url = resolve_python_archive_url()?;
  let archive_path = base_dir.join("python-runtime.tar.gz");
  let temp_dir = base_dir.join(format!("python-{}.tmp", MANAGED_PYTHON_VERSION));

  if temp_dir.exists() {
    fs::remove_dir_all(&temp_dir)?;
  }

  download_archive(&archive_url, &archive_path)?;
  extract_archive(&archive_path, &temp_dir)?;
  let python_bin = resolve_python_bin_in_root(&temp_dir)?;
  ensure_runtime_requirements(app, &temp_dir, &python_bin)?;

  if install_dir.exists() {
    fs::remove_dir_all(&install_dir)?;
  }
  fs::rename(&temp_dir, &install_dir)?;
  let _ = fs::remove_file(&archive_path);

  install_managed_ffmpeg(app)?;

  read_environment_status(app)
}

fn resolve_ffmpeg_archive_url() -> Result<String, AppError> {
  let (package_name, version) = match (std::env::consts::OS, std::env::consts::ARCH) {
    ("windows", "x86_64") => ("win32-x64", MANAGED_FFMPEG_VERSION),
    ("linux", "x86_64") => ("linux-x64", MANAGED_FFMPEG_VERSION),
    ("macos", "x86_64") => ("darwin-x64", MANAGED_FFMPEG_VERSION),
    ("macos", "aarch64") => ("darwin-arm64", MANAGED_FFMPEG_ARM64_MACOS_VERSION),
    _ => {
      return Err(AppError::Validation(format!(
        "managed ffmpeg is not available for {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
      )))
    }
  };

  Ok(format!(
    "{}/@ffmpeg-installer/{}/-/{}-{}.tgz",
    MANAGED_FFMPEG_SOURCE_BASE_URL, package_name, package_name, version
  ))
}

fn resolve_python_archive_url() -> Result<String, AppError> {
  let platform = match (std::env::consts::OS, std::env::consts::ARCH) {
    ("macos", "aarch64") => "aarch64-apple-darwin",
    ("macos", "x86_64") => "x86_64-apple-darwin",
    ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
    ("windows", "x86_64") => "x86_64-pc-windows-msvc",
    _ => {
      return Err(AppError::Validation(format!(
        "managed python environment is not available for {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
      )))
    }
  };

  Ok(format!(
    "{}/{}/cpython-{}+{}-{}-install_only.tar.gz",
    MANAGED_PYTHON_SOURCE_BASE_URL,
    MANAGED_PYTHON_RELEASE,
    MANAGED_PYTHON_VERSION,
    MANAGED_PYTHON_RELEASE,
    platform
  ))
}

fn download_archive(url: &str, destination: &Path) -> Result<(), AppError> {
  let client = Client::builder()
    .build()
    .map_err(|error| AppError::TaskExec(format!("failed to build download client: {}", error)))?;
  let mut response = client
    .get(url)
    .send()
    .and_then(|resp| resp.error_for_status())
    .map_err(|error| AppError::TaskExec(format!("failed to download environment package: {}", error)))?;

  let mut output = BufWriter::new(File::create(destination)?);
  response
    .copy_to(&mut output)
    .map_err(|error| AppError::TaskExec(format!("failed to write environment package: {}", error)))?;
  output.flush()?;
  Ok(())
}

fn extract_archive(archive_path: &Path, destination: &Path) -> Result<(), AppError> {
  fs::create_dir_all(destination)?;
  let file = File::open(archive_path)?;
  let decoder = GzDecoder::new(file);
  let mut archive = Archive::new(decoder);
  archive.unpack(destination)?;
  Ok(())
}

fn resolve_python_bin_in_root(root: &Path) -> Result<PathBuf, AppError> {
  let bin = if cfg!(target_os = "windows") {
    root.join("python").join("python.exe")
  } else {
    root.join("python").join("bin").join("python")
  };

  if bin.exists() {
    Ok(bin)
  } else {
    Err(AppError::PythonStart(
      "downloaded environment does not contain a usable python executable".to_string(),
    ))
  }
}

fn resolve_ffmpeg_bin_in_root(root: &Path) -> Result<PathBuf, AppError> {
  let bin = if cfg!(target_os = "windows") {
    root.join("package").join("ffmpeg.exe")
  } else {
    root.join("package").join("ffmpeg")
  };

  if bin.exists() {
    Ok(bin)
  } else {
    Err(AppError::TaskExec(
      "downloaded ffmpeg package does not contain a usable executable".to_string(),
    ))
  }
}

fn ensure_runtime_requirements(app: &AppHandle, runtime_root: &Path, python_bin: &Path) -> Result<(), AppError> {
  let requirements_path = resolve_runtime_requirements_file(app)?;
  let (ensurepip_stdout, ensurepip_stderr) =
    resolve_environment_command_stdio(app, "managed-runtime-install.log");

  let mut ensurepip_command = Command::new(python_bin);
  ensurepip_command
    .arg("-m")
    .arg("ensurepip")
    .arg("--upgrade")
    .stdout(ensurepip_stdout)
    .stderr(ensurepip_stderr)
    .current_dir(runtime_root);
  let ensurepip_status = configure_background_command(&mut ensurepip_command)
    .status()
    .map_err(|error| AppError::TaskExec(format!("failed to bootstrap pip: {}", error)))?;

  if !ensurepip_status.success() {
    return Err(AppError::TaskExec(
      "failed to bootstrap pip inside managed environment".to_string(),
    ));
  }

  let (install_stdout, install_stderr) = resolve_environment_command_stdio(app, "managed-runtime-install.log");
  let mut install_command = Command::new(python_bin);
  install_command
    .arg("-m")
    .arg("pip")
    .arg("install")
    .arg("--disable-pip-version-check")
    .arg("--no-input")
    .arg("--no-warn-script-location")
    .arg("-r")
    .arg(&requirements_path)
    .arg("-i")
    .arg(PYPI_MIRROR_URL)
    .stdout(install_stdout)
    .stderr(install_stderr)
    .current_dir(runtime_root);
  let install_status = configure_background_command(&mut install_command)
    .status()
    .map_err(|error| AppError::TaskExec(format!("failed to install runtime dependencies: {}", error)))?;

  if !install_status.success() {
    return Err(AppError::TaskExec(
      "failed to install runtime dependencies into managed environment".to_string(),
    ));
  }

  validate_managed_runtime(python_bin)
}

fn install_managed_ffmpeg(app: &AppHandle) -> Result<(), AppError> {
  let install_dir = managed_ffmpeg_root(app)
    .ok_or_else(|| AppError::Io("failed to resolve ffmpeg app data directory".to_string()))?;
  let base_dir = install_dir
    .parent()
    .ok_or_else(|| AppError::Io("failed to resolve ffmpeg install directory".to_string()))?;
  fs::create_dir_all(base_dir)?;

  let archive_url = resolve_ffmpeg_archive_url()?;
  let archive_path = base_dir.join("ffmpeg-runtime.tgz");
  let temp_dir = base_dir.join(format!("ffmpeg-{}.tmp", managed_ffmpeg_version()));

  if temp_dir.exists() {
    fs::remove_dir_all(&temp_dir)?;
  }

  download_archive(&archive_url, &archive_path)?;
  extract_archive(&archive_path, &temp_dir)?;
  let ffmpeg_bin = resolve_ffmpeg_bin_in_root(&temp_dir)?;
  validate_managed_ffmpeg(&ffmpeg_bin)?;

  if install_dir.exists() {
    fs::remove_dir_all(&install_dir)?;
  }
  fs::rename(&temp_dir, &install_dir)?;
  let _ = fs::remove_file(&archive_path);
  Ok(())
}

fn validate_managed_runtime(python_bin: &Path) -> Result<(), AppError> {
  let mut command = Command::new(python_bin);
  command
    .arg("-c")
    .arg("import f2, httpx, sys; print(sys.version)")
    .stdout(Stdio::null())
    .stderr(Stdio::null());
  let status = configure_background_command(&mut command)
    .status()
    .map_err(|error| AppError::TaskExec(format!("failed to verify managed environment: {}", error)))?;

  if status.success() {
    Ok(())
  } else {
    Err(AppError::TaskExec(
      "managed environment verification failed".to_string(),
    ))
  }
}

fn validate_managed_ffmpeg(ffmpeg_bin: &Path) -> Result<(), AppError> {
  let mut command = Command::new(ffmpeg_bin);
  command.arg("-version").stdout(Stdio::null()).stderr(Stdio::null());
  let status = configure_background_command(&mut command)
    .status()
    .map_err(|error| AppError::TaskExec(format!("failed to verify ffmpeg: {}", error)))?;

  if status.success() {
    Ok(())
  } else {
    Err(AppError::TaskExec("managed ffmpeg verification failed".to_string()))
  }
}

fn resolve_runtime_requirements_file(app: &AppHandle) -> Result<PathBuf, AppError> {
  if cfg!(debug_assertions) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_requirements = manifest_dir.join("python").join("requirements-runtime.txt");
    if dev_requirements.exists() {
      return Ok(dev_requirements);
    }
  }

  let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|error| AppError::TaskExec(format!("failed to resolve resource dir: {}", error)))?;
  let requirements = resource_dir.join("python").join("requirements-runtime.txt");
  if requirements.exists() {
    Ok(requirements)
  } else {
    Err(AppError::TaskExec(
      "runtime requirements file not found in bundled resources".to_string(),
    ))
  }
}

fn resolve_environment_command_stdio(app: &AppHandle, file_name: &str) -> (Stdio, Stdio) {
  let Ok(dir) = app.path().app_data_dir() else {
    return (Stdio::null(), Stdio::null());
  };

  if fs::create_dir_all(&dir).is_err() {
    return (Stdio::null(), Stdio::null());
  }

  let path = dir.join(file_name);
  let Ok(file) = OpenOptions::new().create(true).append(true).open(path) else {
    return (Stdio::null(), Stdio::null());
  };

  let Ok(stderr_file) = file.try_clone() else {
    return (Stdio::from(file), Stdio::null());
  };

  (Stdio::from(file), Stdio::from(stderr_file))
}
