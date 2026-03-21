export function toErrorMessage(error: unknown, fallback = 'Unknown error'): string {
  if (typeof error === 'string' && error.trim()) {
    return error;
  }

  if (error && typeof error === 'object') {
    if ('message' in error && typeof error.message === 'string' && error.message.trim()) {
      return error.message;
    }

    if ('error' in error && typeof error.error === 'string' && error.error.trim()) {
      return error.error;
    }
  }

  return fallback;
}
