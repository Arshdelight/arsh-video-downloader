import { useState, useEffect, useCallback, type ReactNode } from 'react'
import { Minus, Pin } from 'lucide-react'
import { cn } from '../lib/utils'
import { getWindow } from '../lib/tauri'

interface TitleBarProps {
  /** App brand name shown on the left */
  brand?: string
  /** Navigation items rendered next to the brand */
  children?: ReactNode
  /** Extra action buttons rendered before window controls */
  actions?: ReactNode
  /** Whether to show the always-on-top pin button */
  enablePin?: boolean
  /** Current pin state (controlled) */
  pinned?: boolean
  /** Called when pin is toggled */
  onPinToggle?: (pinned: boolean) => void
  /** Called when window close is requested */
  onCloseRequest?: () => void
}

export function TitleBar({
  brand = 'Arsh Tauri',
  children,
  actions,
  enablePin = false,
  pinned = false,
  onPinToggle,
  onCloseRequest,
}: TitleBarProps) {
  const win = getWindow()
  const [isMaximized, setIsMaximized] = useState(false)

  useEffect(() => {
    if (!win) return
    let disposed = false
    let unlisten: (() => void) | undefined
    win.isMaximized().then((v) => !disposed && setIsMaximized(v))
    win.onResized(async () => setIsMaximized(await win.isMaximized())).then((un) => {
      if (disposed) un()
      else unlisten = un
    })
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [win])

  const handleDoubleClick = useCallback(() => {
    getWindow()?.toggleMaximize()
  }, [])

  // Plain web environment (no Tauri runtime): hide window controls, keep the bar
  if (!win) {
    return (
      <div className="flex items-stretch shrink-0 h-7 bg-page select-none relative z-50 border-b border-line">
        <span className="flex items-center px-3 text-sm font-medium text-main/60 shrink-0 select-none">
          {brand}
        </span>
        {children}
        <div className="flex-1 min-w-0" />
        {actions && <div className="flex items-center gap-1 px-1.5">{actions}</div>}
      </div>
    )
  }

  const handleMinimize = () => void win.minimize()
  const handleMaximize = () => void win.toggleMaximize()
  const handleClose = () => {
    if (onCloseRequest) {
      onCloseRequest()
    } else {
      void win.close()
    }
  }

  return (
    <div
      className="flex items-stretch shrink-0 h-7 bg-page select-none relative z-50 border-b border-line"
      onDoubleClick={handleDoubleClick}
    >
      {/* Brand */}
      <span
        data-tauri-drag-region
        className="flex items-center px-3 text-sm font-medium text-main/60 shrink-0 select-none"
      >
        {brand}
      </span>

      {/* Navigation items */}
      {children}

      {/* Spacer */}
      <div data-tauri-drag-region className="flex-1 min-w-0" />

      {/* Actions */}
      {actions && (
        <div className="flex items-center gap-1 px-1.5">
          {actions}
        </div>
      )}

      {/* Window controls */}
      <div className="flex items-center gap-0.5 px-1.5">
        {enablePin && (
          <button
            onClick={() => onPinToggle?.(!pinned)}
            className={cn(
              'size-5 flex items-center justify-center transition-all duration-200 cursor-pointer',
              pinned ? 'text-main' : 'text-muted hover:text-main',
            )}
            title={pinned ? 'Unpin window' : 'Pin window'}
          >
            <Pin className={cn('size-4', pinned && 'fill-main')} />
          </button>
        )}
        {enablePin && <div className="w-px h-3.5 bg-line/60 mx-0.5" />}
        <button
          onClick={handleMinimize}
          className="size-5 flex items-center justify-center text-muted hover:text-main transition-all duration-200 cursor-pointer"
          title="Minimize"
          aria-label="Minimize"
        >
          <Minus className="size-4" />
        </button>
        <button
          onClick={handleMaximize}
          className="size-5 flex items-center justify-center text-muted hover:text-main transition-all duration-200 cursor-pointer"
          title={isMaximized ? 'Restore' : 'Maximize'}
          aria-label={isMaximized ? 'Restore' : 'Maximize'}
        >
          {isMaximized ? (
            <svg width="14" height="14" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.3">
              <rect x="3" y="3" width="7" height="7" rx="1" />
              <path d="M3 5H2V2a1 1 0 0 1 1-1h5v1" />
            </svg>
          ) : (
            <svg width="14" height="14" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.3">
              <rect x="1.5" y="1.5" width="9" height="9" rx="1.2" />
            </svg>
          )}
        </button>
        <button
          onClick={handleClose}
          className="size-5 flex items-center justify-center text-muted hover:text-main transition-all duration-200 cursor-pointer"
          title="Close"
          aria-label="Close"
        >
          <svg width="14" height="14" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <path d="M2 2l8 8M10 2l-8 8" />
          </svg>
        </button>
      </div>
    </div>
  )
}
