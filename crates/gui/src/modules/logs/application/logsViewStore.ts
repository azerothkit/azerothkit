import { create } from 'zustand'
import type { LOG_LEVELS } from '../domain/constants'

type LogsLevelFilter = 'all' | (typeof LOG_LEVELS)[number]

type LogsViewStore = {
    level: LogsLevelFilter
    search: string
    setLevel: (level: LogsLevelFilter) => void
    setSearch: (search: string) => void
}

export const useLogsViewStore = create<LogsViewStore>()((set) => ({
    level: 'all',
    search: '',
    setLevel: (level) => set({ level }),
    setSearch: (search) => set({ search }),
}))