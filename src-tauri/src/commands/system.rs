use tauri::State;

use crate::error::AppError;
use crate::domain::SystemInfo;
use crate::application::AppState;
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
