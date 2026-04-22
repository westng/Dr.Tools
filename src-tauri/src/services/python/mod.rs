use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::error::AppError;
use crate::domain::{PythonRequest, PythonResponse};
use crate::services::runtime_log::append_runtime_log;

pub const MANAGED_PYTHON_VERSION: &str = "3.12.12";
pub const MANAGED_PYTHON_RELEASE: &str = "20260203";
pub const MANAGED_PYTHON_SOURCE_LABEL: &str = "NJU Mirror · python-build-standalone";
pub const MANAGED_PYTHON_SOURCE_BASE_URL: &str =
  "https://mirror.nju.edu.cn/github-release/astral-sh/python-build-standalone";

struct PythonProcess {
  child: Child,
  stdin: ChildStdin,
  stdout: BufReader<ChildStdout>,
  next_id: u64,
}

struct PythonState {
  process: Option<PythonProcess>,
}

pub struct PythonManager {
  inner: Mutex<PythonState>,
}

impl PythonManager {
  pub fn new() -> Self {
    Self {
      inner: Mutex::new(PythonState { process: None }),
    }
  }

  pub fn request(&self, app: &AppHandle, method: &str, params: Value) -> Result<Value, AppError> {
    append_runtime_log(app, &format!("python shared request start method={}", method));
    let mut guard = self
      .inner
      .lock()
      .map_err(|_| AppError::TaskExec("python mutex poisoned".to_string()))?;
    ensure_started(&mut guard, app)?;

    let process = guard
      .process
      .as_mut()
      .ok_or_else(|| AppError::PythonStart("python process missing".to_string()))?;

    let id = process.next_id;
    process.next_id = process.next_id.saturating_add(1);

    let request = PythonRequest {
      id,
      method: method.to_string(),
      params,
    };

    let request_line = serde_json::to_string(&request)?;
    writeln!(process.stdin, "{}", request_line)?;
    process.stdin.flush()?;

    let response = read_python_response(&mut process.stdout, "python process")?;
    if response.id != id {
      append_runtime_log(app, &format!("python shared response mismatch method={} request_id={} response_id={}", method, id, response.id));
      return Err(AppError::TaskExec("python response id mismatch".to_string()));
    }

    if let Some(error) = response.error {
      append_runtime_log(app, &format!("python shared request failed method={} error={}", method, error));
      return Err(AppError::TaskExec(error));
    }

    append_runtime_log(app, &format!("python shared request success method={}", method));
    Ok(response.result.unwrap_or_else(|| json!({})))
  }

  pub fn request_isolated(&self, app: &AppHandle, method: &str, params: Value) -> Result<Value, AppError> {
    append_runtime_log(app, &format!("python isolated request start method={}", method));
    let request = PythonRequest {
      id: 1,
      method: method.to_string(),
      params,
    };

    let (bin, args, cwd) = resolve_python_launch(app)?;
    let request_id = Uuid::new_v4().to_string();
    let temp_dir = resolve_python_ipc_dir(app)?;
    let request_path = temp_dir.join(format!("{request_id}.request.json"));
    let response_path = temp_dir.join(format!("{request_id}.response.json"));
    fs::write(&request_path, serde_json::to_vec(&request)?)?;
    append_runtime_log(
      app,
      &format!("python isolated launch method={} bin={} args={:?}", method, bin, args),
    );
    let mut command = Command::new(bin);
    command
      .args(args)
      .arg(&request_path)
      .arg(&response_path)
      .env("PYTHONDONTWRITEBYTECODE", "1")
      .env("PYTHONUNBUFFERED", "1")
      .stdin(Stdio::piped())
      .stdout(Stdio::null())
      .stderr(resolve_isolated_stderr(app));

    if let Some(cwd) = cwd {
      append_runtime_log(app, &format!("python isolated cwd method={} cwd={}", method, cwd.display()));
      command.current_dir(cwd);
    }

    let mut child = command
      .spawn()
      .map_err(|e| AppError::PythonStart(e.to_string()))?;
    append_runtime_log(app, &format!("python isolated spawned method={} pid={}", method, child.id()));

    let status = child.wait()?;
    append_runtime_log(
      app,
      &format!("python isolated process exit method={} status={}", method, status),
    );

    let response_raw = fs::read_to_string(&response_path).map_err(|error| {
      AppError::TaskExec(format!(
        "isolated python process did not produce a response file: {}",
        error
      ))
    })?;
    let response: PythonResponse = serde_json::from_str(&response_raw)?;
    let _ = fs::remove_file(&request_path);
    let _ = fs::remove_file(&response_path);

    if response.id != request.id {
      return Err(AppError::TaskExec("python response id mismatch".to_string()));
    }

    if let Some(error) = response.error {
      append_runtime_log(app, &format!("python isolated request failed method={} error={}", method, error));
      return Err(AppError::TaskExec(error));
    }

    append_runtime_log(app, &format!("python isolated request success method={}", method));
    Ok(response.result.unwrap_or_else(|| json!({})))
  }

