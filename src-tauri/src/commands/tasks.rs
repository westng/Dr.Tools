use tauri::State;

use crate::commands::windowing::{normalize_window_title, open_or_focus_window};
use crate::error::AppError;
use crate::domain::{
  DownloadBatchDetail, DownloadBatchListResult, TaskDetail, TaskRecordDetail, TaskRunRequest, TaskRunResponse,
  TaskSummary, VideoDownloadSubmitPayload, VideoDownloadSubmitResult,
};
use crate::application::AppState;
use crate::services::runtime_log::append_runtime_log;

const BATCH_DETAIL_WINDOW_LABEL: &str = "task-batch-detail";
const TASK_DETAIL_WINDOW_LABEL: &str = "task-detail";

#[tauri::command]
pub fn task_run(
  payload: TaskRunRequest,
  state: State<'_, AppState>,
  app: tauri::AppHandle,
) -> Result<TaskRunResponse, AppError> {
  if payload.task_type.trim().is_empty() {
    return Err(AppError::Validation("task_type is required".to_string()));
  }

  let task_id = state.db.insert_task(&payload.task_type, &payload.payload)?;
  state.db.append_log(&task_id, "info", "task queued")?;

  let run_result = state
    .python
    .request(
      &app,
      "run_task",
      serde_json::json!({
        "task_type": payload.task_type,
        "payload": payload.payload,
      }),
    );

  match run_result {
    Ok(output) => {
      state.db.update_task_success(&task_id, &output)?;
      state.db.append_log(&task_id, "info", "task success")?;
      Ok(TaskRunResponse {
        task_id,
        status: "success".to_string(),
        output: Some(output),
        error: None,
      })
    }
    Err(err) => {
      state.db.update_task_failure(&task_id, &err.to_string())?;
      state
        .db
        .append_log(&task_id, "error", &format!("task failed: {}", err))?;
      Ok(TaskRunResponse {
        task_id,
        status: "failed".to_string(),
        output: None,
        error: Some(err.to_string()),
      })
    }
  }
}

#[tauri::command]
pub fn task_list(limit: Option<u32>, state: State<'_, AppState>) -> Result<Vec<TaskSummary>, AppError> {
  let safe_limit = limit.unwrap_or(50).clamp(1, 500);
  state.db.list_tasks(safe_limit)
}

#[tauri::command]
pub fn download_batch_list(
  page: Option<u32>,
  page_size: Option<u32>,
  state: State<'_, AppState>,
) -> Result<DownloadBatchListResult, AppError> {
  let safe_page = page.unwrap_or(1).max(1);
  let safe_page_size = page_size.unwrap_or(10).clamp(1, 100);
  state.db.list_download_batches(safe_page, safe_page_size)
}

#[tauri::command]
pub fn download_batch_detail(batch_id: String, state: State<'_, AppState>) -> Result<DownloadBatchDetail, AppError> {
  let trimmed = batch_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("batch_id is required".to_string()));
  }

  state
    .db
    .get_download_batch_detail(trimmed)?
    .ok_or_else(|| AppError::Validation("download batch not found".to_string()))
}

#[tauri::command]
pub fn task_batch_details(task_ids: Vec<String>, state: State<'_, AppState>) -> Result<Vec<TaskDetail>, AppError> {
  if task_ids.is_empty() {
    return Ok(Vec::new());
  }

  if task_ids.len() > 100 {
    return Err(AppError::Validation(
      "task_batch_details cannot query more than 100 task ids".to_string(),
    ));
  }

  state.db.get_task_batch_details(&task_ids)
}

#[tauri::command]
pub fn task_detail(task_id: String, state: State<'_, AppState>) -> Result<TaskRecordDetail, AppError> {
  let trimmed = task_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("task_id is required".to_string()));
  }

  state
    .db
    .get_task_detail(trimmed)?
    .ok_or_else(|| AppError::Validation("task not found".to_string()))
}

#[tauri::command]
pub fn download_batch_retry(batch_id: String, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
  let trimmed = batch_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("batch_id is required".to_string()));
  }

  let jobs = state
    .db
    .list_retryable_download_task_ids_for_batch(trimmed)?;

  if jobs.is_empty() {
    return Err(AppError::Validation("download batch has no retryable queued tasks".to_string()));
  }

  state.db.reset_download_batch_for_retry(trimmed)?;
  for task_id in jobs {
    state.db.update_task_status(&task_id, "queued")?;
    state.db.append_log(&task_id, "info", "video download batch retried")?;
  }

  append_runtime_log(&app, &format!("download batch retry requested batch_id={}", trimmed));
  crate::services::launch_batch_worker(&app, trimmed)?;

  Ok(())
}

