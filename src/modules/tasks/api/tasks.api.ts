import { invoke } from '@tauri-apps/api/core';
import type {
  DownloadBatchDetail,
  DownloadBatchListResult,
  TaskDetail,
  TaskRecordDetail,
  TaskRunResponse,
  TaskSummary,
} from '@/modules/tasks/types';

export async function runTask(taskType: string, payload: Record<string, unknown>): Promise<TaskRunResponse> {
  return invoke<TaskRunResponse>('task_run', {
    payload: {
      taskType,
      payload,
    },
  });
}

export async function listTasks(limit = 50): Promise<TaskSummary[]> {
  return invoke<TaskSummary[]>('task_list', { limit });
}

export async function listDownloadBatches(page = 1, pageSize = 10): Promise<DownloadBatchListResult> {
  return invoke<DownloadBatchListResult>('download_batch_list', { page, pageSize });
}

export async function getDownloadBatchDetail(batchId: string): Promise<DownloadBatchDetail> {
  return invoke<DownloadBatchDetail>('download_batch_detail', { batchId });
}

export async function getTaskBatchDetails(taskIds: string[]): Promise<TaskDetail[]> {
  return invoke<TaskDetail[]>('task_batch_details', { taskIds });
}

export async function getTaskDetail(taskId: string): Promise<TaskRecordDetail> {
  return invoke<TaskRecordDetail>('task_detail', { taskId });
}

export async function retryDownloadBatch(batchId: string): Promise<void> {
  return invoke('download_batch_retry', { batchId });
}

export async function openDownloadBatchDetailWindow(batchId: string, title: string): Promise<void> {
  return invoke('open_download_batch_detail_window', { batchId, title });
}

export async function openTaskDetailWindow(taskId: string, title: string): Promise<void> {
  return invoke('open_task_detail_window', { taskId, title });
}
