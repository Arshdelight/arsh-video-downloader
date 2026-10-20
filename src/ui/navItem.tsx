import { type ReactNode, type MouseEvent } from 'react'
import { cn } from '../lib/utils'

interface NavItemProps {
  children: ReactNode
  className?: string
  /** Optional icon shown before the label */
  icon?: ReactNode
  /** Active (highlighted) state */
  active?: boolean
  onClick?: (e: MouseEvent<HTMLButtonElement>) => void
  title?: string
}

/**
 * Top navigation item, typically used as a child of TitleBar.
 * Not bound to any router: the caller computes `active` and handles navigation.
 */
export function NavItem({ children, className, icon, active = false, onClick, title }: NavItemProps) {
  return (
    <button
      onClick={onClick}
      aria-current={active ? 'page' : undefined}
      title={title}
      className={cn(
        'flex items-center gap-1.5 px-3 text-xs font-medium border-r border-line/40 transition-colors cursor-pointer',
        active
          ? 'bg-hover text-main'
          : 'text-muted hover:text-main hover:bg-hover/50',
        className,
      )}
    >
      {icon && <span className="size-3.5 flex items-center justify-center">{icon}</span>}
      {children}
    </button>
  )
}