#[tauri::command]
pub async fn open_download_batch_detail_window(
  batch_id: String,
  title: String,
  app: tauri::AppHandle,
) -> Result<(), AppError> {
  let trimmed = batch_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("batch_id is required".to_string()));
  }

  let normalized_title = normalize_window_title(&title, "Batch Detail");
  let route = format!("#/tasks/batch/{}", trimmed);
  let url = format!("index.html#/tasks/batch/{}", trimmed);

  append_runtime_log(&app, &format!("open batch detail window batch_id={}", trimmed));
  open_or_focus_window(
    &app,
    BATCH_DETAIL_WINDOW_LABEL,
    &normalized_title,
    &route,
    &url,
    980.0,
    780.0,
    840.0,
    620.0,
  )
}

#[tauri::command]
pub async fn open_task_detail_window(task_id: String, title: String, app: tauri::AppHandle) -> Result<(), AppError> {
  let trimmed = task_id.trim();
  if trimmed.is_empty() {
    return Err(AppError::Validation("task_id is required".to_string()));
  }

  let normalized_title = normalize_window_title(&title, "Task Detail");
  let route = format!("#/tasks/detail/{}", trimmed);
  let url = format!("index.html#/tasks/detail/{}", trimmed);

  append_runtime_log(&app, &format!("open task detail window task_id={}", trimmed));
  open_or_focus_window(
    &app,
    TASK_DETAIL_WINDOW_LABEL,
    &normalized_title,
    &route,
    &url,
    920.0,
    760.0,
    760.0,
    560.0,
  )
}

#[tauri::command]
pub fn video_download_submit(
  payload: VideoDownloadSubmitPayload,
  app: tauri::AppHandle,
  state: State<'_, AppState>,
) -> Result<VideoDownloadSubmitResult, AppError> {
  validate_download_platform(&payload.platform)?;

  if payload.urls.len() > 100 {
    return Err(AppError::Validation(
      "video download submission cannot exceed 100 urls".to_string(),
    ));
  }

  let mut seen = std::collections::HashSet::new();
  let mut accepted_urls = Vec::new();
  let mut invalid_urls = Vec::new();
  let mut skipped_count = 0_u32;

  for raw in payload.urls {
    let normalized = raw.trim();
    if normalized.is_empty() {
      skipped_count += 1;
      continue;
    }

    if !is_supported_download_url(normalized) {
      invalid_urls.push(normalized.to_string());
      continue;
    }

    if !seen.insert(normalized.to_string()) {
      skipped_count += 1;
      continue;
    }

    accepted_urls.push(normalized.to_string());
  }

  if accepted_urls.is_empty() {
    return Err(AppError::Validation(
      "video download submission must include at least one valid url".to_string(),
    ));
  }

  let platform = payload.platform;
  let download_cover = payload.download_cover;
  let download_music = payload.download_music;
  let download_description = payload.download_description;
  let download_lyric = payload.download_lyric;
  let batch_id = state
    .db
    .insert_download_batch(&platform, accepted_urls.len() as u32)?;
  let mut created_task_ids = Vec::with_capacity(accepted_urls.len());

  for source_url in accepted_urls {
    let task_payload = serde_json::json!({
      "batchId": batch_id.as_str(),
      "platform": platform.as_str(),
      "sourceUrl": source_url,
      "downloadCover": download_cover,
      "downloadMusic": download_music,
      "downloadDescription": download_description,
      "downloadLyric": download_lyric,
    });

    let task_id = state.db.insert_task("video.download", &task_payload)?;
    state.db.append_log(&task_id, "info", "video download task queued")?;
    created_task_ids.push(task_id);
  }

  crate::services::launch_batch_worker(&app, &batch_id)?;

  Ok(VideoDownloadSubmitResult {
    accepted_count: created_task_ids.len() as u32,
    skipped_count,
    invalid_urls,
    created_task_ids,
  })
}

fn validate_download_platform(value: &str) -> Result<(), AppError> {
  match value.trim() {
    "douyin" | "tiktok" => Ok(()),
    _ => Err(AppError::Validation(
      "platform must be one of douyin/tiktok".to_string(),
    )),
  }
}

fn is_supported_download_url(value: &str) -> bool {
  let lowered = value.trim().to_ascii_lowercase();
  lowered.starts_with("http://") || lowered.starts_with("https://")
}
