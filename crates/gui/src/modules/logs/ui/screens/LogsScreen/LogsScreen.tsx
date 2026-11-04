import { useMemo, useState } from 'react'
import { MAX_LOG_ENTRIES, useLogsStore, type LogLevel } from '@/core/logs'
import { Button, Card } from '@/shared/ui'
import { LogList } from '../../components/LogList'
import { useAppInfo } from '@/core/app'

const levels: LogLevel[] = ['error', 'warn', 'info', 'debug', 'trace']

export const LogsScreen = () => {
    const entries = useLogsStore((state) => state.entries)
    const connection = useLogsStore((state) => state.connection)
    const connectionError = useLogsStore((state) => state.connectionError)
    const clear = useLogsStore((state) => state.clear)
    const [level, setLevel] = useState<LogLevel | 'all'>('all')
    const [search, setSearch] = useState('')
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

    const filtered = level !== 'all' || search.trim().length > 0
    const status = {
        idle: 'Not connected',
        connecting: 'Connecting…',
        ready: 'Live',
        error: 'Connection failed',
    }[connection]

    return (
        <section className="space-y-5">
            <div className="space-y-2">
                <h1 className="text-lg font-semibold tracking-tight">Logs</h1>
            </div>

            <Card className="gap-0 overflow-hidden py-0">
                <div className="flex flex-wrap items-center gap-3 border-b p-4">
                    <input
                        type="search"
                        aria-label="Search logs"
                        placeholder="Search messages or modules…"
                        value={search}
                        onChange={(event) => setSearch(event.target.value)}
                        className="h-9 min-w-40 flex-1 rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    />

                    <select
                        aria-label="Log level"
                        value={level}
                        onChange={(event) => setLevel(event.target.value as LogLevel | 'all')}
                        className="h-9 rounded-md border border-input bg-background px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                        <option value="all">All levels</option>
                        {levels.map((item) => (
                            <option
                                key={item}
                                value={item}
                            >
                                {item.toUpperCase()}
                            </option>
                        ))}
                    </select>

                    <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={clear}
                    >
                        Clear
                    </Button>
                </div>

                {connectionError && (
                    <p
                        role="alert"
                        className="border-b bg-destructive/5 px-4 py-3 text-sm text-destructive"
                    >
                        Failed to subscribe to logs: {connectionError}
                    </p>
                )}

                <LogList
                    entries={visibleEntries}
                    resetKey={JSON.stringify([level, search])}
                    emptyMessage={
                        filtered
                            ? 'No matching logs.'
                            : 'No logs yet. New messages will appear here.'
                    }
                />

                <div className="flex flex-wrap items-center justify-between gap-2 border-t px-4 py-3 text-xs text-muted-foreground">
                    <span>
                        {visibleEntries.length.toLocaleString()} shown ·{' '}
                        {entries.length.toLocaleString()} / {MAX_LOG_ENTRIES.toLocaleString()}{' '}
                        retained
                    </span>
                    <span className="inline-flex items-center gap-2">
                        <span
                            aria-hidden="true"
                            className={`size-1.5 rounded-full ${connection === 'ready' ? 'bg-green-500' : connection === 'error' ? 'bg-red-500' : 'bg-muted-foreground/50'}`}
                        />
                        {status}
                    </span>
                </div>
            </Card>

            {appInfo?.logsDir && (
                <div className="space-y-1.5 px-1">
                    <p className="text-xs font-medium text-muted-foreground">Log files directory</p>

                    <p className="select-all break-all font-mono text-xs text-muted-foreground">
                        {appInfo.logsDir}
                    </p>
                </div>
            )}
        </section>
    )
}