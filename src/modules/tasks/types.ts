export interface TaskSummary {
  id: string;
  taskType: string;
  status: string;
  createdAt: string;
  updatedAt: string;
  errorText?: string | null;
}

export interface TaskLogEntry {
  taskId: string;
  level: string;
  message: string;
  ts: string;
}

export interface TaskDetail extends TaskSummary {
  logs: TaskLogEntry[];
}

export interface TaskRecordDetail extends TaskDetail {
  input?: Record<string, unknown> | null;
  output?: Record<string, unknown> | null;
}

export interface TaskRunResponse {
  taskId: string;
  status: string;
  output?: Record<string, unknown> | null;
  error?: string | null;
}

export interface DownloadBatchSummary {
  id: string;
  platform: string;
  status: string;
  totalCount: number;
  successCount: number;
  failedCount: number;
  runningCount: number;
  createdAt: string;
  updatedAt: string;
  completedAt?: string | null;
}

export interface DownloadBatchListResult {
  items: DownloadBatchSummary[];
  total: number;
  page: number;
  pageSize: number;
}

export interface DownloadBatchTaskItem {
  id: string;
  taskType: string;
  status: string;
  sourceUrl?: string | null;
  authorName?: string | null;
  authorUid?: string | null;
  createdAt: string;
  updatedAt: string;
  errorText?: string | null;
}

export interface DownloadBatchDetail extends DownloadBatchSummary {
  tasks: DownloadBatchTaskItem[];
}

export interface SystemInfo {
  appName: string;
  appVersion: string;
  os: string;
  arch: string;
}
