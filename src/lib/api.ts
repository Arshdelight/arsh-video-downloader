import { invoke } from '@tauri-apps/api/core'
import { hasNativeBridge } from './tauri'

export interface Task {
  id: number
  url: string
  title: string
  status: 'pending' | 'downloading' | 'paused' | 'done' | 'canceled' | 'error'
  progress: number
  speed: string
  dir: string
  error: string
  filePath: string | null
}

/** 所有命令在纯网页模式（npm run dev:web）下降级为空实现 */
export const api = {
  getDir: (): Promise<string> =>
    hasNativeBridge ? invoke<string>('get_download_dir') : Promise.resolve(''),
  pickDir: (): Promise<string | null> =>
    hasNativeBridge ? invoke<string | null>('select_download_dir') : Promise.resolve(null),
  listTasks: (): Promise<Task[]> =>
    hasNativeBridge ? invoke<Task[]>('list_tasks') : Promise.resolve([]),
  addTask: (url: string): Promise<number> =>
    hasNativeBridge ? invoke<number>('add_task', { url }) : Promise.resolve(0),
  cancelTask: (id: number): Promise<void> =>
    hasNativeBridge ? invoke<void>('cancel_task', { id }) : Promise.resolve(),
  pauseTask: (id: number): Promise<void> =>
    hasNativeBridge ? invoke<void>('pause_task', { id }) : Promise.resolve(),
  startTask: (id: number): Promise<void> =>
    hasNativeBridge ? invoke<void>('start_task', { id }) : Promise.resolve(),
  deleteTask: (id: number): Promise<void> =>
    hasNativeBridge ? invoke<void>('delete_task', { id }) : Promise.resolve(),
  clearFinished: (): Promise<number> =>
    hasNativeBridge ? invoke<number>('clear_finished') : Promise.resolve(0),
  openTaskDir: (id: number): Promise<void> =>
    hasNativeBridge ? invoke<void>('open_task_dir', { id }) : Promise.resolve(),
  openTaskFile: (id: number): Promise<void> =>
    hasNativeBridge ? invoke<void>('open_task_file', { id }) : Promise.resolve(),
}
