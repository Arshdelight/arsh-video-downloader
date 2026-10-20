import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import {
  CheckCircle2,
  Clock,
  FolderOpen,
  Link2,
  Loader2,
  Plus,
  X,
  XCircle,
} from 'lucide-react'
import { AppLayout, TitleBar, Button } from './ui'
import { cn } from './lib/utils'
import { hasNativeBridge } from './lib/tauri'
import { api, type Task } from './lib/api'
import { EVENT } from './lib/events'

const STATUS_META: Record<Task['status'], { label: string; className: string }> = {
  pending: { label: '等待中', className: 'text-dim' },
  downloading: { label: '下载中', className: 'text-main' },
  done: { label: '已完成', className: 'text-success' },
  canceled: { label: '已取消', className: 'text-dim' },
  error: { label: '失败', className: 'text-destructive' },
}

function StatusIcon({ status }: { status: Task['status'] }) {
  switch (status) {
    case 'downloading':
      return <Loader2 className="size-4 shrink-0 mt-0.5 animate-spin text-accent" />
    case 'done':
      return <CheckCircle2 className="size-4 shrink-0 mt-0.5 text-success" />
    case 'error':
      return <XCircle className="size-4 shrink-0 mt-0.5 text-destructive" />
    case 'canceled':
      return <XCircle className="size-4 shrink-0 mt-0.5 text-dim" />
    default:
      return <Clock className="size-4 shrink-0 mt-0.5 text-dim" />
  }
}

function TaskRow({ task, onCancel }: { task: Task; onCancel: (id: number) => void }) {
  const meta = STATUS_META[task.status]
  const active = task.status === 'downloading' || task.status === 'pending'

  return (
    <div className="rounded-lg border border-line bg-surface p-3">
      <div className="flex items-start gap-2.5">
        <StatusIcon status={task.status} />
        <div className="flex-1 min-w-0">
          <div className="text-sm text-main truncate" title={task.title}>
            {task.title}
          </div>
          <div className={cn('text-xs mt-0.5 truncate', meta.className)} title={task.error}>
            {meta.label}
            {task.status === 'downloading' && task.speed && ` · ${task.speed}`}
            {task.status === 'downloading' && ` · ${task.progress.toFixed(1)}%`}
            {task.status === 'error' && task.error && ` · ${task.error}`}
          </div>
        </div>
        {active && (
          <button
            onClick={() => onCancel(task.id)}
            className="size-6 shrink-0 flex items-center justify-center rounded-md text-muted hover:text-destructive hover:bg-hover transition-colors cursor-pointer"
            title="取消"
            aria-label="取消"
          >
            <X className="size-4" />
          </button>
        )}
      </div>
      {(task.status === 'downloading' || task.status === 'pending') && (
        <div className="mt-2 h-1.5 rounded-full bg-hover overflow-hidden">
          <div
            className="h-full rounded-full bg-accent transition-[width] duration-300"
            style={{ width: `${task.status === 'pending' ? 0 : task.progress}%` }}
          />
        </div>
      )}
    </div>
  )
}

export default function App() {
  const [url, setUrl] = useState('')
  const [dir, setDir] = useState('')
  const [tasks, setTasks] = useState<Task[]>([])

  useEffect(() => {
    if (!hasNativeBridge) return
    api.getDir().then(setDir).catch(() => {})
    api.listTasks().then(setTasks).catch(() => {})
    const un = listen<Task[]>(EVENT, (e) => setTasks(e.payload))
    return () => {
      un.then((f) => f())
    }
  }, [])

  const add = () => {
    const text = url.trim()
    if (!text) return
    api
      .addTask(text)
      .then(() => setUrl(''))
      .catch((e) => console.error('添加任务失败:', e))
  }

  const pickDir = () => {
    api
      .pickDir()
      .then((picked) => {
        if (picked) setDir(picked)
      })
      .catch((e) => console.error('选择目录失败:', e))
  }

  const doneCount = tasks.filter((t) => t.status === 'done').length

  return (
    <AppLayout titleBar={<TitleBar brand="ArshVideoDownloader" />}>
      <div className="h-full flex flex-col">
        {/* 顶部：粘贴链接 + 下载目录 */}
        <div className="shrink-0 border-b border-line px-4 py-3 space-y-2.5">
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Link2 className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-dim pointer-events-none" />
              <input
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && add()}
                placeholder="粘贴视频链接，支持一次粘贴多个（空格分隔）"
                className="w-full h-8 rounded-lg border border-line bg-surface pl-8 pr-3 text-sm text-main placeholder:text-dim focus:outline-none focus:ring-2 focus:ring-accent/20"
              />
            </div>
            <Button icon={<Plus className="size-4" />} onClick={add} disabled={!url.trim()}>
              添加下载
            </Button>
          </div>
          <div className="flex items-center gap-1.5 text-xs text-muted min-w-0">
            <FolderOpen className="size-3.5 shrink-0" />
            <span className="truncate" title={dir}>
              {dir || '下载目录未设置（默认为系统下载文件夹）'}
            </span>
            <button
              onClick={pickDir}
              className="shrink-0 text-accent hover:underline cursor-pointer"
            >
              更改
            </button>
          </div>
        </div>

        {/* 下载队列 */}
        <div className="flex-1 overflow-y-auto px-4 py-3">
          {tasks.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center gap-2 text-center">
              <Link2 className="size-8 text-line" />
              <p className="text-sm text-muted">还没有下载任务</p>
              <p className="text-xs text-dim">
                复制任意平台的视频链接粘贴到上方，支持抖音、B站、YouTube、Twitter 等站点
              </p>
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between mb-2 text-xs text-dim">
                <span>
                  队列 {tasks.length} 个任务 · 已完成 {doneCount}
                </span>
              </div>
              <div className="space-y-2">
                {tasks.map((t) => (
                  <TaskRow
                    key={t.id}
                    task={t}
                    onCancel={(id) => api.cancelTask(id).catch(() => {})}
                  />
                ))}
              </div>
            </>
          )}
        </div>
      </div>
    </AppLayout>
  )
}
