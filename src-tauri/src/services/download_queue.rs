use std::collections::VecDeque;
use std::panic::{self, AssertUnwindSafe};
use std::process::Command;
use std::sync::Mutex;
use std::thread;

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::application::AppState;
use crate::error::AppError;
use crate::services::configure_background_command;
use crate::services::runtime_log::append_runtime_log;

const DEFAULT_MAX_CONCURRENT_DOWNLOADS: usize = 3;
const KEY_DOUYIN_COOKIE: &str = "settings.douyin_cookie";
const KEY_DOWNLOAD_NOTIFICATIONS_ENABLED: &str = "settings.download_notifications_enabled";
const KEY_EXPORT_DIR: &str = "settings.export_dir";
const KEY_MAX_CONCURRENT_DOWNLOADS: &str = "settings.max_concurrent_downloads";
const KEY_TIKTOK_COOKIE: &str = "settings.tiktok_cookie";

#[derive(Debug, Clone)]
pub struct QueuedDownloadTask {
  pub task_id: String,
  pub batch_id: String,
  pub platform: String,
  pub source_url: String,
  pub download_cover: bool,
  pub download_music: bool,
  pub download_description: bool,
  pub download_lyric: bool,
}

struct DownloadQueueState {
  pending: VecDeque<QueuedDownloadTask>,
  running: usize,
}

pub struct DownloadQueueManager {
  inner: Mutex<DownloadQueueState>,
}

impl DownloadQueueManager {
  pub fn new() -> Self {
    Self {
      inner: Mutex::new(DownloadQueueState {
        pending: VecDeque::new(),
        running: 0,
      }),
    }
  }

  pub fn enqueue_batch(&self, app: &AppHandle, jobs: Vec<QueuedDownloadTask>) -> Result<(), AppError> {
    if jobs.is_empty() {
      return Ok(());
    }

    let mut state = self
      .inner
      .lock()
      .map_err(|_| AppError::TaskExec("download queue mutex poisoned".to_string()))?;

    for job in jobs {
      state.pending.push_back(job);
    }

    drop(state);
    self.schedule(app)
  }

  pub fn finish_and_reschedule(&self, app: &AppHandle) {
    if let Ok(mut state) = self.inner.lock() {
      if state.running > 0 {
        state.running -= 1;
      }
    }

    let _ = self.schedule(app);
  }

  fn schedule(&self, app: &AppHandle) -> Result<(), AppError> {
    loop {
      let next_job = {
        let app_state = app.state::<AppState>();
        let max_concurrency = read_max_concurrent_downloads(&app_state).max(1);
        let mut state = self
          .inner
          .lock()
          .map_err(|_| AppError::TaskExec("download queue mutex poisoned".to_string()))?;

        if state.running >= max_concurrency {
          None
        } else if let Some(job) = state.pending.pop_front() {
          state.running += 1;
          Some(job)
        } else {
          None
        }
      };

      let Some(job) = next_job else {
        break;
      };

      let app_handle = app.clone();
      thread::spawn(move || {
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
          run_download_task(&app_handle, &job);
        }));

        if result.is_err() {
          handle_download_worker_panic(&app_handle, &job);
        }

        let app_state = app_handle.state::<AppState>();
        app_state.download_queue.finish_and_reschedule(&app_handle);
      });
    }

    Ok(())
  }
}

fn read_max_concurrent_downloads(state: &AppState) -> usize {
  let configured = state
    .db
    .get_meta(KEY_MAX_CONCURRENT_DOWNLOADS)
    .ok()
    .flatten()
    .and_then(|raw| raw.trim().parse::<usize>().ok())
    .unwrap_or(DEFAULT_MAX_CONCURRENT_DOWNLOADS);

  if cfg!(all(debug_assertions, target_os = "macos")) {
    configured.min(1)
  } else {
    configured
  }
}

fn run_download_task(app: &AppHandle, job: &QueuedDownloadTask) {
  append_runtime_log(
    app,
    &format!(
      "download worker start task_id={} batch_id={} platform={} url={}",
      job.task_id, job.batch_id, job.platform, job.source_url
    ),
  );
  let state = app.state::<AppState>();
  let output_dir = resolve_download_output_dir(app, &state);
  let cookie = resolve_download_cookie(&state, &job.platform);
  let notifications_enabled = resolve_download_notifications_enabled(&state);
  let payload = json!({
    "taskId": job.task_id,
    "platform": job.platform,
    "sourceUrl": job.source_url,
    "downloadCover": job.download_cover,
    "downloadMusic": job.download_music,
    "downloadDescription": job.download_description,
    "downloadLyric": job.download_lyric,
    "outputDir": output_dir,
    "cookie": cookie,
  });

  let _ = state.db.update_task_status(&job.task_id, "running");
  let _ = state.db.append_log(&job.task_id, "info", "video download task running");

  let request_payload = json!({
    "task_type": "video.download",
    "payload": payload,
  });

  append_runtime_log(app, &format!("download worker mode=isolated-python task_id={}", job.task_id));
  let run_result = state.python.request_isolated(app, "run_task", request_payload);

  match run_result {
    Ok(output) => {
      append_runtime_log(app, &format!("download worker success task_id={}", job.task_id));
      let _ = state.db.update_task_success(&job.task_id, &output);
      let _ = state.db.append_log(&job.task_id, "info", "video download task success");
      let completion = state.db.finish_download_batch_task(&job.batch_id, true).ok().flatten();
      if notifications_enabled {
        if let Some(summary) = completion.as_ref() {
          notify_download_batch_completed(summary);
        }
      }
    }
    Err(error) => {
      let message = error.to_string();
      append_runtime_log(
        app,
        &format!("download worker failure task_id={} error={}", job.task_id, message),
      );
      let _ = state.db.update_task_failure(&job.task_id, &message);
      let _ = state
        .db
        .append_log(&job.task_id, "error", &format!("video download task failed: {}", message));
      let completion = state.db.finish_download_batch_task(&job.batch_id, false).ok().flatten();
      if notifications_enabled {
        if let Some(summary) = completion.as_ref() {
          notify_download_batch_completed(summary);
        }
      }
    }
  }
}

