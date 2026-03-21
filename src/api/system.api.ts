import { invoke } from '@tauri-apps/api/core';
import type { SystemInfo } from '@/modules/tasks/types';

export async function fetchSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>('system_info');
}

export async function pingPython(): Promise<string> {
  return invoke<string>('python_ping');
}
