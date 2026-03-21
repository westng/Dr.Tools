use tauri::{AppHandle, State};

use crate::commands::windowing::{normalize_window_title, open_or_focus_window};
use crate::application::AppState;
use crate::domain::{
  RecordingAccountCreatePayload, RecordingAccountItem, RecordingAccountLogEntry, RecordingAccountResolvePayload,
  RecordingAccountResolveResult, RecordingAccountUpdatePayload, RecordingAccountsSnapshot, RecordingLiveStatusResult,
};
use crate::error::AppError;
use crate::services::runtime_log::append_runtime_log;

const KEY_DOUYIN_COOKIE: &str = "settings.douyin_cookie";
const KEY_TIKTOK_COOKIE: &str = "settings.tiktok_cookie";
const RECORDING_ACCOUNT_CREATE_WINDOW_LABEL: &str = "record-account-create";
const RECORDING_ACCOUNT_EDIT_WINDOW_LABEL_PREFIX: &str = "record-account-edit";
const RECORDING_ACCOUNT_LOGS_WINDOW_LABEL_PREFIX: &str = "record-account-logs";

#[tauri::command]
pub async fn open_recording_account_create_window(
  title: String,
  app: tauri::AppHandle,
) -> Result<(), AppError> {
  let normalized_title = normalize_window_title(&title, "Add Recording Account");
  let route = "#/record/account/create";
  let url = "index.html#/record/account/create";

  append_runtime_log(&app, "open recording account create window");
  open_or_focus_window(
    &app,
    RECORDING_ACCOUNT_CREATE_WINDOW_LABEL,
    &normalized_title,
    route,
    url,
    680.0,
    560.0,
    560.0,
    420.0,
  )
}

#[tauri::command]
pub async fn open_recording_account_edit_window(
  account_id: String,
  title: String,
  app: tauri::AppHandle,
) -> Result<(), AppError> {
  let trimmed = account_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("account_id is required".to_string()));
  }

  let normalized_title = normalize_window_title(&title, "Edit Recording Account");
  let label = format!("{}-{}", RECORDING_ACCOUNT_EDIT_WINDOW_LABEL_PREFIX, trimmed);
  let route = format!("#/record/account/{}/edit", trimmed);
  let url = format!("index.html#/record/account/{}/edit", trimmed);

  append_runtime_log(&app, &format!("open recording account edit window account_id={}", trimmed));
  open_or_focus_window(
    &app,
    &label,
    &normalized_title,
    &route,
    &url,
    680.0,
    560.0,
    560.0,
    420.0,
  )
}

#[tauri::command]
pub async fn open_recording_account_logs_window(
  account_id: String,
  title: String,
  app: tauri::AppHandle,
) -> Result<(), AppError> {
  let trimmed = account_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("account_id is required".to_string()));
  }

  let normalized_title = normalize_window_title(&title, "Recording Account Logs");
  let label = format!("{}-{}", RECORDING_ACCOUNT_LOGS_WINDOW_LABEL_PREFIX, trimmed);
  let route = format!("#/record/account/{}/logs", trimmed);
  let url = format!("index.html#/record/account/{}/logs", trimmed);

  append_runtime_log(&app, &format!("open recording account logs window account_id={}", trimmed));
  open_or_focus_window(
    &app,
    &label,
    &normalized_title,
    &route,
    &url,
    860.0,
    700.0,
    720.0,
    520.0,
  )
}

#[tauri::command]
pub fn resolve_recording_account_profile(
  payload: RecordingAccountResolvePayload,
  state: State<'_, AppState>,
  app: AppHandle,
) -> Result<RecordingAccountResolveResult, AppError> {
  let platform = payload.platform.trim();
  let source_url = payload.source_url.trim();

  if !matches!(platform, "douyin" | "tiktok") {
    return Err(AppError::Validation(
      "platform must be one of douyin/tiktok".to_string(),
    ));
  }

  if source_url.is_empty() {
    return Err(AppError::Validation("sourceUrl is required".to_string()));
  }

  if !source_url.starts_with("http://") && !source_url.starts_with("https://") {
    return Err(AppError::Validation(
      "sourceUrl must be an http or https url".to_string(),
    ));
  }

  let cookie_key = if platform == "douyin" {
    KEY_DOUYIN_COOKIE
  } else {
    KEY_TIKTOK_COOKIE
  };
  let cookie = state
    .db
    .get_meta(cookie_key)?
    .map(|value| normalize_cookie(&value))
    .unwrap_or_default();

  if cookie.is_empty() {
    return Err(AppError::Validation(
      "cookie is required for profile resolution".to_string(),
    ));
  }

  append_runtime_log(
    &app,
    &format!("resolve recording account profile platform={} source_url={}", platform, source_url),
  );

  let result = state.python.request_isolated(
    &app,
    "run_task",
    serde_json::json!({
      "task_type": "recording.account.resolve",
      "payload": {
        "platform": platform,
        "sourceUrl": source_url,
        "cookie": cookie,
      },
    }),
  )?;

  serde_json::from_value(result).map_err(AppError::from)
}

