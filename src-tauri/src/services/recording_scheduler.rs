use std::collections::HashMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use chrono::Utc;
use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::application::AppState;
use crate::domain::{RecordingAccountItem, RecordingLiveStatusResult};
use crate::error::AppError;
use crate::repositories::Db;
use crate::services::python::managed_runtime_bin_path;
use crate::services::runtime_log::append_runtime_log;

const DEFAULT_MAX_CONCURRENT_RECORDINGS: usize = 3;
const KEY_DOUYIN_COOKIE: &str = "settings.douyin_cookie";
const KEY_MAX_CONCURRENT_DOWNLOADS: &str = "settings.max_concurrent_downloads";
const LOOP_INTERVAL_SECS: u64 = 12;
const STALE_TASK_REASON: &str = "recording task recovered from stale running state during app startup";
const STOP_TASK_REASON: &str = "recording task stopped because the application exited";

#[derive(Debug)]
struct ActiveRecordingJob {
  account_id: String,
  task_id: String,
  child: Child,
}

#[derive(Default)]
struct RecordingSchedulerState {
  started: bool,
  stop_requested: bool,
  active: HashMap<String, ActiveRecordingJob>,
}

pub struct RecordingScheduler {
  db_path: PathBuf,
  inner: Arc<Mutex<RecordingSchedulerState>>,
}

impl RecordingScheduler {
  pub fn new(db_path: PathBuf) -> Self {
    Self {
      db_path,
      inner: Arc::new(Mutex::new(RecordingSchedulerState::default())),
    }
  }

  pub fn start(&self, app: AppHandle) -> Result<(), AppError> {
    let mut state = self
      .inner
      .lock()
      .map_err(|_| AppError::TaskExec("recording scheduler mutex poisoned".to_string()))?;

    if state.started {
      return Ok(());
    }

    state.started = true;
    state.stop_requested = false;
    drop(state);

    let inner = Arc::clone(&self.inner);
    let db_path = self.db_path.clone();
    thread::spawn(move || run_scheduler_loop(app, db_path, inner));
    Ok(())
  }

  pub fn stop(&self) -> Result<(), AppError> {
    let db = Db::new(self.db_path.clone());
    let mut state = self
      .inner
      .lock()
      .map_err(|_| AppError::TaskExec("recording scheduler mutex poisoned".to_string()))?;
    state.stop_requested = true;

    let active_jobs = state.active.drain().map(|(_, job)| job).collect::<Vec<_>>();
    drop(state);

    for mut job in active_jobs {
      let _ = terminate_child(&mut job.child);
      let _ = db.update_task_failure(&job.task_id, STOP_TASK_REASON);
      let _ = db.append_log(&job.task_id, "warning", STOP_TASK_REASON);
      let _ = db.append_recording_account_log(&job.account_id, "warning", "应用退出，当前录制任务已终止。");
      let enabled = db.is_recording_account_enabled(&job.account_id).unwrap_or(false);
      let _ = db.mark_recording_account_finished(&job.account_id, &Utc::now().to_rfc3339(), enabled, Some(STOP_TASK_REASON));
    }

    Ok(())
  }

  pub fn stop_account(&self, account_id: &str, reason: &str) -> Result<(), AppError> {
    let mut state = self
      .inner
      .lock()
      .map_err(|_| AppError::TaskExec("recording scheduler mutex poisoned".to_string()))?;
    let Some(mut job) = state.active.remove(account_id) else {
      return Ok(());
    };
    drop(state);

    let db = Db::new(self.db_path.clone());
    let _ = terminate_child(&mut job.child);
    let _ = db.update_task_failure(&job.task_id, reason);
    let _ = db.append_log(&job.task_id, "warning", reason);
    let _ = db.append_recording_account_log(account_id, "warning", reason);
    let enabled = db.is_recording_account_enabled(account_id).unwrap_or(false);
    let _ = db.mark_recording_account_finished(account_id, &Utc::now().to_rfc3339(), enabled, Some(reason));
    Ok(())
  }
}

fn run_scheduler_loop(app: AppHandle, db_path: PathBuf, inner: Arc<Mutex<RecordingSchedulerState>>) {
  let db = Db::new(db_path);
  recover_stale_recording_runs(&app, &db);

  loop {
    if is_stop_requested(&inner) {
      break;
    }

    cleanup_finished_jobs(&app, &db, &inner);
    if let Err(error) = schedule_recordings(&app, &db, &inner) {
      append_runtime_log(&app, &format!("recording scheduler iteration failed error={}", error));
    }

    for _ in 0..LOOP_INTERVAL_SECS {
      if is_stop_requested(&inner) {
        break;
      }
      thread::sleep(Duration::from_secs(1));
    }
  }
}

