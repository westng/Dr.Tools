use std::fs;

use tauri::{AppHandle, Manager};

use crate::repositories::Db;
use crate::error::AppError;
use crate::services::{PythonManager, RecordingScheduler};

pub struct AppState {
  pub db: Db,
  pub python: PythonManager,
  pub recording_scheduler: RecordingScheduler,
}

impl AppState {
  pub fn bootstrap(app: &AppHandle) -> Result<Self, AppError> {
    let data_dir = app
      .path()
      .app_data_dir()
      .map_err(|e| AppError::Io(e.to_string()))?;

    fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("drtools.db");
    let db = Db::new(db_path.clone());
    db.init()?;

    Ok(Self {
      db,
      python: PythonManager::new(),
      recording_scheduler: RecordingScheduler::new(db_path),
    })
  }
}

impl Drop for AppState {
  fn drop(&mut self) {
    let _ = self.recording_scheduler.stop();
    let _ = self.python.stop();
  }
}
