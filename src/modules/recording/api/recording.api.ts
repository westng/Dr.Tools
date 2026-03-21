import { invoke } from '@tauri-apps/api/core';
import type { RecordingAccount, RecordingAccountDraft, RecordingLogEntry, RecordingPlatform, RecordingSnapshot } from '@/modules/recording/types';

export async function openRecordingAccountCreateWindow(title: string): Promise<void> {
  return invoke('open_recording_account_create_window', { title });
}

export async function openRecordingAccountEditWindow(accountId: string, title: string): Promise<void> {
  return invoke('open_recording_account_edit_window', { accountId, title });
}

export async function openRecordingAccountLogsWindow(accountId: string, title: string): Promise<void> {
  return invoke('open_recording_account_logs_window', { accountId, title });
}

export async function resolveRecordingAccountProfile(
  platform: RecordingPlatform,
  sourceUrl: string,
): Promise<RecordingAccountDraft> {
  return invoke<RecordingAccountDraft>('resolve_recording_account_profile', { payload: { platform, sourceUrl } });
}

export async function getRecordingSnapshot(logLimit = 50, runLimit = 12): Promise<RecordingSnapshot> {
  return invoke<RecordingSnapshot>('recording_accounts_snapshot', { logLimit, runLimit });
}

export async function createRecordingAccount(payload: RecordingAccountDraft): Promise<RecordingAccount> {
  return invoke<RecordingAccount>('recording_account_create', { payload });
}

export async function getRecordingAccountDetail(accountId: string): Promise<RecordingAccount> {
  return invoke<RecordingAccount>('recording_account_detail', { accountId });
}

export async function updateRecordingAccount(accountId: string, payload: RecordingAccountDraft): Promise<RecordingAccount> {
  return invoke<RecordingAccount>('recording_account_update', {
    payload: {
      accountId,
      ...payload,
    },
  });
}

export async function setRecordingAccountEnabled(accountId: string, enabled: boolean): Promise<RecordingAccount> {
  return invoke<RecordingAccount>('recording_account_set_enabled', { accountId, enabled });
}

export async function deleteRecordingAccount(accountId: string): Promise<void> {
  return invoke('recording_account_delete', { accountId });
}

export async function checkRecordingAccounts(accountIds?: string[]): Promise<RecordingSnapshot> {
  return invoke<RecordingSnapshot>('recording_accounts_check', { accountIds });
}

export async function getRecordingAccountLogs(accountId: string, limit = 100): Promise<Array<Omit<RecordingLogEntry, 'id'>>> {
  return invoke<Array<Omit<RecordingLogEntry, 'id'>>>('recording_account_logs', { accountId, limit });
}