fn recover_stale_recording_runs(app: &AppHandle, db: &Db) {
  let Ok(stale_runs) = db.mark_stale_recording_runs_failed(STALE_TASK_REASON) else {
    return;
  };

  for item in stale_runs {
    let enabled = db.is_recording_account_enabled(&item.account_id).unwrap_or(false);
    let _ = db.append_recording_account_log(&item.account_id, "warning", "应用重启后回收了上一次未结束的录制任务。");
    let _ = db.mark_recording_account_finished(&item.account_id, &Utc::now().to_rfc3339(), enabled, Some(STALE_TASK_REASON));
    append_runtime_log(app, &format!("recording stale task recovered task_id={} account_id={}", item.id, item.account_id));
  }
}

fn cleanup_finished_jobs(app: &AppHandle, db: &Db, inner: &Arc<Mutex<RecordingSchedulerState>>) {
  let finished_jobs = {
    let mut state = match inner.lock() {
      Ok(value) => value,
      Err(_) => return,
    };

    let mut finished_account_ids = Vec::new();
    for (account_id, job) in &mut state.active {
      match job.child.try_wait() {
        Ok(Some(status)) => {
          finished_account_ids.push((account_id.clone(), job.task_id.clone(), status.success()));
        }
        Ok(None) => {}
        Err(_) => {
          finished_account_ids.push((account_id.clone(), job.task_id.clone(), false));
        }
      }
    }

    let mut drained = Vec::new();
    for (account_id, task_id, success) in finished_account_ids {
      if let Some(job) = state.active.remove(&account_id) {
        drained.push((job.account_id, task_id, success));
      }
    }
    drained
  };

  for (account_id, task_id, success) in finished_jobs {
    let status = db.get_task_status(&task_id).ok().flatten().unwrap_or_default();
    if status == "running" {
      let message = if success {
        "recording worker exited before updating task status"
      } else {
        "recording worker exited unexpectedly"
      };
      let _ = db.update_task_failure(&task_id, message);
      let _ = db.append_log(&task_id, "error", message);
      let _ = db.append_recording_account_log(&account_id, "error", "录制进程异常退出，请查看任务明细。");
      let enabled = db.is_recording_account_enabled(&account_id).unwrap_or(false);
      let _ = db.mark_recording_account_finished(&account_id, &Utc::now().to_rfc3339(), enabled, Some(message));
      append_runtime_log(app, &format!("recording worker exited unexpectedly task_id={} account_id={}", task_id, account_id));
    }
  }
}

fn schedule_recordings(app: &AppHandle, db: &Db, inner: &Arc<Mutex<RecordingSchedulerState>>) -> Result<(), AppError> {
  let active_account_ids = current_active_account_ids(inner)?;
  let max_concurrency = read_max_concurrent_recordings(db).max(1);
  let accounts = db.list_enabled_recording_accounts()?;

  let mut active_count = active_account_ids.len();
  for account in accounts {
    if active_account_ids.contains(&account.id) {
      continue;
    }

    let live_result = match check_account_live_status(app, &account) {
      Ok(value) => value,
      Err(error) => {
        append_runtime_log(&app, &format!("recording live status check failed account_id={} error={}", account.id, error));
        continue;
      }
    };

    if live_result.status != "live" || !account.auto_start || account.platform != "douyin" {
      continue;
    }

    if active_count >= max_concurrency {
      let _ = db.append_recording_account_log(&account.id, "warning", "检测到账号已开播，但当前无可用录制并发槽位。");
      continue;
    }

    if start_recording_job(app, db, inner, &account, &live_result).is_ok() {
      active_count += 1;
    }
  }

  Ok(())
}

fn current_active_account_ids(inner: &Arc<Mutex<RecordingSchedulerState>>) -> Result<Vec<String>, AppError> {
  let state = inner
    .lock()
    .map_err(|_| AppError::TaskExec("recording scheduler mutex poisoned".to_string()))?;
  Ok(state.active.keys().cloned().collect())
}

fn is_stop_requested(inner: &Arc<Mutex<RecordingSchedulerState>>) -> bool {
  match inner.lock() {
    Ok(state) => state.stop_requested,
    Err(_) => true,
  }
}

