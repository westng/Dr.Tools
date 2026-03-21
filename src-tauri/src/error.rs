use std::io;

use rusqlite::Error as SqlError;
use serde::Serialize;
use serde_json::Error as JsonError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
  #[error("validation error: {0}")]
  Validation(String),
  #[error("python start error: {0}")]
  PythonStart(String),
  #[error("task execution error: {0}")]
  TaskExec(String),
  #[error("database error: {0}")]
  Database(String),
  #[error("io error: {0}")]
  Io(String),
  #[error("json error: {0}")]
  Json(String),
}

impl AppError {
  fn code(&self) -> &'static str {
    match self {
      Self::Validation(_) => "VALIDATION_ERROR",
      Self::PythonStart(_) => "PYTHON_START_ERROR",
      Self::TaskExec(_) => "TASK_EXEC_ERROR",
      Self::Database(_) => "DB_ERROR",
      Self::Io(_) => "IO_ERROR",
      Self::Json(_) => "JSON_ERROR",
    }
  }
}

#[derive(Serialize)]
struct ErrorPayload {
  code: &'static str,
  message: String,
}

impl Serialize for AppError {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let payload = ErrorPayload {
      code: self.code(),
      message: self.to_string(),
    };
    payload.serialize(serializer)
  }
}

impl From<SqlError> for AppError {
  fn from(value: SqlError) -> Self {
    Self::Database(value.to_string())
  }
}

impl From<io::Error> for AppError {
  fn from(value: io::Error) -> Self {
    Self::Io(value.to_string())
  }
}

impl From<JsonError> for AppError {
  fn from(value: JsonError) -> Self {
    Self::Json(value.to_string())
  }
}
