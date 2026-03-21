import { invoke } from '@tauri-apps/api/core';
import type { VideoDownloadSubmitPayload, VideoDownloadSubmitResult } from '@/modules/download/types';

export async function submitVideoDownload(payload: VideoDownloadSubmitPayload): Promise<VideoDownloadSubmitResult> {
  return invoke<VideoDownloadSubmitResult>('video_download_submit', { payload });
}
