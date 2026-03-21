use std::env;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::services::runtime_log::append_runtime_log;

pub fn launch_batch_worker(app: &AppHandle, batch_id: &str) -> Result<(), AppError> {
  let db_path = app
    .path()
    .app_data_dir()
    .map_err(|e| AppError::Io(e.to_string()))?
    .join("drtools.db");
  let (python_bin, script_path, cwd) = resolve_batch_worker_launch(app)?;

  append_runtime_log(
    app,
    &format!(
      "launch batch worker batch_id={} bin={} script={}",
      batch_id, python_bin, script_path
    ),
  );

  let mut command = Command::new(python_bin);
  command
    .arg(script_path)
    .arg(&db_path)
    .arg(batch_id)
    .env("PYTHONDONTWRITEBYTECODE", "1")
    .env("PYTHONUNBUFFERED", "1")
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(resolve_worker_stderr(app));

  if let Some(cwd) = cwd {
    command.current_dir(cwd);
  }

  let child = command.spawn().map_err(|e| AppError::PythonStart(e.to_string()))?;
  append_runtime_log(app, &format!("batch worker spawned batch_id={} pid={}", batch_id, child.id()));
  Ok(())
}

fn resolve_batch_worker_launch(app: &AppHandle) -> Result<(String, String, Option<PathBuf>), AppError> {
  if let Ok(bin) = env::var("DRTOOLS_PYTHON_BIN") {
    let script = env::var("DRTOOLS_BATCH_WORKER_SCRIPT").unwrap_or_else(|_| "python/batch_worker.py".to_string());
    return Ok((bin, script, None));
  }

  if cfg!(debug_assertions) {
    if let Some((bin, script)) = detect_manifest_batch_worker_launch() {
      return Ok((bin, script, None));
    }
  }

  let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|e| AppError::PythonStart(e.to_string()))?;
  let script_path = resource_dir.join("python").join("batch_worker.py");

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
    "embedded python runtime not found for batch worker".to_string(),
  ))
}

fn detect_manifest_batch_worker_launch() -> Option<(String, String)> {
  let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let python = manifest_dir.parent()?.join(".venv").join("bin").join("python");
  let script = manifest_dir.join("python").join("batch_worker.py");
  if python.exists() && script.exists() {
    return Some((
      python.to_string_lossy().to_string(),
      script.to_string_lossy().to_string(),
    ));
  }
  None
}

fn resolve_worker_stderr(app: &AppHandle) -> Stdio {
  let Ok(dir) = app.path().app_data_dir() else {
    return Stdio::null();
  };

  if fs::create_dir_all(&dir).is_err() {
    return Stdio::null();
  }

  let path = dir.join("batch-worker.stderr.log");
  match OpenOptions::new().create(true).append(true).open(path) {
    Ok(file) => Stdio::from(file),
    Err(_) => Stdio::null(),
  }
}
