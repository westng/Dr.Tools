use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
  pub app_name: String,
  pub app_version: String,
  pub os: String,
  pub arch: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRunRequest {
  pub task_type: String,
  pub payload: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRunResponse {
  pub task_id: String,
  pub status: String,
  pub output: Option<Value>,
  pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
  pub id: String,
  pub task_type: String,
  pub status: String,
  pub created_at: String,
  pub updated_at: String,
  pub error_text: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskLogEntry {
  pub task_id: String,
  pub level: String,
  pub message: String,
  pub ts: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
  pub id: String,
  pub task_type: String,
  pub status: String,
  pub created_at: String,
  pub updated_at: String,
  pub error_text: Option<String>,
  pub logs: Vec<TaskLogEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecordDetail {
  pub id: String,
  pub task_type: String,
  pub status: String,
  pub created_at: String,
  pub updated_at: String,
  pub error_text: Option<String>,
  pub input: Option<Value>,
  pub output: Option<Value>,
  pub logs: Vec<TaskLogEntry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadBatchSummary {
  pub id: String,
  pub platform: String,
  pub status: String,
  pub total_count: u32,
  pub success_count: u32,
  pub failed_count: u32,
  pub running_count: u32,
  pub created_at: String,
  pub updated_at: String,
  pub completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadBatchListResult {
  pub items: Vec<DownloadBatchSummary>,
  pub total: u32,
  pub page: u32,
  pub page_size: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadBatchTaskItem {
  pub id: String,
  pub task_type: String,
  pub status: String,
  pub source_url: Option<String>,
  pub author_name: Option<String>,
  pub author_uid: Option<String>,
  pub created_at: String,
  pub updated_at: String,
  pub error_text: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadBatchDetail {
  pub id: String,
  pub platform: String,
  pub status: String,
  pub total_count: u32,
  pub success_count: u32,
  pub failed_count: u32,
  pub running_count: u32,
  pub created_at: String,
  pub updated_at: String,
  pub completed_at: Option<String>,
  pub tasks: Vec<DownloadBatchTaskItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PythonRequest {
  pub id: u64,
  pub method: String,
  pub params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PythonResponse {
  pub id: u64,
  pub result: Option<Value>,
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
  pub theme_mode: String,
  pub liquid_glass_style: String,
  pub accent_color: String,
  pub locale: String,
  pub export_dir: String,
  pub max_concurrent_downloads: u32,
  pub download_notifications_enabled: bool,
  pub douyin_cookie: String,
  pub douyin_cookie_updated_at: Option<String>,
  pub douyin_last_checked_at: Option<String>,
  pub douyin_last_check_status: String,
  pub douyin_last_check_message: Option<String>,
  pub tiktok_cookie: String,
  pub tiktok_cookie_updated_at: Option<String>,
  pub tiktok_last_checked_at: Option<String>,
  pub tiktok_last_check_status: String,
  pub tiktok_last_check_message: Option<String>,
  pub auto_check_updates: bool,
  pub last_update_check_at: Option<String>,
  pub last_update_status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsPatch {
  pub theme_mode: Option<String>,
  pub liquid_glass_style: Option<String>,
  pub accent_color: Option<String>,
  pub locale: Option<String>,
  pub export_dir: Option<String>,
  pub max_concurrent_downloads: Option<u32>,
  pub download_notifications_enabled: Option<bool>,
  pub douyin_cookie: Option<String>,
  pub tiktok_cookie: Option<String>,
  pub auto_check_updates: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenValidationPayload {
  pub platform: String,
  pub cookie: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenValidationResult {
  pub platform: String,
  pub checked_at: String,
  pub status: String,
  pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAccountResolvePayload {
  pub platform: String,
  pub source_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAccountResolveResult {
  pub platform: String,
  pub account_input: String,
  pub account_name: String,
  pub account_uid: String,
  pub account_avatar_url: Option<String>,
  pub account_room_id: Option<String>,
  pub account_web_rid: Option<String>,
  pub account_sec_user_id: Option<String>,
  pub account_unique_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAccountItem {
  pub id: String,
  pub platform: String,
  pub account_input: String,
  pub account_name: String,
  pub account_uid: String,
  pub account_avatar_url: Option<String>,
  pub account_room_id: Option<String>,
  pub account_web_rid: Option<String>,
  pub account_sec_user_id: Option<String>,
  pub account_unique_id: Option<String>,
  pub auto_start: bool,
  pub retry_on_disconnect: bool,
  pub split_recording: bool,
  pub save_snapshot: bool,
  pub enabled: bool,
  pub status: String,
  pub last_checked_at: Option<String>,
  pub last_recorded_at: Option<String>,
  pub last_error: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAccountLogEntry {
  pub account_id: String,
  pub level: String,
  pub message: String,
  pub ts: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingRunItem {
  pub id: String,
  pub account_id: String,
  pub platform: String,
  pub account_name: String,
  pub status: String,
  pub created_at: String,
  pub updated_at: String,
  pub error_text: Option<String>,
  pub output_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAccountsSnapshot {
  pub accounts: Vec<RecordingAccountItem>,
  pub logs: Vec<RecordingAccountLogEntry>,
  pub runs: Vec<RecordingRunItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAccountCreatePayload {
  pub platform: String,
  pub account_input: String,
  pub account_name: String,
  pub account_uid: String,
  pub account_avatar_url: Option<String>,
  pub account_room_id: Option<String>,
  pub account_web_rid: Option<String>,
  pub account_sec_user_id: Option<String>,
  pub account_unique_id: Option<String>,
  pub auto_start: bool,
  pub retry_on_disconnect: bool,
  pub split_recording: bool,
  pub save_snapshot: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingAccountUpdatePayload {
  pub account_id: String,
  pub platform: String,
  pub account_input: String,
  pub account_name: String,
  pub account_uid: String,
  pub account_avatar_url: Option<String>,
  pub account_room_id: Option<String>,
  pub account_web_rid: Option<String>,
  pub account_sec_user_id: Option<String>,
  pub account_unique_id: Option<String>,
  pub auto_start: bool,
  pub retry_on_disconnect: bool,
  pub split_recording: bool,
  pub save_snapshot: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingLiveStatusResult {
  pub platform: String,
  pub status: String,
  pub account_room_id: Option<String>,
  pub account_web_rid: Option<String>,
  pub live_title: Option<String>,
  pub checked_at: String,
  pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDownloadSubmitPayload {
  pub platform: String,
  pub urls: Vec<String>,
  pub download_cover: bool,
  pub download_music: bool,
  pub download_description: bool,
  pub download_lyric: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDownloadSubmitResult {
  pub created_task_ids: Vec<String>,
  pub accepted_count: u32,
  pub skipped_count: u32,
  pub invalid_urls: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
  pub checked_at: String,
  pub status: String,
  pub message: String,
  pub current_version: String,
  pub latest_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedEnvironmentStatus {
  pub python_version: String,
  pub ffmpeg_version: String,
  pub source_label: String,
  pub source_url: String,
  pub install_dir: String,
  pub python_bin: Option<String>,
  pub ffmpeg_bin: Option<String>,
  pub installed: bool,
  pub status: String,
  pub message: String,
}
