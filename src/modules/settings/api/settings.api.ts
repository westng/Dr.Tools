import { invoke } from '@tauri-apps/api/core';
import type {
  AppSettings,
  AppSettingsPatch,
  ManagedEnvironmentStatus,
  TokenValidationPayload,
  TokenValidationResult,
  UpdateCheckResult,
} from '@/modules/settings/types';

export async function getSettings(): Promise<AppSettings> {
  return invoke<AppSettings>('settings_get');
}

export async function updateSettings(payload: AppSettingsPatch): Promise<AppSettings> {
  return invoke<AppSettings>('settings_update', { payload });
}

export async function checkUpdate(): Promise<UpdateCheckResult> {
  return invoke<UpdateCheckResult>('settings_check_update');
}

export async function selectExportDirectory(current?: string): Promise<string | null> {
  return invoke<string | null>('select_export_directory', { current });
}

export async function validateToken(payload: TokenValidationPayload): Promise<TokenValidationResult> {
  return invoke<TokenValidationResult>('token_validate', { payload });
}

export async function getEnvironmentStatus(): Promise<ManagedEnvironmentStatus> {
  return invoke<ManagedEnvironmentStatus>('environment_status');
}

export async function downloadEnvironment(): Promise<ManagedEnvironmentStatus> {
  return invoke<ManagedEnvironmentStatus>('environment_download');
}
