import { useMemo } from 'react'
import { useAppInfo } from '@/core/app'
import { MAX_LOG_ENTRIES, useLogsStore } from '@/core/logs'
import { useLogsViewStore } from '../../application/logsViewStore'
import { LOG_LEVEL_ITEMS } from '../../domain/constants'

export function useLogsScreen() {
    const entries = useLogsStore((state) => state.entries)
    const connection = useLogsStore((state) => state.connection)
    const connectionError = useLogsStore((state) => state.connectionError)
    const clear = useLogsStore((state) => state.clear)

    const level = useLogsViewStore((state) => state.level)
    const search = useLogsViewStore((state) => state.search)
    const setLevel = useLogsViewStore((state) => state.setLevel)
    const setSearch = useLogsViewStore((state) => state.setSearch)

    const { data: appInfo } = useAppInfo()

    const visibleEntries = useMemo(() => {
        const query = search.trim().toLowerCase()

        return entries.filter(
            (entry) =>
                (level === 'all' || entry.level === level) &&
                (!query ||
                    entry.message.toLowerCase().includes(query) ||
                    entry.target.toLowerCase().includes(query)),
        )
    }, [entries, level, search])

    const status = {
        idle: 'Not connected',
        connecting: 'Connecting…',
        ready: 'Live',
        error: 'Connection failed',
    }[connection]

    return {
        entries,
        visibleEntries,
        connection,
        connectionError,
        clear,
        level,
        search,
        setLevel,
        setSearch,
        appInfo,
        status,
        filtered: level !== 'all' || search.trim().length > 0,
        resetKey: JSON.stringify([level, search]),
        maxEntries: MAX_LOG_ENTRIES,
        levelItems: LOG_LEVEL_ITEMS,
    }
}