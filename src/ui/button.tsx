import { type ReactNode, type ButtonHTMLAttributes } from 'react'
import { cn } from '../lib/utils'
import { Loader2 } from 'lucide-react'

const variants = {
  default: 'bg-accent text-fg hover:opacity-80',
  secondary: 'bg-surface border border-line text-main hover:bg-hover',
  ghost: 'text-muted hover:text-main hover:bg-hover',
  destructive: 'bg-destructive text-white hover:opacity-80',
  link: 'text-accent underline-offset-4 hover:underline',
} as const

const sizes = {
  sm: 'px-2.5 py-1 text-xs rounded-md',
  md: 'px-4 py-1.5 text-sm rounded-lg',
  lg: 'px-5 py-2 text-sm rounded-lg',
} as const

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'className'> {
  /** Button visual style */
  variant?: keyof typeof variants
  /** Button size */
  size?: keyof typeof sizes
  /** Optional icon shown before the label */
  icon?: ReactNode
  /** Show loading spinner */
  loading?: boolean
  className?: string
  children?: ReactNode
}

export function Button({
  variant = 'default',
  size = 'md',
  icon,
  loading = false,
  disabled,
  className,
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      disabled={disabled || loading}
      className={cn(
        'inline-flex items-center justify-center gap-1.5 font-medium transition-all cursor-pointer select-none',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/20',
        'disabled:opacity-40 disabled:cursor-not-allowed',
        variants[variant],
        sizes[size],
        className,
      )}
      {...props}
    >
      {loading ? (
        <Loader2 className={cn('size-4 animate-spin', children && 'shrink-0')} />
      ) : icon ? (
        <span className="size-4 shrink-0 flex items-center justify-center">{icon}</span>
      ) : null}
      {children}
    </button>
  )
}
