export type DownloadPlatform = 'douyin' | 'tiktok' | 'twitter' | 'weibo';

export interface VideoDownloadSubmitPayload {
  platform: DownloadPlatform;
  urls: string[];
  downloadCover: boolean;
  downloadMusic: boolean;
  downloadDescription: boolean;
  downloadLyric: boolean;
}

export interface VideoDownloadSubmitResult {
  createdTaskIds: string[];
  acceptedCount: number;
  skippedCount: number;
  invalidUrls: string[];
}
