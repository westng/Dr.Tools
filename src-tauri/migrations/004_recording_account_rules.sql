ALTER TABLE recording_accounts ADD COLUMN auto_start INTEGER NOT NULL DEFAULT 1;
ALTER TABLE recording_accounts ADD COLUMN retry_on_disconnect INTEGER NOT NULL DEFAULT 1;
ALTER TABLE recording_accounts ADD COLUMN split_recording INTEGER NOT NULL DEFAULT 0;
ALTER TABLE recording_accounts ADD COLUMN save_snapshot INTEGER NOT NULL DEFAULT 0;
