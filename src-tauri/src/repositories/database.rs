use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use uuid::Uuid;

use crate::error::AppError;
use crate::domain::{
  DownloadBatchDetail, DownloadBatchListResult, DownloadBatchSummary, DownloadBatchTaskItem, TaskDetail, TaskLogEntry,
  TaskRecordDetail, TaskSummary, RecordingAccountCreatePayload, RecordingAccountItem, RecordingAccountLogEntry,
  RecordingRunItem, RecordingAccountUpdatePayload,
};

pub struct Db {
  path: PathBuf,
}

#[derive(Clone, Copy, Default)]
struct BatchTaskStats {
  total_seen: u32,
  queued_count: u32,
  running_count: u32,
  success_count: u32,
  failed_count: u32,
}

impl Db {
  pub fn new(path: PathBuf) -> Self {
    Self { path }
  }

  pub fn init(&self) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute_batch(
      "CREATE TABLE IF NOT EXISTS _migrations (
        version TEXT PRIMARY KEY,
        applied_at TEXT NOT NULL
      );",
    )?;

    self.apply_migration(&conn, "001_init", include_str!("../../migrations/001_init.sql"))?;
    self.apply_migration(
      &conn,
      "002_download_batches",
      include_str!("../../migrations/002_download_batches.sql"),
    )?;
    self.apply_migration(
      &conn,
      "003_recording_accounts",
      include_str!("../../migrations/003_recording_accounts.sql"),
    )?;
    self.apply_migration(
      &conn,
      "004_recording_account_rules",
      include_str!("../../migrations/004_recording_account_rules.sql"),
    )?;
    Ok(())
  }

  fn apply_migration(&self, conn: &Connection, version: &str, sql: &str) -> Result<(), AppError> {
    let exists: i64 = conn.query_row(
      "SELECT COUNT(1) FROM _migrations WHERE version = ?1",
      params![version],
      |row| row.get(0),
    )?;
    if exists > 0 {
      return Ok(());
    }

    conn.execute_batch(sql)?;
    conn.execute(
      "INSERT INTO _migrations (version, applied_at) VALUES (?1, ?2)",
      params![version, Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn insert_task(&self, task_type: &str, input_json: &Value) -> Result<String, AppError> {
    let conn = self.open()?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
      "INSERT INTO tasks (
        id, task_type, status, input_json, output_json, error_text, created_at, updated_at
      ) VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?5)",
      params![id, task_type, "queued", input_json.to_string(), now],
    )?;

    Ok(id)
  }

  pub fn insert_download_batch(&self, platform: &str, total_count: u32) -> Result<String, AppError> {
    let conn = self.open()?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
      "INSERT INTO download_batches (
        id, platform, total_count, success_count, failed_count, completion_handled, created_at, updated_at, completed_at
      ) VALUES (?1, ?2, ?3, 0, 0, 0, ?4, ?4, NULL)",
      params![id, platform, total_count, now],
    )?;

    Ok(id)
  }

  pub fn update_task_success(&self, task_id: &str, output_json: &Value) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "UPDATE tasks SET status = ?2, output_json = ?3, error_text = NULL, updated_at = ?4 WHERE id = ?1",
      params![task_id, "success", output_json.to_string(), Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn update_task_status(&self, task_id: &str, status: &str) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "UPDATE tasks SET status = ?2, updated_at = ?3 WHERE id = ?1",
      params![task_id, status, Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn update_task_failure(&self, task_id: &str, error_text: &str) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "UPDATE tasks SET status = ?2, error_text = ?3, updated_at = ?4 WHERE id = ?1",
      params![task_id, "failed", error_text, Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn append_log(&self, task_id: &str, level: &str, message: &str) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "INSERT INTO task_logs (id, task_id, level, message, ts) VALUES (?1, ?2, ?3, ?4, ?5)",
      params![Uuid::new_v4().to_string(), task_id, level, message, Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn list_tasks(&self, limit: u32) -> Result<Vec<TaskSummary>, AppError> {
    let conn = self.open()?;
    let mut stmt = conn.prepare(
      "SELECT id, task_type, status, created_at, updated_at, error_text
       FROM tasks ORDER BY created_at DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit], |row| {
      Ok(TaskSummary {
        id: row.get(0)?,
        task_type: row.get(1)?,
        status: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        error_text: row.get(5)?,
      })
    })?;

    let mut tasks = Vec::new();
    for row in rows {
      tasks.push(row?);
    }

    Ok(tasks)
  }

  pub fn list_download_batches(&self, page: u32, page_size: u32) -> Result<DownloadBatchListResult, AppError> {
    let conn = self.open()?;
    let safe_page = page.max(1);
    let offset = safe_page.saturating_sub(1).saturating_mul(page_size);
    let total: u32 = conn.query_row("SELECT COUNT(1) FROM download_batches", [], |row| row.get(0))?;
    let mut stmt = conn.prepare(
      "SELECT id, platform, total_count, success_count, failed_count, created_at, updated_at, completed_at
       FROM download_batches
       ORDER BY created_at DESC
       LIMIT ?1 OFFSET ?2",
    )?;

    let rows = stmt.query_map(params![page_size, offset], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, u32>(2)?,
        row.get::<_, String>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, Option<String>>(7)?,
      ))
    })?;

    let mut raw_batches = Vec::new();
    let mut batch_ids = HashSet::new();
    for row in rows {
      let row = row?;
      batch_ids.insert(row.0.clone());
      raw_batches.push(row);
    }

    let live_stats = self.collect_batch_task_stats(&conn, &batch_ids)?;
    let mut batches = Vec::with_capacity(raw_batches.len());
    for (id, platform, total_count, created_at, updated_at, completed_at) in raw_batches {
      let stats = live_stats.get(&id).copied().unwrap_or_default();
      let effective_total = total_count.max(stats.total_seen);
      batches.push(DownloadBatchSummary {
        id: id.clone(),
        platform,
        status: derive_batch_status(effective_total, stats),
        total_count: effective_total,
        success_count: stats.success_count,
        failed_count: stats.failed_count,
        running_count: stats.running_count,
        created_at,
        updated_at,
        completed_at,
      });
    }

    Ok(DownloadBatchListResult {
      items: batches,
      total,
      page: safe_page,
      page_size,
    })
  }

  pub fn get_download_batch_detail(&self, batch_id: &str) -> Result<Option<DownloadBatchDetail>, AppError> {
    let conn = self.open()?;
    let batch = conn
      .query_row(
        "SELECT id, platform, total_count, success_count, failed_count, created_at, updated_at, completed_at
         FROM download_batches
         WHERE id = ?1",
        params![batch_id],
        |row| {
          Ok(DownloadBatchDetail {
            id: row.get(0)?,
            platform: row.get(1)?,
            status: "queued".to_string(),
            total_count: row.get(2)?,
            success_count: 0,
            failed_count: 0,
            running_count: 0,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            completed_at: row.get(7)?,
            tasks: Vec::new(),
          })
        },
      )
      .optional()?;

    let Some(mut batch) = batch else {
      return Ok(None);
    };

    batch.tasks = self.list_tasks_for_batch(&conn, batch_id)?;
    let stats = summarize_batch_task_items(&batch.tasks, batch.total_count);
    let effective_total = batch.total_count.max(stats.total_seen);
    batch.total_count = effective_total;
    batch.success_count = stats.success_count;
    batch.failed_count = stats.failed_count;
    batch.running_count = stats.running_count;
    batch.status = derive_batch_status(effective_total, stats);
    Ok(Some(batch))
  }

  pub fn get_task_batch_details(&self, task_ids: &[String]) -> Result<Vec<TaskDetail>, AppError> {
    let conn = self.open()?;
    let mut details = Vec::new();

    for task_id in task_ids {
      let task = conn
        .query_row(
          "SELECT id, task_type, status, created_at, updated_at, error_text
           FROM tasks WHERE id = ?1",
          params![task_id],
          |row| {
            Ok(TaskDetail {
              id: row.get(0)?,
              task_type: row.get(1)?,
              status: row.get(2)?,
              created_at: row.get(3)?,
              updated_at: row.get(4)?,
              error_text: row.get(5)?,
              logs: Vec::new(),
            })
          },
        )
        .optional()?;

      let Some(mut task) = task else {
        continue;
      };

      let mut log_stmt = conn.prepare(
        "SELECT task_id, level, message, ts
         FROM task_logs
         WHERE task_id = ?1
         ORDER BY ts ASC",
      )?;

      let rows = log_stmt.query_map(params![task_id], |row| {
        Ok(TaskLogEntry {
          task_id: row.get(0)?,
          level: row.get(1)?,
          message: row.get(2)?,
          ts: row.get(3)?,
        })
      })?;

      for row in rows {
        task.logs.push(row?);
      }

      details.push(task);
    }

    Ok(details)
  }

  pub fn get_task_detail(&self, task_id: &str) -> Result<Option<TaskRecordDetail>, AppError> {
    let conn = self.open()?;
    let task = conn
      .query_row(
        "SELECT id, task_type, status, created_at, updated_at, error_text, input_json, output_json
         FROM tasks WHERE id = ?1",
        params![task_id],
        |row| {
          let input_json: String = row.get(6)?;
          let output_json: Option<String> = row.get(7)?;
          Ok(TaskRecordDetail {
            id: row.get(0)?,
            task_type: row.get(1)?,
            status: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
            error_text: row.get(5)?,
            input: parse_optional_json(Some(input_json)),
            output: parse_optional_json(output_json),
            logs: Vec::new(),
          })
        },
      )
      .optional()?;

    let Some(mut task) = task else {
      return Ok(None);
    };

    let mut log_stmt = conn.prepare(
      "SELECT task_id, level, message, ts
       FROM task_logs
       WHERE task_id = ?1
       ORDER BY ts ASC",
    )?;

    let rows = log_stmt.query_map(params![task_id], |row| {
      Ok(TaskLogEntry {
        task_id: row.get(0)?,
        level: row.get(1)?,
        message: row.get(2)?,
        ts: row.get(3)?,
      })
    })?;

    for row in rows {
      task.logs.push(row?);
    }

    Ok(Some(task))
  }

  pub fn get_task_status(&self, task_id: &str) -> Result<Option<String>, AppError> {
    let conn = self.open()?;
    conn
      .query_row(
        "SELECT status FROM tasks WHERE id = ?1",
        params![task_id],
        |row| row.get::<_, String>(0),
      )
      .optional()
      .map_err(AppError::from)
  }

  pub fn list_retryable_download_task_ids_for_batch(&self, batch_id: &str) -> Result<Vec<String>, AppError> {
    let conn = self.open()?;
    let mut stmt = conn.prepare(
      "SELECT id, task_type, status, input_json
       FROM tasks
       ORDER BY created_at ASC",
    )?;

    let rows = stmt.query_map([], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
      ))
    })?;

    let mut task_ids = Vec::new();
    for row in rows {
      let (task_id, task_type, status, input_json) = row?;
      if task_type != "video.download" || status != "queued" {
        continue;
      }

      let input = parse_optional_json(Some(input_json));
      if read_json_string_field(input.as_ref(), "batchId").as_deref() != Some(batch_id) {
        continue;
      }

      task_ids.push(task_id);
    }

    Ok(task_ids)
  }

  pub fn reset_download_batch_for_retry(&self, batch_id: &str) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "UPDATE download_batches
       SET success_count = 0,
           failed_count = 0,
           completion_handled = 0,
           completed_at = NULL,
           updated_at = ?2
       WHERE id = ?1",
      params![batch_id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn get_meta(&self, key: &str) -> Result<Option<String>, AppError> {
    let conn = self.open()?;
    let value = conn
      .query_row(
        "SELECT value FROM app_meta WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
      )
      .optional()?;
    Ok(value)
  }

  pub fn set_meta(&self, key: &str, value: &str) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "INSERT INTO app_meta (key, value, updated_at)
       VALUES (?1, ?2, ?3)
       ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
      params![key, value, Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn list_recording_accounts(&self) -> Result<Vec<RecordingAccountItem>, AppError> {
    let conn = self.open()?;
    let mut stmt = conn.prepare(
      "SELECT id, platform, account_input, account_name, account_uid, account_avatar_url,
              account_room_id, account_web_rid, account_sec_user_id, account_unique_id,
              auto_start, retry_on_disconnect, split_recording, save_snapshot,
              enabled, status, last_checked_at, last_recorded_at, last_error, created_at, updated_at
       FROM recording_accounts
       ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map([], map_recording_account_row)?;
    let mut items = Vec::new();
    for row in rows {
      items.push(row?);
    }
    Ok(items)
  }

  pub fn list_recording_account_logs(&self, limit: u32) -> Result<Vec<RecordingAccountLogEntry>, AppError> {
    let conn = self.open()?;
    let mut stmt = conn.prepare(
      "SELECT account_id, level, message, ts
       FROM recording_account_logs
       ORDER BY ts DESC
       LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit], |row| {
      Ok(RecordingAccountLogEntry {
        account_id: row.get(0)?,
        level: row.get(1)?,
        message: row.get(2)?,
        ts: row.get(3)?,
      })
    })?;

    let mut items = Vec::new();
    for row in rows {
      items.push(row?);
    }
    Ok(items)
  }

  pub fn list_recording_logs_for_account(
    &self,
    account_id: &str,
    limit: u32,
  ) -> Result<Vec<RecordingAccountLogEntry>, AppError> {
    let conn = self.open()?;
    let mut stmt = conn.prepare(
      "SELECT account_id, level, message, ts
       FROM recording_account_logs
       WHERE account_id = ?1
       ORDER BY ts DESC
       LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![account_id, limit], |row| {
      Ok(RecordingAccountLogEntry {
        account_id: row.get(0)?,
        level: row.get(1)?,
        message: row.get(2)?,
        ts: row.get(3)?,
      })
    })?;

    let mut items = Vec::new();
    for row in rows {
      items.push(row?);
    }
    Ok(items)
  }

  pub fn insert_recording_account(
    &self,
    payload: &RecordingAccountCreatePayload,
  ) -> Result<RecordingAccountItem, AppError> {
    let conn = self.open()?;
    let duplicate: Option<String> = conn
      .query_row(
        "SELECT id
         FROM recording_accounts
         WHERE platform = ?1 AND lower(trim(account_input)) = lower(trim(?2))
         LIMIT 1",
        params![payload.platform.trim(), payload.account_input.trim()],
        |row| row.get(0),
      )
      .optional()?;

    if duplicate.is_some() {
      return Err(AppError::Validation("recording account already exists".to_string()));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
      "INSERT INTO recording_accounts (
         id, platform, account_input, account_name, account_uid, account_avatar_url,
         account_room_id, account_web_rid, account_sec_user_id, account_unique_id,
         auto_start, retry_on_disconnect, split_recording, save_snapshot,
         enabled, status, last_checked_at, last_recorded_at, last_error, created_at, updated_at
       ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 1, 'watching', NULL, NULL, NULL, ?15, ?15)",
      params![
        id,
        payload.platform.trim(),
        payload.account_input.trim(),
        payload.account_name.trim(),
        payload.account_uid.trim(),
        payload.account_avatar_url.as_deref(),
        payload.account_room_id.as_deref(),
        payload.account_web_rid.as_deref(),
        payload.account_sec_user_id.as_deref(),
        payload.account_unique_id.as_deref(),
        if payload.auto_start { 1 } else { 0 },
        if payload.retry_on_disconnect { 1 } else { 0 },
        if payload.split_recording { 1 } else { 0 },
        if payload.save_snapshot { 1 } else { 0 },
        now,
      ],
    )?;

    self.get_recording_account(&id)?.ok_or_else(|| AppError::Database("failed to insert recording account".to_string()))
  }

  pub fn get_recording_account(&self, account_id: &str) -> Result<Option<RecordingAccountItem>, AppError> {
    let conn = self.open()?;
    conn
      .query_row(
        "SELECT id, platform, account_input, account_name, account_uid, account_avatar_url,
                account_room_id, account_web_rid, account_sec_user_id, account_unique_id,
                auto_start, retry_on_disconnect, split_recording, save_snapshot,
                enabled, status, last_checked_at, last_recorded_at, last_error, created_at, updated_at
         FROM recording_accounts
         WHERE id = ?1",
        params![account_id],
        map_recording_account_row,
      )
      .optional()
      .map_err(AppError::from)
  }

  pub fn update_recording_account(
    &self,
    payload: &RecordingAccountUpdatePayload,
  ) -> Result<RecordingAccountItem, AppError> {
    let conn = self.open()?;
    let trimmed_account_id = payload.account_id.trim();
    let duplicate: Option<String> = conn
      .query_row(
        "SELECT id
         FROM recording_accounts
         WHERE platform = ?1
           AND lower(trim(account_input)) = lower(trim(?2))
           AND id != ?3
         LIMIT 1",
        params![payload.platform.trim(), payload.account_input.trim(), trimmed_account_id],
        |row| row.get(0),
      )
      .optional()?;

    if duplicate.is_some() {
      return Err(AppError::Validation("recording account already exists".to_string()));
    }

    let now = Utc::now().to_rfc3339();
    conn.execute(
      "UPDATE recording_accounts
       SET platform = ?2,
           account_input = ?3,
           account_name = ?4,
           account_uid = ?5,
           account_avatar_url = ?6,
           account_room_id = ?7,
           account_web_rid = ?8,
           account_sec_user_id = ?9,
           account_unique_id = ?10,
           auto_start = ?11,
           retry_on_disconnect = ?12,
           split_recording = ?13,
           save_snapshot = ?14,
           updated_at = ?15
       WHERE id = ?1",
      params![
        trimmed_account_id,
        payload.platform.trim(),
        payload.account_input.trim(),
        payload.account_name.trim(),
        payload.account_uid.trim(),
        payload.account_avatar_url.as_deref(),
        payload.account_room_id.as_deref(),
        payload.account_web_rid.as_deref(),
        payload.account_sec_user_id.as_deref(),
        payload.account_unique_id.as_deref(),
        if payload.auto_start { 1 } else { 0 },
        if payload.retry_on_disconnect { 1 } else { 0 },
        if payload.split_recording { 1 } else { 0 },
        if payload.save_snapshot { 1 } else { 0 },
        now,
      ],
    )?;

    self
      .get_recording_account(trimmed_account_id)?
      .ok_or_else(|| AppError::Database("failed to update recording account".to_string()))
  }

  pub fn set_recording_account_enabled(&self, account_id: &str, enabled: bool) -> Result<(), AppError> {
    let conn = self.open()?;
    let next_status = if enabled { "watching" } else { "idle" };
    conn.execute(
      "UPDATE recording_accounts
       SET enabled = ?2, status = ?3, updated_at = ?4
       WHERE id = ?1",
      params![account_id, if enabled { 1 } else { 0 }, next_status, Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn delete_recording_account(&self, account_id: &str) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute("DELETE FROM recording_account_logs WHERE account_id = ?1", params![account_id])?;
    conn.execute("DELETE FROM recording_accounts WHERE id = ?1", params![account_id])?;
    Ok(())
  }

  pub fn append_recording_account_log(&self, account_id: &str, level: &str, message: &str) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "INSERT INTO recording_account_logs (id, account_id, level, message, ts)
       VALUES (?1, ?2, ?3, ?4, ?5)",
      params![Uuid::new_v4().to_string(), account_id, level, message, Utc::now().to_rfc3339()],
    )?;
    Ok(())
  }

  pub fn update_recording_account_check_result(
    &self,
    account_id: &str,
    status: &str,
    account_room_id: Option<&str>,
    account_web_rid: Option<&str>,
    checked_at: &str,
    error_message: Option<&str>,
  ) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "UPDATE recording_accounts
       SET status = ?2,
           account_room_id = COALESCE(?3, account_room_id),
           account_web_rid = COALESCE(?4, account_web_rid),
           last_checked_at = ?5,
           last_error = ?6,
           updated_at = ?5
       WHERE id = ?1",
      params![account_id, status, account_room_id, account_web_rid, checked_at, error_message],
    )?;
    Ok(())
  }

  pub fn list_enabled_recording_accounts(&self) -> Result<Vec<RecordingAccountItem>, AppError> {
    let conn = self.open()?;
    let mut stmt = conn.prepare(
      "SELECT id, platform, account_input, account_name, account_uid, account_avatar_url,
              account_room_id, account_web_rid, account_sec_user_id, account_unique_id,
              auto_start, retry_on_disconnect, split_recording, save_snapshot,
              enabled, status, last_checked_at, last_recorded_at, last_error, created_at, updated_at
       FROM recording_accounts
       WHERE enabled = 1
       ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map([], map_recording_account_row)?;
    let mut items = Vec::new();
    for row in rows {
      items.push(row?);
    }
    Ok(items)
  }

  pub fn list_recording_runs(&self, limit: u32) -> Result<Vec<RecordingRunItem>, AppError> {
    let conn = self.open()?;
    let mut stmt = conn.prepare(
      "SELECT id, status, input_json, output_json, error_text, created_at, updated_at
       FROM tasks
       WHERE task_type = 'recording.live'
       ORDER BY created_at DESC
       LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?,
        row.get::<_, Option<String>>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, String>(6)?,
      ))
    })?;

    let mut items = Vec::new();
    for row in rows {
      let (id, status, input_json, output_json, error_text, created_at, updated_at) = row?;
      let input = parse_optional_json(Some(input_json));
      let output = parse_optional_json(output_json);
      items.push(RecordingRunItem {
        id,
        account_id: read_json_string_field(input.as_ref(), "accountId").unwrap_or_default(),
        platform: read_json_string_field(input.as_ref(), "platform").unwrap_or_default(),
        account_name: read_json_string_field(input.as_ref(), "accountName").unwrap_or_default(),
        status,
        created_at,
        updated_at,
        error_text,
        output_path: read_json_string_field(output.as_ref(), "outputPath"),
      });
    }

    Ok(items)
  }

  pub fn list_active_recording_runs(&self) -> Result<Vec<RecordingRunItem>, AppError> {
    let conn = self.open()?;
    let mut stmt = conn.prepare(
      "SELECT id, status, input_json, output_json, error_text, created_at, updated_at
       FROM tasks
       WHERE task_type = 'recording.live' AND status = 'running'
       ORDER BY created_at ASC",
    )?;

    let rows = stmt.query_map([], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?,
        row.get::<_, Option<String>>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, String>(6)?,
      ))
    })?;

    let mut items = Vec::new();
    for row in rows {
      let (id, status, input_json, output_json, error_text, created_at, updated_at) = row?;
      let input = parse_optional_json(Some(input_json));
      let output = parse_optional_json(output_json);
      items.push(RecordingRunItem {
        id,
        account_id: read_json_string_field(input.as_ref(), "accountId").unwrap_or_default(),
        platform: read_json_string_field(input.as_ref(), "platform").unwrap_or_default(),
        account_name: read_json_string_field(input.as_ref(), "accountName").unwrap_or_default(),
        status,
        created_at,
        updated_at,
        error_text,
        output_path: read_json_string_field(output.as_ref(), "outputPath"),
      });
    }

    Ok(items)
  }

  pub fn mark_stale_recording_runs_failed(&self, reason: &str) -> Result<Vec<RecordingRunItem>, AppError> {
    let active = self.list_active_recording_runs()?;
    if active.is_empty() {
      return Ok(Vec::new());
    }

    let conn = self.open()?;
    let now = Utc::now().to_rfc3339();
    for item in &active {
      conn.execute(
        "UPDATE tasks SET status = 'failed', error_text = ?2, updated_at = ?3 WHERE id = ?1",
        params![item.id, reason, now],
      )?;
      conn.execute(
        "INSERT INTO task_logs (id, task_id, level, message, ts) VALUES (?1, ?2, 'warning', ?3, ?4)",
        params![Uuid::new_v4().to_string(), item.id, reason, now],
      )?;
    }

    Ok(active)
  }

  pub fn mark_recording_account_started(&self, account_id: &str, started_at: &str) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "UPDATE recording_accounts
       SET status = 'recording',
           last_checked_at = ?2,
           last_error = NULL,
           updated_at = ?2
       WHERE id = ?1",
      params![account_id, started_at],
    )?;
    Ok(())
  }

  pub fn mark_recording_account_finished(
    &self,
    account_id: &str,
    finished_at: &str,
    enabled: bool,
    last_error: Option<&str>,
  ) -> Result<(), AppError> {
    let conn = self.open()?;
    conn.execute(
      "UPDATE recording_accounts
       SET status = ?2,
           last_checked_at = ?3,
           last_recorded_at = ?3,
           last_error = ?4,
           updated_at = ?3
       WHERE id = ?1",
      params![account_id, if enabled { "watching" } else { "idle" }, finished_at, last_error],
    )?;
    Ok(())
  }

  pub fn is_recording_account_enabled(&self, account_id: &str) -> Result<bool, AppError> {
    let conn = self.open()?;
    let enabled = conn
      .query_row(
        "SELECT enabled FROM recording_accounts WHERE id = ?1",
        params![account_id],
        |row| row.get::<_, i64>(0),
      )
      .optional()?
      .unwrap_or(0);
    Ok(enabled != 0)
  }

  fn open(&self) -> Result<Connection, AppError> {
    let conn = Connection::open(&self.path)?;
    conn.busy_timeout(Duration::from_secs(30))?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    Ok(conn)
  }

  fn collect_batch_task_stats(
    &self,
    conn: &Connection,
    batch_ids: &HashSet<String>,
  ) -> Result<HashMap<String, BatchTaskStats>, AppError> {
    if batch_ids.is_empty() {
      return Ok(HashMap::new());
    }

    let mut stmt = conn.prepare(
      "SELECT status, input_json
       FROM tasks
       WHERE task_type = 'video.download'
       ORDER BY created_at ASC",
    )?;

    let rows = stmt.query_map([], |row| {
      Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut stats_map = HashMap::new();
    for row in rows {
      let (status, input_json) = row?;
      let parsed_input = parse_optional_json(Some(input_json));
      let Some(batch_id) = read_json_string_field(parsed_input.as_ref(), "batchId") else {
        continue;
      };

      if !batch_ids.contains(&batch_id) {
        continue;
      }

      let entry = stats_map.entry(batch_id).or_insert_with(BatchTaskStats::default);
      entry.total_seen += 1;
      apply_task_status(entry, &status);
    }

    Ok(stats_map)
  }

  fn list_tasks_for_batch(&self, conn: &Connection, batch_id: &str) -> Result<Vec<DownloadBatchTaskItem>, AppError> {
    let mut stmt = conn.prepare(
      "SELECT id, task_type, status, input_json, output_json, created_at, updated_at, error_text
       FROM tasks
       ORDER BY created_at ASC",
    )?;

    let rows = stmt.query_map([], |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, Option<String>>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, String>(6)?,
        row.get::<_, Option<String>>(7)?,
      ))
    })?;

    let mut tasks = Vec::new();
    for row in rows {
      let (id, task_type, status, input_json, output_json, created_at, updated_at, error_text) = row?;
      let parsed_input = parse_optional_json(Some(input_json));
      let parsed_output = parse_optional_json(output_json);
      if read_json_string_field(parsed_input.as_ref(), "batchId").as_deref() != Some(batch_id) {
        continue;
      }

      tasks.push(DownloadBatchTaskItem {
        id,
        task_type,
        status,
        source_url: read_json_string_field(parsed_input.as_ref(), "sourceUrl"),
        author_name: read_json_string_field(parsed_output.as_ref(), "authorName"),
        author_uid: read_json_string_field(parsed_output.as_ref(), "authorUid"),
        created_at,
        updated_at,
        error_text,
      });
    }

    Ok(tasks)
  }
}

