CREATE TABLE IF NOT EXISTS download_batches (
  id TEXT PRIMARY KEY,
  platform TEXT NOT NULL,
  total_count INTEGER NOT NULL,
  success_count INTEGER NOT NULL DEFAULT 0,
  failed_count INTEGER NOT NULL DEFAULT 0,
  completion_handled INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_download_batches_created_at ON download_batches(created_at DESC);