fn start_recording_job(
  app: &AppHandle,
  db: &Db,
  inner: &Arc<Mutex<RecordingSchedulerState>>,
  account: &RecordingAccountItem,
  live_result: &RecordingLiveStatusResult,
) -> Result<(), AppError> {
  let room_id = live_result
    .account_room_id
    .as_deref()
    .or(account.account_room_id.as_deref())
    .unwrap_or_default()
    .trim()
    .to_string();

  if room_id.is_empty() {
    return Err(AppError::Validation("recording requires a valid room id".to_string()));
  }

  let payload = json!({
    "accountId": account.id,
    "platform": account.platform,
    "accountName": account.account_name,
    "accountUid": account.account_uid,
    "accountRoomId": room_id,
    "accountWebRid": live_result.account_web_rid.as_deref().or(account.account_web_rid.as_deref()),
    "accountSecUserId": account.account_sec_user_id,
    "retryOnDisconnect": account.retry_on_disconnect,
    "splitRecording": account.split_recording,
    "saveSnapshot": account.save_snapshot,
  });

  let task_id = db.insert_task("recording.live", &payload)?;
  let now = Utc::now().to_rfc3339();
  db.update_task_status(&task_id, "running")?;
  db.append_log(&task_id, "info", "live recording task running")?;
  db.append_recording_account_log(&account.id, "success", "检测到账号开播，已启动直播录制任务。")?;
  db.mark_recording_account_started(&account.id, &now)?;

  let child = match spawn_recording_worker(app, &task_id) {
    Ok(value) => value,
    Err(error) => {
      let message = error.to_string();
      let _ = db.update_task_failure(&task_id, &message);
      let _ = db.append_log(&task_id, "error", &message);
      let enabled = db.is_recording_account_enabled(&account.id).unwrap_or(false);
      let _ = db.mark_recording_account_finished(&account.id, &now, enabled, Some(&message));
      return Err(error);
    }
  };
  let mut state = inner
    .lock()
    .map_err(|_| AppError::TaskExec("recording scheduler mutex poisoned".to_string()))?;
  state.active.insert(
    account.id.clone(),
    ActiveRecordingJob {
      account_id: account.id.clone(),
      task_id,
      child,
    },
  );
  Ok(())
}