fn map_recording_account_row(row: &rusqlite::Row<'_>) -> Result<RecordingAccountItem, rusqlite::Error> {
  Ok(RecordingAccountItem {
    id: row.get(0)?,
    platform: row.get(1)?,
    account_input: row.get(2)?,
    account_name: row.get(3)?,
    account_uid: row.get(4)?,
    account_avatar_url: row.get(5)?,
    account_room_id: row.get(6)?,
    account_web_rid: row.get(7)?,
    account_sec_user_id: row.get(8)?,
    account_unique_id: row.get(9)?,
    auto_start: row.get::<_, i64>(10)? != 0,
    retry_on_disconnect: row.get::<_, i64>(11)? != 0,
    split_recording: row.get::<_, i64>(12)? != 0,
    save_snapshot: row.get::<_, i64>(13)? != 0,
    enabled: row.get::<_, i64>(14)? != 0,
    status: row.get(15)?,
    last_checked_at: row.get(16)?,
    last_recorded_at: row.get(17)?,
    last_error: row.get(18)?,
    created_at: row.get(19)?,
    updated_at: row.get(20)?,
  })
}

fn parse_optional_json(raw: Option<String>) -> Option<Value> {
  raw.and_then(|value| serde_json::from_str::<Value>(&value).ok())
}

