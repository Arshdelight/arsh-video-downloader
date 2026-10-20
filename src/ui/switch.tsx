import { cn } from '../lib/utils'

interface SwitchProps {
  checked: boolean
  onChange: (checked: boolean) => void
  /** Label text shown next to the switch */
  label?: string
  /** Position of the label */
  labelPosition?: 'left' | 'right'
  disabled?: boolean
  className?: string
}

export function Switch({
  checked,
  onChange,
  label,
  labelPosition = 'right',
  disabled = false,
  className,
}: SwitchProps) {
  const handleClick = () => {
    if (!disabled) onChange(!checked)
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      handleClick()
    }
  }

  const switchEl = (
    <button
      onClick={handleClick}
      disabled={disabled}
      className={cn(
        'relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full',
        'border-2 border-transparent transition-colors duration-200 ease-in-out',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/20',
        'disabled:opacity-40 disabled:cursor-not-allowed',
        checked ? 'bg-accent' : 'bg-hover',
      )}
      role="switch"
      aria-checked={checked}
      onKeyDown={handleKeyDown}
    >
      <span
        className={cn(
          'pointer-events-none block size-3.5 rounded-full bg-surface shadow-sm',
          'ring-0 transition-transform duration-200 ease-in-out',
          checked ? 'translate-x-[18px]' : 'translate-x-0.5',
        )}
      />
    </button>
  )

  if (!label) return switchEl

  return (
    <label
      className={cn(
        'inline-flex items-center gap-2 select-none cursor-pointer',
        disabled && 'opacity-40 cursor-not-allowed',
        className,
      )}
    >
      {labelPosition === 'left' && (
        <span className="text-sm text-main">{label}</span>
      )}
      {switchEl}
      {labelPosition === 'right' && (
        <span className="text-sm text-main">{label}</span>
      )}
    </label>
  )
}