#[tauri::command]
pub fn recording_accounts_snapshot(
  log_limit: Option<u32>,
  run_limit: Option<u32>,
  state: State<'_, AppState>,
) -> Result<RecordingAccountsSnapshot, AppError> {
  let safe_limit = log_limit.unwrap_or(50).clamp(1, 200);
  let safe_run_limit = run_limit.unwrap_or(12).clamp(1, 100);
  Ok(RecordingAccountsSnapshot {
    accounts: state.db.list_recording_accounts()?,
    logs: state.db.list_recording_account_logs(safe_limit)?,
    runs: state.db.list_recording_runs(safe_run_limit)?,
  })
}

#[tauri::command]
pub fn recording_account_create(
  payload: RecordingAccountCreatePayload,
  state: State<'_, AppState>,
) -> Result<RecordingAccountItem, AppError> {
  validate_recording_platform(&payload.platform)?;
  if payload.account_input.trim().is_empty() {
    return Err(AppError::Validation("accountInput is required".to_string()));
  }
  if payload.account_name.trim().is_empty() {
    return Err(AppError::Validation("accountName is required".to_string()));
  }
  if payload.account_uid.trim().is_empty() {
    return Err(AppError::Validation("accountUid is required".to_string()));
  }

  let account = state.db.insert_recording_account(&payload)?;
  state
    .db
    .append_recording_account_log(&account.id, "success", "录制账号已添加到列表。")?;
  Ok(account)
}

#[tauri::command]
pub fn recording_account_detail(account_id: String, state: State<'_, AppState>) -> Result<RecordingAccountItem, AppError> {
  let trimmed = account_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("account_id is required".to_string()));
  }

  state
    .db
    .get_recording_account(trimmed)?
    .ok_or_else(|| AppError::Validation("recording account not found".to_string()))
}

#[tauri::command]
pub fn recording_account_update(
  payload: RecordingAccountUpdatePayload,
  state: State<'_, AppState>,
) -> Result<RecordingAccountItem, AppError> {
  validate_recording_platform(&payload.platform)?;
  if payload.account_id.trim().is_empty() {
    return Err(AppError::Validation("accountId is required".to_string()));
  }
  if payload.account_input.trim().is_empty() {
    return Err(AppError::Validation("accountInput is required".to_string()));
  }
  if payload.account_name.trim().is_empty() {
    return Err(AppError::Validation("accountName is required".to_string()));
  }
  if payload.account_uid.trim().is_empty() {
    return Err(AppError::Validation("accountUid is required".to_string()));
  }

  let account = state.db.update_recording_account(&payload)?;
  state
    .db
    .append_recording_account_log(&account.id, "info", "录制账号配置已更新。")?;
  Ok(account)
}

#[tauri::command]
pub fn recording_account_set_enabled(
  account_id: String,
  enabled: bool,
  state: State<'_, AppState>,
) -> Result<RecordingAccountItem, AppError> {
  let trimmed = account_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("account_id is required".to_string()));
  }

  state.db.set_recording_account_enabled(trimmed, enabled)?;
  if !enabled {
    state
      .recording_scheduler
      .stop_account(trimmed, "录制账号已停用，当前录制任务已终止。")?;
  }
  state.db.append_recording_account_log(
    trimmed,
    if enabled { "info" } else { "warning" },
    if enabled { "录制账号已启用。" } else { "录制账号已停用。" },
  )?;

  state
    .db
    .get_recording_account(trimmed)?
    .ok_or_else(|| AppError::Validation("recording account not found".to_string()))
}

#[tauri::command]
pub fn recording_account_delete(account_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
  let trimmed = account_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("account_id is required".to_string()));
  }

  state.db.delete_recording_account(trimmed)?;
  Ok(())
}

#[tauri::command]
pub fn recording_account_logs(
  account_id: String,
  limit: Option<u32>,
  state: State<'_, AppState>,
) -> Result<Vec<RecordingAccountLogEntry>, AppError> {
  let trimmed = account_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("account_id is required".to_string()));
  }

  let safe_limit = limit.unwrap_or(100).clamp(1, 500);
  state.db.list_recording_logs_for_account(trimmed, safe_limit)
}

