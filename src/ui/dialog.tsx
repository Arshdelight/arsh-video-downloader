import { useEffect, useCallback, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { cn } from '../lib/utils'

interface DialogProps {
  open: boolean
  onClose: () => void
  children: ReactNode
  /** Whether clicking the overlay closes the dialog */
  closeOnOverlay?: boolean
  /** Whether to close on Escape key */
  closeOnEscape?: boolean
  className?: string
  overlayClassName?: string
}

export function Dialog({
  open,
  onClose,
  children,
  closeOnOverlay = true,
  closeOnEscape = true,
  className,
  overlayClassName,
}: DialogProps) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (closeOnEscape && e.key === 'Escape') onClose()
    },
    [onClose, closeOnEscape],
  )

  useEffect(() => {
    if (!open) return
    document.addEventListener('keydown', handleKeyDown)
    document.body.style.overflow = 'hidden'
    return () => {
      document.removeEventListener('keydown', handleKeyDown)
      document.body.style.overflow = ''
    }
  }, [open, handleKeyDown])

  if (!open) return null

  return createPortal(
    <div
      className={cn(
        'fixed inset-0 z-60 flex items-center justify-center',
        'animate-[fade-in_150ms_ease-out]',
        overlayClassName,
      )}
      style={{ background: 'rgba(0,0,0,0.45)' }}
      onClick={(e) => {
        if (closeOnOverlay && e.target === e.currentTarget) onClose()
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        className={cn(
          'bg-surface rounded-lg shadow-lg max-w-md w-full mx-4',
          'animate-[scale-in_150ms_ease-out]',
          className,
        )}
      >
        {children}
      </div>
    </div>,
    document.body,
  )
}
