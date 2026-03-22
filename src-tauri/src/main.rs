mod commands;
mod application;
mod domain;
mod repositories;
mod services;
mod error;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic;
use std::path::PathBuf;

use commands::{
  clear_recording_runs, download_batch_detail, download_batch_list, download_batch_retry, frontend_log_error,
  open_download_batch_detail_window, open_external_url, open_recording_account_create_window,
  open_recording_account_edit_window, open_recording_account_logs_window, open_task_detail_window, python_ping,
  recording_account_create, recording_account_delete, recording_account_detail, recording_account_logs,
  recording_account_set_enabled, recording_account_update, recording_accounts_check, recording_accounts_snapshot,
  resolve_recording_account_profile, select_export_directory, settings_check_update, settings_get, settings_update,
  system_info, task_batch_details, task_detail, task_list, task_run, token_validate, video_download_submit,
};
use tauri::Manager;
use crate::services::runtime_log::append_runtime_log;

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      install_panic_hook(resolve_panic_log_path(app.handle()));
      let app_state = application::AppState::bootstrap(app.handle())?;
      app.manage(app_state);
      app
        .state::<application::AppState>()
        .recording_scheduler
        .start(app.handle().clone())?;
      Ok(())
    })
    .on_window_event(|window, event| match event {
      tauri::WindowEvent::CloseRequested { .. } => {
        append_runtime_log(&window.app_handle(), &format!("window close requested label={}", window.label()));
      }
      tauri::WindowEvent::Destroyed => {
        append_runtime_log(&window.app_handle(), &format!("window destroyed label={}", window.label()));
      }
      _ => {}
    })
    .invoke_handler(tauri::generate_handler![
      system_info,
      python_ping,
      frontend_log_error,
      open_external_url,
      task_run,
      task_list,
      download_batch_list,
      download_batch_detail,
      download_batch_retry,
      open_download_batch_detail_window,
      open_recording_account_create_window,
      open_recording_account_edit_window,
      open_recording_account_logs_window,
      recording_accounts_snapshot,
      clear_recording_runs,
      recording_account_create,
      recording_account_detail,
      recording_account_update,
      recording_account_set_enabled,
      recording_account_delete,
      recording_account_logs,
      recording_accounts_check,
      resolve_recording_account_profile,
      open_task_detail_window,
      task_batch_details,
      task_detail,
      video_download_submit,
      settings_get,
      settings_update,
      settings_check_update,
      select_export_directory,
      token_validate
    ])
    .run(tauri::generate_context!())
    .expect("failed to run tauri application");
}

fn resolve_panic_log_path(app: &tauri::AppHandle) -> PathBuf {
  let base = app
    .path()
    .app_data_dir()
    .ok()
    .unwrap_or_else(|| std::env::temp_dir());
  let _ = fs::create_dir_all(&base);
  base.join("main-process.panic.log")
}

fn install_panic_hook(log_path: PathBuf) {
  panic::set_hook(Box::new(move |panic_info| {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
      let _ = writeln!(file, "panic: {}", panic_info);
    }
  }));
}
