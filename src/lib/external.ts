import { isTauri } from './ipc';

export async function openExternal(url: string) {
  if (!/^https?:\/\//.test(url)) return;
  if (isTauri) {
    try {
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(url);
      return;
    } catch { /* fall through */ }
  }
  window.open(url, '_blank', 'noopener');
}
