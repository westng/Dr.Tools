export type RecordingPlatform = 'douyin' | 'tiktok';

export type RecordingAccountStatus = 'idle' | 'watching' | 'live' | 'recording' | 'error';

export interface RecordingAccount {
  id: string;
  platform: RecordingPlatform;
  accountInput: string;
  accountName: string;
  accountUid: string;
  accountAvatarUrl?: string | null;
  accountRoomId?: string | null;
  accountWebRid?: string | null;
  accountSecUserId?: string | null;
  accountUniqueId?: string | null;
  autoStart: boolean;
  retryOnDisconnect: boolean;
  splitRecording: boolean;
  saveSnapshot: boolean;
  enabled: boolean;
  status: RecordingAccountStatus;
  lastCheckedAt: string | null;
  lastRecordedAt: string | null;
  lastError: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface RecordingAccountDraft {
  platform: RecordingPlatform;
  accountInput: string;
  accountName: string;
  accountUid: string;
  accountAvatarUrl?: string | null;
  accountRoomId?: string | null;
  accountWebRid?: string | null;
  accountSecUserId?: string | null;
  accountUniqueId?: string | null;
  autoStart: boolean;
  retryOnDisconnect: boolean;
  splitRecording: boolean;
  saveSnapshot: boolean;
}

export interface RecordingLogEntry {
  id: string;
  ts: string;
  level: 'info' | 'success' | 'warning' | 'error';
  message: string;
  accountId?: string;
}

export interface RecordingRunItem {
  id: string;
  accountId: string;
  platform: RecordingPlatform;
  accountName: string;
  status: 'queued' | 'running' | 'success' | 'failed';
  createdAt: string;
  updatedAt: string;
  errorText?: string | null;
  outputPath?: string | null;
}

export interface RecordingSnapshot {
  accounts: RecordingAccount[];
  logs: Array<Omit<RecordingLogEntry, 'id'>>;
  runs: RecordingRunItem[];
}
