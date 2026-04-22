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
  environment_download, environment_status, open_download_batch_detail_window, open_external_url,
  open_recording_account_create_window,
  open_recording_account_edit_window, open_recording_account_logs_window, open_task_detail_window, python_ping,
  recording_account_create, recording_account_delete, recording_account_detail, recording_account_logs,
  recording_account_set_enabled, recording_account_update, recording_accounts_check, recording_accounts_snapshot,
  resolve_recording_account_profile, select_export_directory, settings_check_update, settings_get, settings_update,
  system_info, task_batch_details, task_detail, task_list, task_run, token_validate, video_download_submit,
};
use tauri::Manager;
#[cfg(target_os = "macos")]
use tauri::menu::{
  AboutMetadata, HELP_SUBMENU_ID, Menu, PredefinedMenuItem, Submenu, WINDOW_SUBMENU_ID,
};
use crate::services::runtime_log::append_runtime_log;

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      install_macos_menu(app)?;
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
      token_validate,
      environment_status,
      environment_download
    ])
    .run(tauri::generate_context!())
    .expect("failed to run tauri application");
}

#[cfg(target_os = "macos")]
fn install_macos_menu(app: &mut tauri::App) -> tauri::Result<()> {
  let handle = app.handle();
  let pkg_info = handle.package_info();
  let config = handle.config();
  let about_text = format!("关于 {}", pkg_info.name);
  let hide_text = format!("隐藏 {}", pkg_info.name);
  let quit_text = format!("退出 {}", pkg_info.name);
  let about_metadata = AboutMetadata {
    name: Some(pkg_info.name.clone()),
    version: Some(pkg_info.version.to_string()),
    copyright: config.bundle.copyright.clone(),
    authors: config.bundle.publisher.clone().map(|publisher| vec![publisher]),
    ..Default::default()
  };

  let app_menu = Submenu::with_items(
    handle,
    pkg_info.name.clone(),
    true,
    &[
      &PredefinedMenuItem::about(handle, Some(&about_text), Some(about_metadata))?,
      &PredefinedMenuItem::separator(handle)?,
      &PredefinedMenuItem::services(handle, Some("服务"))?,
      &PredefinedMenuItem::separator(handle)?,
      &PredefinedMenuItem::hide(handle, Some(&hide_text))?,
      &PredefinedMenuItem::hide_others(handle, Some("隐藏其他"))?,
      &PredefinedMenuItem::separator(handle)?,
      &PredefinedMenuItem::quit(handle, Some(&quit_text))?,
    ],
  )?;
  let file_menu = Submenu::with_items(
    handle,
    "文件",
    true,
    &[&PredefinedMenuItem::close_window(handle, Some("关闭窗口"))?],
  )?;
  let edit_menu = Submenu::with_items(
    handle,
    "编辑",
    true,
    &[
      &PredefinedMenuItem::undo(handle, Some("撤销"))?,
      &PredefinedMenuItem::redo(handle, Some("重做"))?,
      &PredefinedMenuItem::separator(handle)?,
      &PredefinedMenuItem::cut(handle, Some("剪切"))?,
      &PredefinedMenuItem::copy(handle, Some("复制"))?,
      &PredefinedMenuItem::paste(handle, Some("粘贴"))?,
      &PredefinedMenuItem::select_all(handle, Some("全选"))?,
    ],
  )?;
  let view_menu = Submenu::with_items(
    handle,
    "显示",
    true,
    &[&PredefinedMenuItem::fullscreen(handle, Some("切换全屏"))?],
  )?;
  let window_menu = Submenu::with_id_and_items(
    handle,
    WINDOW_SUBMENU_ID,
    "窗口",
    true,
    &[
      &PredefinedMenuItem::minimize(handle, Some("最小化"))?,
      &PredefinedMenuItem::maximize(handle, Some("缩放"))?,
      &PredefinedMenuItem::separator(handle)?,
      &PredefinedMenuItem::close_window(handle, Some("关闭窗口"))?,
    ],
  )?;
  let help_menu = Submenu::with_id_and_items(handle, HELP_SUBMENU_ID, "帮助", true, &[])?;
  let menu = Menu::with_items(
    handle,
    &[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu, &help_menu],
  )?;

  app.set_menu(menu)?;
  Ok(())
}

#[cfg(not(target_os = "macos"))]
fn install_macos_menu(_app: &mut tauri::App) -> tauri::Result<()> {
  Ok(())
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
