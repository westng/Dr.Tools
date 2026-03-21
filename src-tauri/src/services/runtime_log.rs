use std::fs::{self, OpenOptions};
use std::io::Write;

use chrono::Utc;
use tauri::{AppHandle, Manager};

pub fn append_runtime_log(app: &AppHandle, message: &str) {
  let Ok(dir) = app.path().app_data_dir() else {
    return;
  };

  if fs::create_dir_all(&dir).is_err() {
    return;
  }

  let path = dir.join("runtime.log");
  let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
    return;
  };

  let _ = writeln!(file, "{} {}", Utc::now().to_rfc3339(), message);
}
