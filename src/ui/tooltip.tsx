import { useState, useRef, useCallback, useLayoutEffect, useEffect, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { cn } from '../lib/utils'

type TooltipPosition = 'top' | 'bottom' | 'left' | 'right'

interface TooltipProps {
  children: ReactNode
  content: string
  position?: TooltipPosition
  /** Delay in ms before showing (default: 400) */
  showDelay?: number
  /** Make the trigger element fill full width */
  fullWidth?: boolean
  /** Temporarily disable tooltip */
  disabled?: boolean
}

const GAP = 6

export function Tooltip({ children, content, position = 'bottom', showDelay = 400, fullWidth = false, disabled = false }: TooltipProps) {
  const [open, setOpen] = useState(false)
  const triggerRef = useRef<HTMLDivElement>(null)
  const tooltipRef = useRef<HTMLDivElement>(null)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const clearTimer = () => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current)
      timerRef.current = null
    }
  }

  // Hiding when disabled is derived from `visible`; no setState inside the effect
  useEffect(() => {
    if (disabled) clearTimer()
  }, [disabled])
  const visible = open && !disabled

  const calcPosition = useCallback(() => {
    const trigger = triggerRef.current
    const tooltip = tooltipRef.current
    if (!trigger || !tooltip) return null

    const triggerRect = trigger.getBoundingClientRect()
    const { width: tw, height: th } = tooltip.getBoundingClientRect()
    const viewW = window.innerWidth
    const viewH = window.innerHeight

    let top = 0, left = 0, useTranslateX = true

    switch (position) {
      case 'top': {
        top = triggerRect.top - GAP - th
        left = triggerRect.left + triggerRect.width / 2
        if (top < 0) { top = triggerRect.bottom + GAP }
        break
      }
      case 'bottom': {
        top = triggerRect.bottom + GAP
        left = triggerRect.left + triggerRect.width / 2
        if (top + th > viewH) { top = triggerRect.top - GAP - th }
        break
      }
      case 'left': {
        top = triggerRect.top + triggerRect.height / 2
        left = triggerRect.left - GAP - tw
        useTranslateX = false
        if (left < 0) { left = triggerRect.right + GAP }
        break
      }
      case 'right': {
        top = triggerRect.top + triggerRect.height / 2
        left = triggerRect.right + GAP
        useTranslateX = false
        if (left + tw > viewW) { left = triggerRect.left - GAP - tw }
        break
      }
    }

    // Clamp
    if (useTranslateX) {
      const halfW = tw / 2
      if (left < halfW) left = halfW
      if (left > viewW - halfW) left = viewW - halfW
      return { position: 'fixed' as const, top, left, transform: 'translate(-50%, 0)' }
    } else {
      const halfH = th / 2
      if (top < halfH) top = halfH
      if (top > viewH - halfH) top = viewH - halfH
      return { position: 'fixed' as const, top, left, transform: 'translate(0, -50%)' }
    }
  }, [position])

  // After opening, use the measured dimensions for positioning before the browser paints: write directly to the DOM, avoiding setState cascades
  useLayoutEffect(() => {
    if (!visible) return
    const pos = calcPosition()
    if (pos && tooltipRef.current) Object.assign(tooltipRef.current.style, pos)
  }, [visible, calcPosition])

  const handleMouseEnter = () => {
    if (disabled) return
    clearTimer()
    timerRef.current = setTimeout(() => setOpen(true), showDelay)
  }

  const handleMouseLeave = () => {
    clearTimer()
    setOpen(false)
  }

  return (
    <>
      <div
        ref={triggerRef}
        className={cn('inline-flex', fullWidth && 'w-full')}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onFocus={handleMouseEnter}
        onBlur={handleMouseLeave}
      >
        {children}
      </div>
      {visible && createPortal(
        <div
          ref={tooltipRef}
          className={cn(
            'px-2 py-0.5 text-xs rounded-md',
            'bg-surface text-main text-center',
            'shadow-md border border-line whitespace-nowrap',
            'pointer-events-none select-none z-[100]',
          )}
        >
          {content}
        </div>,
        document.body,
      )}
    </>
  )
}
