import { isTauri } from '@tauri-apps/api/core'
import { commands, type AppError, type AppInfo } from '../gen/bindings'

export type { AppError, AppInfo }

/** Transport interface between the Svelte shell and the Tauri desktop host. */
export interface DesktopClient {
  /** Reports whether the shell is running inside Tauri or the IPC test bridge. */
  isDesktop(): boolean
  /** Fetches static host metadata and the active `DIFFAGENT_HOME` directory. */
  getAppInfo(): Promise<AppInfo>
  /** Signals that the frontend shell finished mounting its lifecycle handlers. */
  markFrontendReady(): Promise<void>
}

const WEB_FALLBACK_INFO: AppInfo = {
  name: 'diffagent-web',
  version: '0.1.0',
  homeDir: '~',
}

function isDesktopRuntime(): boolean {
  if (typeof window === 'undefined') return false
  return isTauri() || Boolean((window as unknown as { isTauri?: boolean }).isTauri)
}

/** Formats an unknown IPC or runtime failure into a user-readable message. */
export function errorMessage(error: unknown): string {
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message)
  }
  return error instanceof Error ? error.message : String(error)
}

/** Production desktop transport with standalone browser preview fallback. */
export const desktopClient: DesktopClient = {
  isDesktop(): boolean {
    return isDesktopRuntime()
  },

  async getAppInfo(): Promise<AppInfo> {
    if (!isDesktopRuntime()) return WEB_FALLBACK_INFO
    return await commands.appInfo()
  },

  async markFrontendReady(): Promise<void> {
    if (!isDesktopRuntime()) return
    await commands.appFrontendReady()
  },
}
