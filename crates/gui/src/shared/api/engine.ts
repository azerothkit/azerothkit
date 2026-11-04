import { listen } from '@tauri-apps/api/event'
import type { commands } from './rust/bindings'

export type EngineSnapshot = Extract<
    Awaited<ReturnType<typeof commands.getEngineSnapshot>>,
    { status: 'ok' }
>['data']

export type EngineStatus = EngineSnapshot['status']

export function listenEngineStatus(onSnapshot: (snapshot: EngineSnapshot) => void) {
    return listen<EngineSnapshot>('engine-status-changed', ({ payload }) => {
        onSnapshot(payload)
    })
}