  pub fn stop(&self) -> Result<(), AppError> {
    let mut guard = self
      .inner
      .lock()
      .map_err(|_| AppError::TaskExec("python mutex poisoned".to_string()))?;

    if let Some(process) = guard.process.as_mut() {
      process.child.kill()?;
      process.child.wait()?;
    }
    guard.process = None;
    Ok(())
  }
}

fn read_python_response(reader: &mut BufReader<ChildStdout>, process_name: &str) -> Result<PythonResponse, AppError> {
  let mut ignored_lines: Vec<String> = Vec::new();

  loop {
    let mut line = String::new();
    let read_count = reader.read_line(&mut line)?;
    if read_count == 0 {
      if ignored_lines.is_empty() {
        return Err(AppError::TaskExec(format!(
          "{process_name} closed unexpectedly"
        )));
      }

      return Err(AppError::TaskExec(format!(
        "{process_name} returned no valid JSON response; ignored stdout: {}",
        ignored_lines.join(" | ")
      )));
    }

    let trimmed = line.trim();
    if trimmed.is_empty() {
      continue;
    }

    match serde_json::from_str::<PythonResponse>(trimmed) {
      Ok(response) => return Ok(response),
      Err(_) => {
        ignored_lines.push(trimmed.to_string());
        if ignored_lines.len() >= 3 {
          return Err(AppError::TaskExec(format!(
            "{process_name} returned non-JSON stdout: {}",
            ignored_lines.join(" | ")
          )));
        }
      }
    }
  }
}

fn ensure_started(state: &mut PythonState, app: &AppHandle) -> Result<(), AppError> {
  if let Some(process) = state.process.as_mut() {
    if process.child.try_wait()?.is_none() {
      return Ok(());
    }
    state.process = None;
  }

  let (bin, args, cwd) = resolve_python_launch(app)?;
  append_runtime_log(app, &format!("python shared launch bin={} args={:?}", bin, args));
  let mut command = Command::new(bin);
  command
    .args(args)
    .env("PYTHONDONTWRITEBYTECODE", "1")
    .env("PYTHONUNBUFFERED", "1")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(resolve_shared_stderr(app));

  if let Some(cwd) = cwd {
    append_runtime_log(app, &format!("python shared cwd={}", cwd.display()));
    command.current_dir(cwd);
  }

  let mut child = command
    .spawn()
    .map_err(|e| AppError::PythonStart(e.to_string()))?;
  append_runtime_log(app, &format!("python shared spawned pid={}", child.id()));

  let stdin = child
    .stdin
    .take()
    .ok_or_else(|| AppError::PythonStart("failed to capture python stdin".to_string()))?;
  let stdout = child
    .stdout
    .take()
    .ok_or_else(|| AppError::PythonStart("failed to capture python stdout".to_string()))?;

  state.process = Some(PythonProcess {
    child,
    stdin,
    stdout: BufReader::new(stdout),
    next_id: 1,
  });

  Ok(())
}

fn resolve_python_launch(app: &AppHandle) -> Result<(String, Vec<String>, Option<PathBuf>), AppError> {
  if let Ok(bin) = env::var("DRTOOLS_PYTHON_BIN") {
    let script = env::var("DRTOOLS_PYTHON_SCRIPT").unwrap_or_else(|_| "python/main.py".to_string());
    return Ok((bin, vec![script], None));
  }

  if let Some(runtime_bin) = managed_runtime_bin_path(app) {
    let script = resolve_python_script_path(app)?;
    return Ok((
      runtime_bin.to_string_lossy().to_string(),
      vec![script.to_string_lossy().to_string()],
      None,
    ));
  }

  if cfg!(debug_assertions) {
    if let Some((venv_python, script)) = detect_manifest_python_launch() {
      return Ok((venv_python, vec![script], None));
    }

    if let Some(venv_python) = detect_workspace_venv_python() {
      let script = detect_workspace_python_script();
      return Ok((venv_python, vec![script], None));
    }

    return Ok(("python3".to_string(), vec!["python/main.py".to_string()], None));
  }

  let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|e| AppError::PythonStart(e.to_string()))?;
  let script_path = resource_dir.join("python").join("main.py");

  let runtime_bin = if cfg!(target_os = "windows") {
    resource_dir.join("python-runtime").join("python.exe")
  } else {
    resource_dir.join("python-runtime").join("python")
  };

  if runtime_bin.exists() && script_path.exists() {
    return Ok((
      runtime_bin.to_string_lossy().to_string(),
      vec![script_path.to_string_lossy().to_string()],
      Some(resource_dir),
    ));
  }

  Err(AppError::PythonStart(
    "embedded python runtime not found; set DRTOOLS_PYTHON_BIN for development".to_string(),
  ))
}

