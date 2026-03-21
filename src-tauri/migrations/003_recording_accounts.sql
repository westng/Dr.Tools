CREATE TABLE IF NOT EXISTS recording_accounts (
  id TEXT PRIMARY KEY,
  platform TEXT NOT NULL,
  account_input TEXT NOT NULL,
  account_name TEXT NOT NULL,
  account_uid TEXT NOT NULL,
  account_avatar_url TEXT,
  account_room_id TEXT,
  account_web_rid TEXT,
  account_sec_user_id TEXT,
  account_unique_id TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  status TEXT NOT NULL DEFAULT 'watching',
  last_checked_at TEXT,
  last_recorded_at TEXT,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS recording_account_logs (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  level TEXT NOT NULL,
  message TEXT NOT NULL,
  ts TEXT NOT NULL,
  FOREIGN KEY(account_id) REFERENCES recording_accounts(id)
);

CREATE INDEX IF NOT EXISTS idx_recording_accounts_created_at ON recording_accounts(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_recording_account_logs_account_id ON recording_account_logs(account_id);
CREATE INDEX IF NOT EXISTS idx_recording_account_logs_ts ON recording_account_logs(ts DESC);
