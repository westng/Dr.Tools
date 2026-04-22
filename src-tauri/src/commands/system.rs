use std::process::Command;

use tauri::State;

use crate::application::AppState;
use crate::domain::SystemInfo;
use crate::error::AppError;
use crate::services::configure_background_command;
use crate::services::runtime_log::append_runtime_log;

#[tauri::command]
pub fn system_info() -> Result<SystemInfo, AppError> {
  Ok(SystemInfo {
    app_name: "Dr.Tools".to_string(),
    app_version: env!("CARGO_PKG_VERSION").to_string(),
    os: std::env::consts::OS.to_string(),
    arch: std::env::consts::ARCH.to_string(),
  })
}

#[tauri::command]
pub fn python_ping(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<String, AppError> {
  let result = state.python.request(&app, "ping", serde_json::json!({}))?;
  let message = result
    .get("message")
    .and_then(|v| v.as_str())
    .unwrap_or("pong")
    .to_string();
  Ok(message)
}

#[tauri::command]
pub fn frontend_log_error(scope: String, message: String, app: tauri::AppHandle) -> Result<(), AppError> {
  let normalized_scope = scope.trim();
  let normalized_message = message.replace(['\n', '\r'], " ");
  let normalized_message = normalized_message.trim();

  if normalized_scope.is_empty() || normalized_message.is_empty() {
    return Ok(());
  }

  append_runtime_log(
    &app,
    &format!("frontend error scope={} message={}", normalized_scope, normalized_message),
  );
  Ok(())
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), AppError> {
  let normalized_url = url.trim();
  if !(normalized_url.starts_with("https://") || normalized_url.starts_with("http://")) {
    return Err(AppError::Validation("url must start with http:// or https://".to_string()));
  }

  #[cfg(target_os = "macos")]
  {
    let mut open_command = Command::new("open");
    open_command.arg(normalized_url);
    let open_status = configure_background_command(&mut open_command).status()?;
    if !open_status.success() {
      let escaped_url = normalized_url.replace('\\', "\\\\").replace('"', "\\\"");
      let mut fallback_command = Command::new("osascript");
      fallback_command.args(["-e", &format!("open location \"{}\"", escaped_url)]);
      let fallback_status = configure_background_command(&mut fallback_command).status()?;

      if !fallback_status.success() {
        return Err(AppError::Io(format!(
          "failed to open url with macOS handlers status_open={:?} status_osascript={:?}",
          open_status.code(),
          fallback_status.code()
        )));
      }
    }
  }

  #[cfg(target_os = "windows")]
  {
    let mut command = Command::new("cmd");
    command.args(["/C", "start", "", normalized_url]);
    let status = configure_background_command(&mut command).status()?;
    if !status.success() {
      return Err(AppError::Io(format!(
        "failed to open url with Windows shell status={:?}",
        status.code()
      )));
    }
  }

  #[cfg(all(unix, not(target_os = "macos")))]
  {
    let mut command = Command::new("xdg-open");
    command.arg(normalized_url);
    let status = configure_background_command(&mut command).status()?;
    if !status.success() {
      return Err(AppError::Io(format!(
        "failed to open url with xdg-open status={:?}",
        status.code()
      )));
    }
  }

  Ok(())
}