pub fn managed_runtime_root(app: &AppHandle) -> Option<PathBuf> {
  let base = app.path().app_data_dir().ok()?;
  Some(
    base
      .join("python-runtime-managed")
      .join(format!("python-{}", MANAGED_PYTHON_VERSION)),
  )
}

pub fn managed_runtime_bin_path(app: &AppHandle) -> Option<PathBuf> {
  let root = managed_runtime_root(app)?;
  let bin = if cfg!(target_os = "windows") {
    root.join("python").join("python.exe")
  } else {
    root.join("python").join("bin").join("python")
  };

  if bin.exists() {
    Some(bin)
  } else {
    None
  }
}

pub fn resolve_python_script_path(app: &AppHandle) -> Result<PathBuf, AppError> {
  if cfg!(debug_assertions) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_script = manifest_dir.join("python").join("main.py");
    if dev_script.exists() {
      return Ok(dev_script);
    }
  }

  let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|e| AppError::PythonStart(e.to_string()))?;
  let bundled_script = resource_dir.join("python").join("main.py");
  if bundled_script.exists() {
    Ok(bundled_script)
  } else {
    Err(AppError::PythonStart(
      "python main script not found in bundled resources".to_string(),
    ))
  }
}

fn detect_workspace_venv_python() -> Option<String> {
  let current_dir = env::current_dir().ok()?;
  let parent = current_dir.parent()?;
  let candidates = [
    current_dir.join(".venv").join("bin").join("python"),
    parent.join(".venv").join("bin").join("python"),
  ];

  candidates
    .into_iter()
    .find(|path| path.exists())
    .map(|path| path.to_string_lossy().to_string())
}

fn detect_workspace_python_script() -> String {
  let current_dir = match env::current_dir() {
    Ok(value) => value,
    Err(_) => return "python/main.py".to_string(),
  };

  let mut candidates = vec![current_dir.join("python").join("main.py")];
  candidates.push(current_dir.join("src-tauri").join("python").join("main.py"));
  if let Some(parent) = current_dir.parent() {
    candidates.push(parent.join("src-tauri").join("python").join("main.py"));
  }

  candidates
    .into_iter()
    .find(|path| path.exists())
    .map(|path| path.to_string_lossy().to_string())
    .unwrap_or_else(|| "python/main.py".to_string())
}

fn detect_manifest_python_launch() -> Option<(String, String)> {
  let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let python = manifest_dir.parent()?.join(".venv").join("bin").join("python");
  let script = manifest_dir.join("python").join("main.py");

  if python.exists() && script.exists() {
    return Some((
      python.to_string_lossy().to_string(),
      script.to_string_lossy().to_string(),
    ));
  }

  None
}

fn resolve_shared_stderr(app: &AppHandle) -> Stdio {
  let Ok(dir) = app.path().app_data_dir() else {
    return Stdio::null();
  };

  if fs::create_dir_all(&dir).is_err() {
    return Stdio::null();
  }

  let path = dir.join("python-shared.stderr.log");
  match OpenOptions::new().create(true).append(true).open(path) {
    Ok(file) => Stdio::from(file),
    Err(_) => Stdio::null(),
  }
}

fn resolve_isolated_stderr(app: &AppHandle) -> Stdio {
  let Ok(dir) = app.path().app_data_dir() else {
    return Stdio::null();
  };

  if fs::create_dir_all(&dir).is_err() {
    return Stdio::null();
  }

  let path = dir.join("python-isolated.stderr.log");
  match OpenOptions::new().create(true).append(true).open(path) {
    Ok(file) => Stdio::from(file),
    Err(_) => Stdio::null(),
  }
}

fn resolve_python_ipc_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
  let dir = app
    .path()
    .app_data_dir()
    .map_err(|e| AppError::Io(e.to_string()))?
    .join("python-ipc");
  fs::create_dir_all(&dir)?;
  Ok(dir)
}
