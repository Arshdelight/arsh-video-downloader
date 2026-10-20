import type { ReactNode } from 'react'
import { cn } from '../lib/utils'

interface AppLayoutProps {
  /** The TitleBar component */
  titleBar: ReactNode
  /** Main content area */
  children: ReactNode
  /** Optional sidebar rendered between title bar and content */
  sidebar?: ReactNode
  className?: string
}

/**
 * AppLayout provides the top-level shell for an application.
 * It stacks the title bar, optional sidebar, and main content vertically.
 * Height follows the parent (h-full): use 100vh on the root container (e.g. #root).
 */
export function AppLayout({ titleBar, children, sidebar, className }: AppLayoutProps) {
  return (
    <div className={cn('h-full flex flex-col overflow-hidden bg-page', className)}>
      {titleBar}
      <div className="flex-1 flex overflow-hidden">
        {sidebar && (
          <div className="shrink-0 border-r border-line">
            {sidebar}
          </div>
        )}
        <main className="flex-1 overflow-auto">
          {children}
        </main>
      </div>
    </div>
  )
}
