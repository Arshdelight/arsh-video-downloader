/**
 * Native (Tauri) bridge detection.
 *
 * UI primitives must render correctly in plain web apps too. Anything that
 * needs the native window (title-bar controls, drag regions) checks this
 * first and degrades gracefully when it is absent.
 */
import { getCurrentWindow, type Window } from '@tauri-apps/api/window'

export const hasNativeBridge =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** The focused app window, or null when running as a plain web page. */
export function getWindow(): Window | null {
  if (!hasNativeBridge) return null
  try {
    return getCurrentWindow()
  } catch {
    return null
  }
}