#[tauri::command]
pub fn recording_accounts_check(
  account_ids: Option<Vec<String>>,
  state: State<'_, AppState>,
  app: AppHandle,
) -> Result<RecordingAccountsSnapshot, AppError> {
  let accounts = resolve_recording_accounts_to_check(&state, account_ids)?;
  for account in accounts {
    if !account.enabled {
      continue;
    }

    let cookie = resolve_recording_cookie(&state, &account.platform);
    let checked_at = chrono::Utc::now().to_rfc3339();
    if cookie.is_empty() {
      state.db.update_recording_account_check_result(
        &account.id,
        "error",
        account.account_room_id.as_deref(),
        account.account_web_rid.as_deref(),
        &checked_at,
        Some("cookie is required for live status check"),
      )?;
      state
        .db
        .append_recording_account_log(&account.id, "error", "直播状态检测失败：未配置可用 Cookie。")?;
      continue;
    }

    let result = state.python.request_isolated(
      &app,
      "run_task",
      serde_json::json!({
        "task_type": "recording.live_status.check",
        "payload": {
          "platform": account.platform,
          "sourceUrl": account.account_input,
          "accountRoomId": account.account_room_id,
          "accountWebRid": account.account_web_rid,
          "accountSecUserId": account.account_sec_user_id,
          "accountUniqueId": account.account_unique_id,
          "cookie": cookie,
        },
      }),
    );

    match result {
      Ok(value) => {
        let status: RecordingLiveStatusResult = serde_json::from_value(value).map_err(AppError::from)?;
        let next_status = map_recording_account_status(&status.status);
        let should_append_log = should_append_check_log(&account, next_status, status.error_message.as_deref());
        state.db.update_recording_account_check_result(
          &account.id,
          next_status,
          status.account_room_id.as_deref(),
          status.account_web_rid.as_deref(),
          &status.checked_at,
          status.error_message.as_deref(),
        )?;
        if should_append_log {
          state.db.append_recording_account_log(
            &account.id,
            match status.status.as_str() {
              "live" => "info",
              "recording" => "success",
              _ => "info",
            },
            &build_recording_check_message(&account.account_name, &status),
          )?;
        }
      }
      Err(error) => {
        let message = error.to_string();
        let should_append_log = account.status != "error" || account.last_error.as_deref() != Some(message.as_str());
        state.db.update_recording_account_check_result(
          &account.id,
          "error",
          account.account_room_id.as_deref(),
          account.account_web_rid.as_deref(),
          &checked_at,
          Some(&message),
        )?;
        if should_append_log {
          state.db.append_recording_account_log(
            &account.id,
            "error",
            &format!("{} 直播状态检测失败：{}", account.account_name, message),
          )?;
        }
      }
    }
  }

  recording_accounts_snapshot(Some(50), Some(12), state)
}

fn normalize_cookie(value: &str) -> String {
  value
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

fn validate_recording_platform(platform: &str) -> Result<(), AppError> {
  if matches!(platform.trim(), "douyin" | "tiktok") {
    Ok(())
  } else {
    Err(AppError::Validation(
      "platform must be one of douyin/tiktok".to_string(),
    ))
  }
}

fn resolve_recording_cookie(state: &AppState, platform: &str) -> String {
  let key = if platform == "douyin" {
    KEY_DOUYIN_COOKIE
  } else {
    KEY_TIKTOK_COOKIE
  };

  state
    .db
    .get_meta(key)
    .ok()
    .flatten()
    .map(|value| normalize_cookie(&value))
    .unwrap_or_default()
}

fn resolve_recording_accounts_to_check(
  state: &AppState,
  account_ids: Option<Vec<String>>,
) -> Result<Vec<RecordingAccountItem>, AppError> {
  let Some(ids) = account_ids else {
    return state.db.list_enabled_recording_accounts();
  };

  if ids.is_empty() {
    return state.db.list_enabled_recording_accounts();
  }

  let mut accounts = Vec::new();
  let mut seen = std::collections::HashSet::new();
  for id in ids {
    let trimmed = id.trim();
    if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
      continue;
    }
    if let Some(account) = state.db.get_recording_account(trimmed)? {
      accounts.push(account);
    }
  }
  Ok(accounts)
}

fn map_recording_account_status(status: &str) -> &str {
  match status {
    "live" => "live",
    "recording" => "recording",
    "error" => "error",
    _ => "watching",
  }
}

fn build_recording_check_message(account_name: &str, result: &RecordingLiveStatusResult) -> String {
  match result.status.as_str() {
    "live" => format!("{} 当前正在直播。", account_name),
    "recording" => format!("{} 当前正在录制中。", account_name),
    "error" => format!(
      "{} 直播状态异常：{}",
      account_name,
      result.error_message.as_deref().unwrap_or("未知错误")
    ),
    _ => format!("{} 当前未开播。", account_name),
  }
}

fn should_append_check_log(account: &RecordingAccountItem, next_status: &str, error_message: Option<&str>) -> bool {
  if next_status == "error" {
    return account.status != "error" || account.last_error.as_deref() != error_message;
  }

  account.status != next_status || account.last_error.is_some()
}
