import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { AppLayout, TitleBar, Button } from './ui'
import { hasNativeBridge } from './lib/tauri'

export default function App() {
  const [count, setCount] = useState(0)
  const [greetMsg, setGreetMsg] = useState('')

  async function greet() {
    if (!hasNativeBridge) return
    setGreetMsg(await invoke<string>('greet', { name: 'Arsh' }))
  }

  return (
    <AppLayout titleBar={<TitleBar brand="Arshvideodownloader" />}>
      <div className="h-full flex flex-col items-center justify-center gap-4 text-center px-6">
        <h1 className="text-3xl font-semibold text-main">Congratulations!</h1>
        <p className="text-sm text-muted max-w-md leading-relaxed">
          Your Tauri project has been initialized. Open this project with any AI
          coding tool and build your own marvelous Tauri app.
        </p>
        <div className="flex items-center gap-2">
          <Button onClick={() => setCount((c) => c + 1)}>count is {count}</Button>
          <Button variant="secondary" onClick={greet}>greet from Rust</Button>
        </div>
        {greetMsg && <p className="text-xs text-dim">{greetMsg}</p>}
        <p className="text-xs text-dim">
          Edit src/App.tsx and save to see hot reload. The Rust backend lives in
          src-tauri/.
        </p>
      </div>
    </AppLayout>
  )
}
