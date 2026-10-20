import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import { listen } from '@tauri-apps/api/event'
import {
  CheckCircle2,
  Clock,
  FileVideo2,
  FolderOpen,
  ExternalLink,
  Link2,
  Loader2,
  Pause,
  Play,
  Plus,
  RotateCw,
  Settings,
  Trash2,
  X,
  XCircle,
  type LucideIcon,
} from 'lucide-react'
import { AppLayout, TitleBar, Button, Dialog } from './ui'
import { cn } from './lib/utils'
import { hasNativeBridge } from './lib/tauri'
import { api, type Task } from './lib/api'
import { EVENT } from './lib/events'

const STATUS_META: Record<Task['status'], { label: string; className: string }> = {
  pending: { label: '等待中', className: 'text-dim' },
  downloading: { label: '下载中', className: 'text-main' },
  paused: { label: '已暂停', className: 'text-warning' },
  done: { label: '已完成', className: 'text-success' },
  canceled: { label: '已取消', className: 'text-dim' },
  error: { label: '失败', className: 'text-destructive' },
}

function StatusIcon({ status }: { status: Task['status'] }) {
  switch (status) {
    case 'downloading':
      return <Loader2 className="size-4 shrink-0 mt-0.5 animate-spin text-accent" />
    case 'paused':
      return <Pause className="size-4 shrink-0 mt-0.5 text-warning" />
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

interface IconBtnProps {
  title: string
  icon: LucideIcon
  onClick: () => void
  danger?: boolean
}

function IconBtn({ title, icon: Icon, onClick, danger = false }: IconBtnProps) {
  return (
    <button
      onClick={(e) => {
        e.stopPropagation()
        onClick()
      }}
      onContextMenu={(e) => e.stopPropagation()}
      className={cn(
        'size-6 flex items-center justify-center rounded-md transition-colors cursor-pointer',
        danger
          ? 'text-muted hover:text-destructive hover:bg-hover'
          : 'text-muted hover:text-main hover:bg-hover',
      )}
      title={title}
      aria-label={title}
    >
      <Icon className="size-3.5" />
    </button>
  )
}

interface TaskRowProps {
  task: Task
  onPause: (id: number) => void
  onStart: (id: number) => void
  onDelete: (id: number) => void
  onOpenFile: (task: Task) => void
  onOpenDir: (task: Task) => void
  onContextMenu: (e: React.MouseEvent, task: Task) => void
}

function TaskRow({ task, onPause, onStart, onDelete, onOpenFile, onOpenDir, onContextMenu }: TaskRowProps) {
  const meta = STATUS_META[task.status]

  return (
    <div
      className={cn(
        'group rounded-lg border border-line bg-surface p-3 select-none',
        'hover:border-accent/30 hover:bg-hover/30 transition-colors cursor-default',
      )}
      onContextMenu={(e) => onContextMenu(e, task)}
      onDoubleClick={() => (task.status === 'done' ? onOpenFile(task) : onOpenDir(task))}
    >
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

        {/* hover 快捷操作 */}
        <div
          className={cn(
            'flex items-center gap-0.5 shrink-0 transition-opacity',
            'opacity-0 group-hover:opacity-100',
          )}
        >
          {task.status === 'downloading' && (
            <IconBtn title="暂停" icon={Pause} onClick={() => onPause(task.id)} />
          )}
          {task.status === 'paused' && <IconBtn title="继续" icon={Play} onClick={() => onStart(task.id)} />}
          {(task.status === 'error' || task.status === 'canceled') && (
            <IconBtn title="重新下载" icon={RotateCw} onClick={() => onStart(task.id)} />
          )}
          {task.status === 'done' && (
            <IconBtn title="打开文件" icon={FileVideo2} onClick={() => onOpenFile(task)} />
          )}
          <IconBtn title="删除任务" icon={Trash2} danger onClick={() => onDelete(task.id)} />
        </div>
      </div>
      {(task.status === 'downloading' || task.status === 'paused') && (
        <div className="mt-2 h-1.5 rounded-full bg-hover overflow-hidden">
          <div
            className={cn(
              'h-full rounded-full transition-[width] duration-300',
              task.status === 'paused' ? 'bg-warning' : 'bg-accent',
            )}
            style={{ width: `${task.progress}%` }}
          />
        </div>
      )}
    </div>
  )
}

interface MenuItem {
  label: string
  icon: LucideIcon
  danger?: boolean
  onClick: () => void
}

function menuItemsFor(
  task: Task,
  actions: {
    onPause: (id: number) => void
    onStart: (id: number) => void
    onDelete: (id: number) => void
    onOpenFile: (task: Task) => void
    onOpenDir: (task: Task) => void
  },
): MenuItem[] {
  const items: MenuItem[] = []
  if (task.status === 'downloading') {
    items.push({ label: '暂停', icon: Pause, onClick: () => actions.onPause(task.id) })
  }
  if (task.status === 'paused') {
    items.push({ label: '继续', icon: Play, onClick: () => actions.onStart(task.id) })
  }
  if (task.status === 'error' || task.status === 'canceled') {
    items.push({ label: '重新下载', icon: RotateCw, onClick: () => actions.onStart(task.id) })
  }
  items.push({ label: '打开目录', icon: FolderOpen, onClick: () => actions.onOpenDir(task) })
  if (task.status === 'done') {
    items.push({ label: '打开文件', icon: FileVideo2, onClick: () => actions.onOpenFile(task) })
  }
  items.push({ label: '删除任务', icon: Trash2, danger: true, onClick: () => actions.onDelete(task.id) })
  return items
}

interface MenuState {
  x: number
  y: number
  task: Task
}

function ContextMenu({
  menu,
  actions,
  onClose,
}: {
  menu: MenuState
  actions: Parameters<typeof menuItemsFor>[1]
  onClose: () => void
}) {
  useEffect(() => {
    const close = () => onClose()
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && onClose()
    window.addEventListener('click', close)
    window.addEventListener('blur', close)
    window.addEventListener('keydown', onKey)
    return () => {
      window.removeEventListener('click', close)
      window.removeEventListener('blur', close)
      window.removeEventListener('keydown', onKey)
    }
  }, [onClose])

  const items = menuItemsFor(menu.task, actions)
  const left = Math.min(menu.x, window.innerWidth - 176)
  const top = Math.min(menu.y, window.innerHeight - items.length * 32 - 8)

  return createPortal(
    <div className="fixed inset-0 z-[90]" onClick={onClose} onContextMenu={(e) => { e.preventDefault(); onClose() }}>
      <div
        className="absolute min-w-40 rounded-lg border border-line bg-surface py-1 shadow-lg animate-[fade-in_100ms_ease-out]"
        style={{ left, top }}
        onClick={(e) => e.stopPropagation()}
      >
        {items.map((item) => (
          <button
            key={item.label}
            onClick={() => {
              item.onClick()
              onClose()
            }}
            className={cn(
              'w-full flex items-center gap-2 px-3 py-1.5 text-xs text-left cursor-pointer transition-colors',
              item.danger ? 'text-destructive hover:bg-destructive/10' : 'text-main hover:bg-hover',
            )}
          >
            <item.icon className="size-3.5 shrink-0" />
            {item.label}
          </button>
        ))}
      </div>
    </div>,
    document.body,
  )
}

export default function App() {
  const [url, setUrl] = useState('')
  const [dir, setDir] = useState('')
  const [tasks, setTasks] = useState<Task[]>([])
  const [notice, setNotice] = useState('')
  const [menu, setMenu] = useState<MenuState | null>(null)
  const [updateVersion, setUpdateVersion] = useState<string | null>(null)
  const [updating, setUpdating] = useState(false)
  const [installed, setInstalled] = useState(false)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [appVersion, setAppVersion] = useState('')
  const [engines, setEngines] = useState({ ytdlp: true, ffmpeg: true, parsehub: true })
  const [checkState, setCheckState] = useState<'idle' | 'checking' | 'latest' | 'available'>('idle')

  useEffect(() => {
    if (!hasNativeBridge) return
    api.getDir().then(setDir).catch(() => {})
    api.listTasks().then(setTasks).catch(() => {})
    api.isInstalled().then(setInstalled).catch(() => {})
    // 启动静默检查更新（失败静默：无网络/latest.json 不存在时忽略）
    api.checkUpdate().then((v) => v && setUpdateVersion(v)).catch(() => {})
    const un = listen<Task[]>(EVENT, (e) => setTasks(e.payload))
    return () => {
      un.then((f) => f())
    }
  }, [])

  useEffect(() => {
    if (!settingsOpen || !hasNativeBridge) return
    // 每次打开设置页刷新版本与引擎状态
    api.getAppVersion().then(setAppVersion).catch(() => {})
    api.engineStatus().then(setEngines).catch(() => {})
  }, [settingsOpen])

  const checkNow = () => {
    setCheckState('checking')
    api
      .checkUpdate()
      .then((v) => {
        if (v) {
          setUpdateVersion(v)
          setCheckState('available')
        } else {
          setCheckState('latest')
        }
      })
      .catch(() => setCheckState('idle'))
  }

  const doUpdate = () => {
    setUpdating(true)
    // 安装成功时应用会自动重启
    api.installUpdate().catch((e) => {
      setNotice(typeof e === 'string' ? e : String(e))
      setUpdating(false)
    })
  }

  const add = () => {
    const text = url.trim()
    if (!text) return
    api
      .addTask(text)
      .then((added) => {
        if (added > 0) {
          setUrl('')
          setNotice('')
        } else {
          setNotice('未在输入中识别到视频链接，请粘贴包含 http(s):// 的链接')
          window.setTimeout(() => setNotice(''), 4000)
        }
      })
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

  const withOpenError = (p: Promise<void>) =>
    p.catch((e) => {
      const msg = typeof e === 'string' ? e : String(e)
      setNotice(msg)
      window.setTimeout(() => setNotice(''), 5000)
    })
  const actions = {
    onPause: (id: number) => api.pauseTask(id).catch(() => {}),
    onStart: (id: number) => api.startTask(id).catch(() => {}),
    onDelete: (id: number) => api.deleteTask(id).catch(() => {}),
    onOpenFile: (task: Task) => withOpenError(api.openTaskFile(task.id)),
    onOpenDir: (task: Task) => withOpenError(api.openTaskDir(task.id)),
  }

  const doneCount = tasks.filter((t) => t.status === 'done').length
  const finishedCount = tasks.filter((t) =>
    ['done', 'canceled', 'error'].includes(t.status),
  ).length

  return (
    <AppLayout
      titleBar={
        <TitleBar
          brand="ArshVideoDownloader"
          actions={
            <button
              onClick={() => setSettingsOpen(true)}
              className="size-5 flex items-center justify-center text-muted hover:text-main transition-colors cursor-pointer"
              title="设置"
              aria-label="设置"
            >
              <Settings className="size-3.5" />
            </button>
          }
        />
      }
    >
      {updateVersion && (
        <div className="shrink-0 flex items-center gap-2 px-4 py-1.5 bg-warning/10 border-b border-line text-xs">
          <span className="text-main">发现新版本 v{updateVersion}</span>
          {installed ? (
            <button
              onClick={doUpdate}
              disabled={updating}
              className="font-medium text-accent hover:underline cursor-pointer disabled:opacity-50"
            >
              {updating ? '正在下载并安装…' : '立即更新'}
            </button>
          ) : (
            <a
              href="https://github.com/Arshdelight/arsh-video-downloader/releases/latest"
              target="_blank"
              rel="noreferrer"
              className="font-medium text-accent hover:underline cursor-pointer"
            >
              前往下载页
            </a>
          )}
          <button
            onClick={() => setUpdateVersion(null)}
            className="ml-auto text-dim hover:text-main cursor-pointer"
            aria-label="关闭"
          >
            <X className="size-3.5" />
          </button>
        </div>
      )}
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
                placeholder="粘贴视频链接或整段分享文案，会自动识别其中的链接"
                className="w-full h-8 rounded-lg border border-line bg-surface pl-8 pr-3 text-sm text-main placeholder:text-dim focus:outline-none focus:ring-2 focus:ring-accent/20"
              />
            </div>
            <Button icon={<Plus className="size-4" />} onClick={add} disabled={!url.trim()}>
              添加下载
            </Button>
          </div>
          {notice && <div className="text-xs text-destructive">{notice}</div>}
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
                {finishedCount > 0 && (
                  <button
                    onClick={() => api.clearFinished().catch(() => {})}
                    className="hover:text-main transition-colors cursor-pointer"
                  >
                    清空已完成
                  </button>
                )}
              </div>
              <div className="space-y-2">
                {tasks.map((t) => (
                  <TaskRow
                    key={t.id}
                    task={t}
                    onPause={actions.onPause}
                    onStart={actions.onStart}
                    onDelete={actions.onDelete}
                    onOpenFile={actions.onOpenFile}
                    onOpenDir={actions.onOpenDir}
                    onContextMenu={(e, task) => {
                      e.preventDefault()
                      setMenu({ x: e.clientX, y: e.clientY, task })
                    }}
                  />
                ))}
              </div>
            </>
          )}
        </div>
      </div>

      <Dialog open={settingsOpen} onClose={() => setSettingsOpen(false)}>
        <div className="p-4 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-semibold text-main">设置</h2>
            <button
              onClick={() => setSettingsOpen(false)}
              className="text-muted hover:text-main transition-colors cursor-pointer"
              aria-label="关闭"
            >
              <X className="size-4" />
            </button>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted">版本</span>
              <span className="text-sm text-main">v{appVersion}</span>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted">检查更新</span>
              <Button size="sm" variant="secondary" onClick={checkNow} loading={checkState === 'checking'}>
                {checkState === 'checking' ? '检查中…' : '检查更新'}
              </Button>
            </div>
            {checkState === 'latest' && <div className="text-xs text-success">✓ 已是最新版本</div>}
            {checkState === 'available' && (
              <div className="text-xs text-warning">
                发现新版本 v{updateVersion}
                {installed && (
                  <button
                    onClick={doUpdate}
                    disabled={updating}
                    className="ml-2 text-accent hover:underline cursor-pointer disabled:opacity-50"
                  >
                    {updating ? '正在更新…' : '立即更新'}
                  </button>
                )}
                {!installed && (
                  <button
                    onClick={() => api.openRepo().catch(() => {})}
                    className="ml-2 text-accent hover:underline cursor-pointer"
                  >
                    前往下载页
                  </button>
                )}
              </div>
            )}
          </div>

          <div className="h-px bg-line" />

          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs text-muted">
              <span>引擎状态</span>
              <span className={cn(engines.ytdlp && engines.ffmpeg && engines.parsehub ? 'text-success' : 'text-destructive')}>
                yt-dlp {engines.ytdlp ? '✓' : '✗'} · ffmpeg {engines.ffmpeg ? '✓' : '✗'} · ParseHub{' '}
                {engines.parsehub ? '✓' : '✗'}
              </span>
            </div>
            <div className="flex gap-2 pt-1">
              <Button
                size="sm"
                variant="secondary"
                icon={<FolderOpen className="size-3.5" />}
                onClick={() => api.openDataDir().catch(() => {})}
              >
                数据目录
              </Button>
              <Button
                size="sm"
                variant="secondary"
                icon={<ExternalLink className="size-3.5" />}
                onClick={() => api.openRepo().catch(() => {})}
              >
                GitHub 仓库
              </Button>
            </div>
          </div>
        </div>
      </Dialog>

      {menu && (
        <ContextMenu menu={menu} actions={actions} onClose={() => setMenu(null)} />
      )}
    </AppLayout>
  )
}
