import { cn } from '../lib/utils'
import { Loader2 } from 'lucide-react'

interface SpinnerProps {
  /** Spinner size */
  size?: 'sm' | 'md' | 'lg'
  /** Show label below the spinner */
  label?: string
  className?: string
}

const sizes = {
  sm: { icon: 'size-4', label: 'text-xs' },
  md: { icon: 'size-6', label: 'text-sm' },
  lg: { icon: 'size-8', label: 'text-sm' },
}

export function Spinner({ size = 'md', label, className }: SpinnerProps) {
  const s = sizes[size]

  return (
    <div className={cn('inline-flex flex-col items-center gap-2', className)}>
      <Loader2
        className={cn('animate-spin text-accent', s.icon)}
        role="status"
        aria-label={label || 'Loading'}
      />
      {label && (
        <span className={cn('text-muted', s.label)}>{label}</span>
      )}
    </div>
  )
}
