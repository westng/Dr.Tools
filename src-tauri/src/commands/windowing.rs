use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppError;

pub(crate) fn normalize_window_title(value: &str, fallback: &str) -> String {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    return fallback.to_string();
  }

  trimmed.to_string()
}

pub(crate) fn open_or_focus_window(
  app: &tauri::AppHandle,
  label: &str,
  title: &str,
  route: &str,
  url: &str,
  width: f64,
  height: f64,
  min_width: f64,
  min_height: f64,
) -> Result<(), AppError> {
  if let Some(window) = app.get_webview_window(label) {
    window.set_title(title).map_err(|e| AppError::TaskExec(e.to_string()))?;
    let script = format!(
      "if (window.location.hash !== {route}) {{ window.location.hash = {route}; }}",
      route = serde_json::to_string(&route).map_err(AppError::from)?,
    );
    window.eval(script).map_err(|e| AppError::TaskExec(e.to_string()))?;
    window.show().map_err(|e| AppError::TaskExec(e.to_string()))?;
    window.set_focus().map_err(|e| AppError::TaskExec(e.to_string()))?;
    return Ok(());
  }

  WebviewWindowBuilder::new(app, label, WebviewUrl::App(url.into()))
    .title(title)
    .inner_size(width, height)
    .min_inner_size(min_width, min_height)
    .center()
    .focused(true)
    .build()
    .map_err(|e| AppError::TaskExec(e.to_string()))?;

  Ok(())
}
