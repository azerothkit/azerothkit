import {
    Button,
    Card,
    Input,
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from '@/shared/ui'
import { LogList } from '../../components/LogList'
import { useLogsScreen } from '../../hooks/useLogsScreen'

export const LogsScreen = () => {
    const {
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
        filtered,
        resetKey,
        maxEntries,
        levelItems,
    } = useLogsScreen()

    return (
        <section className="space-y-5">
            <Card className="gap-0 overflow-hidden py-0">
                <div className="flex flex-wrap items-center gap-3 border-b p-3">
                    <div className="min-w-40 flex-1">
                        <Input
                            type="search"
                            aria-label="Search logs"
                            placeholder="Search messages or modules…"
                            value={search}
                            onChange={(event) => setSearch(event.target.value)}
                        />
                    </div>

                    <Select
                        items={levelItems}
                        value={level}
                        onValueChange={(value) => {
                            if (value !== null) setLevel(value)
                        }}
                    >
                        <SelectTrigger
                            aria-label="Log level"
                            className="min-w-32"
                        >
                            <SelectValue />
                        </SelectTrigger>

                        <SelectContent
                            align="start"
                            alignItemWithTrigger={false}
                        >
                            {levelItems.map((item) => (
                                <SelectItem
                                    key={item.value}
                                    value={item.value}
                                >
                                    {item.label}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>

                    <Button
                        type="button"
                        variant="outline"
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
                    resetKey={resetKey}
                    emptyMessage={
                        filtered
                            ? 'No matching logs.'
                            : 'No logs yet. New messages will appear here.'
                    }
                />

                <div className="flex flex-wrap items-center justify-between gap-2 border-t px-4 py-3 text-xs text-muted-foreground">
                    <span>
                        {visibleEntries.length.toLocaleString()} shown ·{' '}
                        {entries.length.toLocaleString()} / {maxEntries.toLocaleString()} retained
                    </span>

                    <span className="inline-flex items-center gap-2">
                        <span
                            aria-hidden="true"
                            className={`size-1.5 rounded-full ${
                                connection === 'ready'
                                    ? 'bg-green-500'
                                    : connection === 'error'
                                      ? 'bg-red-500'
                                      : 'bg-muted-foreground/50'
                            }`}
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