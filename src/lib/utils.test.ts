import { describe, expect, it } from 'vitest'
import { cn } from './utils'

describe('cn', () => {
  it('joins conditional classes', () => {
    expect(cn('flex', false && 'hidden', 'items-center')).toBe('flex items-center')
  })

  it('lets later tailwind classes win conflicts', () => {
    expect(cn('px-2', 'px-4')).toBe('px-4')
    expect(cn('text-sm text-main', 'text-lg')).toBe('text-main text-lg')
  })

  it('resolves conflicting class groups, not just identical classes', () => {
    expect(cn('p-2', 'py-4')).toBe('p-2 py-4')
  })
})