fn handle_download_worker_panic(app: &AppHandle, job: &QueuedDownloadTask) {
  let state = app.state::<AppState>();
  let message = "video download worker panicked unexpectedly";
  append_runtime_log(app, &format!("download worker panic task_id={}", job.task_id));
  let _ = state.db.update_task_failure(&job.task_id, message);
  let _ = state.db.append_log(&job.task_id, "error", message);
  let completion = state.db.finish_download_batch_task(&job.batch_id, false).ok().flatten();

  if resolve_download_notifications_enabled(&state) {
    if let Some(summary) = completion.as_ref() {
      notify_download_batch_completed(summary);
    }
  }

  eprintln!("download worker panic recovered for task {}", job.task_id);
}

fn resolve_download_output_dir(app: &AppHandle, state: &AppState) -> String {
  if let Ok(Some(value)) = state.db.get_meta(KEY_EXPORT_DIR) {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
      return trimmed.to_string();
    }
  }

  app
    .path()
    .download_dir()
    .ok()
    .or_else(|| app.path().home_dir().ok())
    .map(|path| path.to_string_lossy().to_string())
    .unwrap_or_else(|| ".".to_string())
}

fn resolve_download_cookie(state: &AppState, platform: &str) -> Option<String> {
  let key = match platform {
    "douyin" => KEY_DOUYIN_COOKIE,
    "tiktok" => KEY_TIKTOK_COOKIE,
    _ => return None,
  };

  state
    .db
    .get_meta(key)
    .ok()
    .flatten()
    .and_then(|value| {
      let trimmed = value.trim();
      if trimmed.is_empty() {
        None
      } else {
        Some(trimmed.to_string())
      }
    })
}

fn resolve_download_notifications_enabled(state: &AppState) -> bool {
  state
    .db
    .get_meta(KEY_DOWNLOAD_NOTIFICATIONS_ENABLED)
    .ok()
    .flatten()
    .and_then(|value| match value.trim().to_ascii_lowercase().as_str() {
      "true" | "1" | "yes" | "on" => Some(true),
      "false" | "0" | "no" | "off" => Some(false),
      _ => None,
    })
    .unwrap_or(true)
}

fn notify_download_batch_completed(summary: &crate::domain::DownloadBatchCompletion) {
  let _ = &summary.batch_id;
  let (title, body) = if summary.success_count == 0 {
    (
      "批次下载失败",
      format!(
        "{} · 全部失败，共 {} 条",
        platform_label(&summary.platform),
        summary.failed_count
      ),
    )
  } else if summary.failed_count == 0 {
    (
      "批次下载完成",
      format!(
        "{} · 全部成功，共 {} 条",
        platform_label(&summary.platform),
        summary.success_count
      ),
    )
  } else {
    (
      "批次下载完成",
      format!(
        "{} · 成功 {} 条，失败 {} 条，共 {} 条",
        platform_label(&summary.platform),
        summary.success_count,
        summary.failed_count,
        summary.total_count
      ),
    )
  };

  send_system_notification(title, &body);
}

fn platform_label(platform: &str) -> &str {
  match platform {
    "douyin" => "抖音",
    "tiktok" => "TikTok",
    _ => platform,
  }
}

fn send_system_notification(title: &str, body: &str) {
  #[cfg(target_os = "macos")]
  {
    let script = format!(
      "display notification {} with title {}",
      apple_script_string(body),
      apple_script_string(title),
    );
    let mut command = Command::new("osascript");
    command.arg("-e").arg(script);
    let _ = configure_background_command(&mut command).spawn();
  }

  #[cfg(target_os = "windows")]
  {
    let script = format!(
      r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] > $null
$template = @"
<toast>
  <visual>
    <binding template="ToastGeneric">
      <text>{}</text>
      <text>{}</text>
    </binding>
  </visual>
</toast>
"@
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
$notifier = [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("Dr.Tools")
$notifier.Show($toast)
"#,
      xml_escape(title),
      xml_escape(body),
    );
    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    let _ = configure_background_command(&mut command).status();
  }

  #[cfg(target_os = "linux")]
  {
    let mut command = Command::new("notify-send");
    command.args([title, body]);
    let _ = configure_background_command(&mut command).spawn();
  }
}

#[cfg(target_os = "macos")]
fn apple_script_string(value: &str) -> String {
  let escaped = value
    .replace('\\', "\\\\")
    .replace('"', "\\\"")
    .replace('\n', " ")
    .replace('\r', " ");
  format!("\"{}\"", escaped)
}

#[cfg(target_os = "windows")]
fn xml_escape(value: &str) -> String {
  value
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
}