fn read_json_string_field(value: Option<&Value>, key: &str) -> Option<String> {
  value
    .and_then(|item| item.get(key))
    .and_then(|field| field.as_str())
    .map(|field| field.to_string())
}

fn summarize_batch_task_items(items: &[DownloadBatchTaskItem], total_count: u32) -> BatchTaskStats {
  let mut stats = BatchTaskStats {
    total_seen: items.len() as u32,
    ..BatchTaskStats::default()
  };

  for item in items {
    apply_task_status(&mut stats, &item.status);
  }

  if stats.total_seen == 0 {
    stats.total_seen = total_count;
  }

  stats
}

fn apply_task_status(stats: &mut BatchTaskStats, status: &str) {
  match status {
    "success" => stats.success_count += 1,
    "failed" => stats.failed_count += 1,
    "running" => stats.running_count += 1,
    _ => stats.queued_count += 1,
  }
}

fn derive_batch_status(total_count: u32, stats: BatchTaskStats) -> String {
  if total_count == 0 || stats.queued_count == total_count {
    return "queued".to_string();
  }

  if stats.running_count > 0 {
    return "running".to_string();
  }

  if stats.success_count == total_count {
    return "success".to_string();
  }

  if stats.failed_count == total_count {
    return "failed".to_string();
  }

  if stats.success_count + stats.failed_count >= total_count {
    if stats.success_count > 0 && stats.failed_count > 0 {
      return "partial".to_string();
    }
    if stats.success_count == total_count {
      return "success".to_string();
    }
    if stats.failed_count == total_count {
      return "failed".to_string();
    }
  }

  if stats.success_count > 0 && stats.failed_count > 0 {
    return "partial".to_string();
  }

  "running".to_string()
}
