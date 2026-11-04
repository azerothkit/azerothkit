import { attachLogger, LogLevel as TauriLogLevel } from '@tauri-apps/plugin-log'
import { MAX_LOG_ENTRIES, useLogsStore, type LogEntry } from './store'

const FLUSH_INTERVAL_MS = 100

const levels: Record<TauriLogLevel, LogEntry['level']> = {
    [TauriLogLevel.Trace]: 'trace',
    [TauriLogLevel.Debug]: 'debug',
    [TauriLogLevel.Info]: 'info',
    [TauriLogLevel.Warn]: 'warn',
    [TauriLogLevel.Error]: 'error',
}

let nextId = 0

function decodeMessage(raw: string): Pick<LogEntry, 'timestamp' | 'target' | 'message'> {
    try {
        const value: unknown = JSON.parse(raw)

        if (
            typeof value === 'object' &&
            value !== null &&
            'timestamp' in value &&
            typeof value.timestamp === 'number' &&
            Number.isFinite(new Date(value.timestamp).getTime()) &&
            'target' in value &&
            typeof value.target === 'string' &&
            'message' in value &&
            typeof value.message === 'string'
        ) {
            return {
                timestamp: value.timestamp,
                target: value.target,
                message: value.message,
            }
        }
    } catch {
        // Keep messages readable if the Rust formatter has not been connected yet.
    }

    return { timestamp: Date.now(), target: '', message: raw }
}

export function connectLogs(): () => void {
    let disposed = false
    let detach: (() => void) | undefined
    let pending: LogEntry[] = []
    let generation = useLogsStore.getState().generation

    const synchronizeClear = () => {
        const current = useLogsStore.getState().generation

        if (current !== generation) {
            pending = []
            generation = current
        }
    }

    const flush = () => {
        synchronizeClear()

        if (pending.length === 0) return

        const batch = pending
        pending = []
        useLogsStore.getState().append(batch)
    }

    useLogsStore.setState({ connection: 'connecting', connectionError: null })

    const timer = setInterval(flush, FLUSH_INTERVAL_MS)

    void attachLogger(({ level, message }) => {
        if (disposed) return

        synchronizeClear()

        // Bound the intermediate queue as well as the store.
        if (pending.length >= MAX_LOG_ENTRIES) {
            pending.splice(0, pending.length - MAX_LOG_ENTRIES + 1)
        }

        pending.push({
            ...decodeMessage(message),
            id: ++nextId,
            level: levels[level] ?? 'info',
        })
    })
        .then((unlisten) => {
            if (disposed) {
                unlisten()
                return
            }

            detach = unlisten
            useLogsStore.setState({ connection: 'ready', connectionError: null })
        })
        .catch((error: unknown) => {
            if (disposed) return

            clearInterval(timer)
            useLogsStore.setState({
                connection: 'error',
                connectionError: error instanceof Error ? error.message : String(error),
            })
        })

    return () => {
        disposed = true
        clearInterval(timer)
        pending = []
        detach?.()
        useLogsStore.setState({ connection: 'idle', connectionError: null })
    }
}