fn check_account_live_status(app: &AppHandle, account: &RecordingAccountItem) -> Result<RecordingLiveStatusResult, AppError> {
  let state = app.state::<AppState>();
  let cookie = resolve_recording_cookie(&state, &account.platform);
  let checked_at = Utc::now().to_rfc3339();
  if cookie.is_empty() {
    state.db.update_recording_account_check_result(
      &account.id,
      "error",
      account.account_room_id.as_deref(),
      account.account_web_rid.as_deref(),
      &checked_at,
      Some("cookie is required for live status check"),
    )?;
    if should_append_check_log(account, "error", Some("cookie is required for live status check")) {
      state
        .db
        .append_recording_account_log(&account.id, "error", "直播状态检测失败：未配置可用 Cookie。")?;
    }
    return Err(AppError::Validation("cookie is required for live status check".to_string()));
  }

  let value = state.python.request_isolated(
    app,
    "run_task",
    json!({
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
  )?;

  let live_status: RecordingLiveStatusResult = serde_json::from_value(value)?;
  let next_status = map_recording_account_status(&live_status.status);
  let should_append_log = should_append_check_log(account, next_status, live_status.error_message.as_deref());
  state.db.update_recording_account_check_result(
    &account.id,
    next_status,
    live_status.account_room_id.as_deref(),
    live_status.account_web_rid.as_deref(),
    &live_status.checked_at,
    live_status.error_message.as_deref(),
  )?;

  if should_append_log {
    state.db.append_recording_account_log(
      &account.id,
      match live_status.status.as_str() {
        "live" => "info",
        "recording" => "success",
        _ => "info",
      },
      &build_recording_check_message(&account.account_name, &live_status),
    )?;
  }

  Ok(live_status)
}

fn resolve_recording_cookie(state: &AppState, platform: &str) -> String {
  let key = match platform {
    "douyin" => KEY_DOUYIN_COOKIE,
    _ => return String::new(),
  };

  state
    .db
    .get_meta(key)
    .ok()
    .flatten()
    .map(|value| normalize_cookie(&value))
    .unwrap_or_default()
}

fn read_max_concurrent_recordings(db: &Db) -> usize {
  let configured = db
    .get_meta(KEY_MAX_CONCURRENT_DOWNLOADS)
    .ok()
    .flatten()
    .and_then(|raw| raw.trim().parse::<usize>().ok())
    .unwrap_or(DEFAULT_MAX_CONCURRENT_RECORDINGS);

  configured.clamp(1, 8)
}

fn spawn_recording_worker(app: &AppHandle, task_id: &str) -> Result<Child, AppError> {
  let (python_bin, script_path, cwd) = resolve_recording_worker_launch(app)?;
  let db_path = app
    .path()
    .app_data_dir()
    .map_err(|e| AppError::Io(e.to_string()))?
    .join("drtools.db");

  append_runtime_log(
    app,
    &format!(
      "launch recording worker task_id={} bin={} script={}",
      task_id, python_bin, script_path
    ),
  );

  let mut command = Command::new(python_bin);
  command
    .arg(script_path)
    .arg(&db_path)
    .arg(task_id)
    .env("PYTHONDONTWRITEBYTECODE", "1")
    .env("PYTHONUNBUFFERED", "1")
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(resolve_worker_stderr(app));

  if let Some(cwd) = cwd {
    command.current_dir(cwd);
  }

  command.spawn().map_err(|e| AppError::PythonStart(e.to_string()))
}

fn resolve_recording_worker_launch(app: &AppHandle) -> Result<(String, String, Option<PathBuf>), AppError> {
  if let Ok(bin) = env::var("DRTOOLS_PYTHON_BIN") {
    let script = env::var("DRTOOLS_RECORDING_WORKER_SCRIPT")
      .unwrap_or_else(|_| "python/recording_worker.py".to_string());
    return Ok((bin, script, None));
  }

  if let Some(runtime_bin) = managed_runtime_bin_path(app) {
    let script = resolve_managed_recording_worker_script(app)?;
    return Ok((
      runtime_bin.to_string_lossy().to_string(),
      script.to_string_lossy().to_string(),
      None,
    ));
  }

  if cfg!(debug_assertions) {
    if let Some((bin, script)) = detect_manifest_recording_worker_launch() {
      return Ok((bin, script, None));
    }
  }

  let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|e| AppError::PythonStart(e.to_string()))?;
  let script_path = resource_dir.join("python").join("recording_worker.py");

  let runtime_bin = if cfg!(target_os = "windows") {
    resource_dir.join("python-runtime").join("python.exe")
  } else {
    resource_dir.join("python-runtime").join("python")
  };

  if runtime_bin.exists() && script_path.exists() {
    return Ok((
      runtime_bin.to_string_lossy().to_string(),
      script_path.to_string_lossy().to_string(),
      Some(resource_dir),
    ));
  }

  Err(AppError::PythonStart(
    "embedded python runtime not found for recording worker".to_string(),
  ))
}

fn detect_manifest_recording_worker_launch() -> Option<(String, String)> {
  let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let python = manifest_dir.parent()?.join(".venv").join("bin").join("python");
  let script = manifest_dir.join("python").join("recording_worker.py");
  if python.exists() && script.exists() {
    return Some((
      python.to_string_lossy().to_string(),
      script.to_string_lossy().to_string(),
    ));
  }
  None
}

fn resolve_managed_recording_worker_script(app: &AppHandle) -> Result<PathBuf, AppError> {
  if cfg!(debug_assertions) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_script = manifest_dir.join("python").join("recording_worker.py");
    if dev_script.exists() {
      return Ok(dev_script);
    }
  }

  let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|e| AppError::PythonStart(e.to_string()))?;
  let script = resource_dir.join("python").join("recording_worker.py");
  if script.exists() {
    Ok(script)
  } else {
    Err(AppError::PythonStart(
      "recording worker script not found in bundled resources".to_string(),
    ))
  }
}

fn resolve_worker_stderr(app: &AppHandle) -> Stdio {
  let Ok(dir) = app.path().app_data_dir() else {
    return Stdio::null();
  };

  if fs::create_dir_all(&dir).is_err() {
    return Stdio::null();
  }

  let path = dir.join("recording-worker.stderr.log");
  match OpenOptions::new().create(true).append(true).open(path) {
    Ok(file) => Stdio::from(file),
    Err(_) => Stdio::null(),
  }
}

fn normalize_cookie(value: &str) -> String {
  value
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
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

fn terminate_child(child: &mut Child) -> Result<(), AppError> {
  #[cfg(unix)]
  {
    let status = Command::new("kill")
      .arg("-TERM")
      .arg(child.id().to_string())
      .status()
      .map_err(|e| AppError::TaskExec(e.to_string()))?;
    if !status.success() {
      child.kill()?;
    }
  }

  #[cfg(not(unix))]
  {
    child.kill()?;
  }

  let _ = child.wait();
  Ok(())
}
