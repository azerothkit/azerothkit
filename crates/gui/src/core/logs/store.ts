import { create } from 'zustand'

export const MAX_LOG_ENTRIES = 5_000

export type LogLevel = 'trace' | 'debug' | 'info' | 'warn' | 'error'

export type LogEntry = {
    id: number
    timestamp: number
    level: LogLevel
    target: string
    message: string
}

type LogsStore = {
    entries: LogEntry[]
    connection: 'idle' | 'connecting' | 'ready' | 'error'
    connectionError: string | null
    generation: number
    append: (entries: LogEntry[]) => void
    clear: () => void
}

export const useLogsStore = create<LogsStore>()((set) => ({
    entries: [],
    connection: 'idle',
    connectionError: null,
    generation: 0,
    append: (batch) => {
        if (batch.length === 0) return

        set((state) => ({
            entries: [...state.entries, ...batch].slice(-MAX_LOG_ENTRIES),
        }))
    },
    clear: () => set((state) => ({
        entries: [],
        generation: state.generation + 1,
    })),
}))
