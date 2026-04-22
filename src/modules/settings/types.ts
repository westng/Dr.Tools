export type ThemeMode = 'auto' | 'light' | 'dark';
export type LiquidGlassStyle = 'transparent' | 'tinted';
export type LocaleCode = 'zh-CN' | 'en-US';
export type TokenPlatform = 'douyin' | 'tiktok';
export type TokenCheckStatus = 'not_configured' | 'unchecked' | 'checking' | 'valid' | 'invalid' | 'expired';

export interface AppSettings {
  themeMode: ThemeMode;
  liquidGlassStyle: LiquidGlassStyle;
  accentColor: string;
  locale: LocaleCode;
  exportDir: string;
  maxConcurrentDownloads: number;
  downloadNotificationsEnabled: boolean;
  douyinCookie: string;
  douyinCookieUpdatedAt?: string | null;
  douyinLastCheckedAt?: string | null;
  douyinLastCheckStatus: TokenCheckStatus;
  douyinLastCheckMessage?: string | null;
  tiktokCookie: string;
  tiktokCookieUpdatedAt?: string | null;
  tiktokLastCheckedAt?: string | null;
  tiktokLastCheckStatus: TokenCheckStatus;
  tiktokLastCheckMessage?: string | null;
  autoCheckUpdates: boolean;
  lastUpdateCheckAt?: string | null;
  lastUpdateStatus?: string | null;
}

export interface AppSettingsPatch {
  themeMode?: ThemeMode;
  liquidGlassStyle?: LiquidGlassStyle;
  accentColor?: string;
  locale?: LocaleCode;
  exportDir?: string;
  maxConcurrentDownloads?: number;
  downloadNotificationsEnabled?: boolean;
  douyinCookie?: string;
  tiktokCookie?: string;
  autoCheckUpdates?: boolean;
}

export interface UpdateCheckResult {
  checkedAt: string;
  status: string;
  message: string;
  currentVersion: string;
  latestVersion: string;
}

export interface TokenValidationPayload {
  platform: TokenPlatform;
  cookie: string;
}

export interface TokenValidationResult {
  platform: TokenPlatform;
  checkedAt: string;
  status: TokenCheckStatus;
  message: string;
}

export type ManagedEnvironmentState = 'missing' | 'ready' | 'invalid';

export interface ManagedEnvironmentStatus {
  pythonVersion: string;
  sourceLabel: string;
  sourceUrl: string;
  installDir: string;
  pythonBin?: string | null;
  installed: boolean;
  status: ManagedEnvironmentState;
  message: string;
}
