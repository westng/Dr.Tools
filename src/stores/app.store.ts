import { defineStore } from 'pinia';
import { fetchSystemInfo, pingPython } from '@/api/system.api';
import { toErrorMessage } from '@/lib/errors';
import type { SystemInfo } from '@/modules/tasks/types';

interface AppState {
  systemInfo: SystemInfo | null;
  pythonStatus: 'idle' | 'ok' | 'error';
  pythonMessage: string;
  loading: boolean;
}

export const useAppStore = defineStore('app', {
  state: (): AppState => ({
    systemInfo: null,
    pythonStatus: 'idle',
    pythonMessage: '',
    loading: false
  }),
  actions: {
    async bootstrap(): Promise<void> {
      this.loading = true;
      try {
        this.systemInfo = await fetchSystemInfo();
        const pong = await pingPython();
        this.pythonStatus = 'ok';
        this.pythonMessage = pong;
      } catch (error) {
        this.pythonStatus = 'error';
        this.pythonMessage = toErrorMessage(error);
      } finally {
        this.loading = false;
      }
    }
  }
});
