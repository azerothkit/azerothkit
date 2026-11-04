import { useLayoutEffect, useRef, useState } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import type { LogEntry, LogLevel } from '@/core/logs'
import { Button } from '@/shared/ui'

type LogListProps = {
    entries: LogEntry[]
    resetKey: string
    emptyMessage: string
}

const ROW_HEIGHT = 32
const BOTTOM_THRESHOLD = 8

const levelClasses: Record<LogLevel, string> = {
    trace: 'text-muted-foreground/70',
    debug: 'text-muted-foreground',
    info: 'text-sky-600 dark:text-sky-400',
    warn: 'text-amber-600 dark:text-amber-400',
    error: 'text-red-600 dark:text-red-400',
}

const timeFormatter = new Intl.DateTimeFormat(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23',
})

function formatTime(timestamp: number) {
    if (!Number.isFinite(new Date(timestamp).getTime())) return '--:--:--.---'

    const milliseconds = String(((timestamp % 1_000) + 1_000) % 1_000).padStart(3, '0')
    return `${timeFormatter.format(timestamp)}.${milliseconds}`
}

export function LogList({ entries, resetKey, emptyMessage }: LogListProps) {
    'use no memo'

    const scrollRef = useRef<HTMLDivElement>(null)
    const followingRef = useRef(true)
    const previousRef = useRef({ entries, resetKey })
    const [following, setFollowing] = useState(true)

    // oxlint-disable-next-line react/incompatible-library
    const virtualizer = useVirtualizer({
        count: entries.length,
        getScrollElement: () => scrollRef.current,
        estimateSize: () => ROW_HEIGHT,
        getItemKey: (index) => entries[index].id,
        overscan: 12,
    })

    const scrollToLatest = () => {
        followingRef.current = true
        setFollowing(true)

        if (entries.length > 0) {
            virtualizer.scrollToIndex(entries.length - 1, { align: 'end' })
        }
    }

    useLayoutEffect(() => {
        const previous = previousRef.current
        previousRef.current = { entries, resetKey }

        if (previous.resetKey !== resetKey || entries.length === 0) {
            followingRef.current = true
            setFollowing(true)
        }

        if (followingRef.current) {
            if (entries.length > 0) {
                virtualizer.scrollToIndex(entries.length - 1, { align: 'end' })
            }
            return
        }

        // Retention removes rows at the beginning. Keep the same visible entry
        // anchored while it is still present in the retained buffer.
        const firstRetained = entries[0]
        if (!firstRetained || previous.entries === entries) return

        const removed = previous.entries.findIndex((entry) => entry.id === firstRetained.id)
        const scroll = scrollRef.current

        if (scroll && removed > 0) {
            scroll.scrollTop = Math.max(0, scroll.scrollTop - removed * ROW_HEIGHT)
        } else if (scroll && removed === -1) {
            scroll.scrollTop = 0
        }
    }, [entries, resetKey, virtualizer])

    return (
        <div className="relative">
            <div
                ref={scrollRef}
                role="region"
                aria-label="Runtime logs"
                tabIndex={0}
                className="h-[min(60vh,560px)] min-h-64 overflow-auto outline-none focus-visible:ring-2 focus-visible:ring-ring"
                style={{ overflowAnchor: 'none' }}
                onScroll={(event) => {
                    const element = event.currentTarget
                    const atBottom =
                        element.scrollHeight - element.clientHeight - element.scrollTop <=
                        BOTTOM_THRESHOLD

                    followingRef.current = atBottom
                    setFollowing(atBottom)
                }}
            >
                {entries.length === 0 ? (
                    <div className="flex h-full items-center justify-center px-6 text-center text-sm text-muted-foreground">
                        {emptyMessage}
                    </div>
                ) : (
                    <div
                        role="list"
                        aria-label="Log entries"
                        style={{ height: virtualizer.getTotalSize(), position: 'relative' }}
                    >
                        {virtualizer.getVirtualItems().map((item) => {
                            const entry = entries[item.index]
                            const date = new Date(entry.timestamp)
                            const dateLabel = Number.isFinite(date.getTime())
                                ? date.toISOString()
                                : 'Unknown time'
                            const fullText = `${dateLabel} [${entry.level.toUpperCase()}] [${entry.target}] ${entry.message}`

                            return (
                                <div
                                    key={entry.id}
                                    role="listitem"
                                    aria-posinset={item.index + 1}
                                    aria-setsize={entries.length}
                                    className="absolute left-0 top-0 flex w-full items-center gap-3 border-b border-border/40 px-4 font-mono text-xs hover:bg-muted/40"
                                    style={{
                                        height: ROW_HEIGHT,
                                        transform: `translateY(${item.start}px)`,
                                    }}
                                    title={fullText}
                                >
                                    <time className="w-[12ch] shrink-0 tabular-nums text-muted-foreground">
                                        {formatTime(entry.timestamp)}
                                    </time>

                                    <span
                                        className={`w-[5ch] shrink-0 font-semibold ${levelClasses[entry.level]}`}
                                    >
                                        {entry.level.toUpperCase()}
                                    </span>

                                    <span className="hidden w-36 shrink-0 truncate text-muted-foreground sm:block">
                                        {entry.target || '—'}
                                    </span>

                                    <span
                                        className={`min-w-0 flex-1 truncate whitespace-pre ${entry.level === 'error' ? levelClasses.error : 'text-foreground'}`}
                                    >
                                        {entry.message.replace(/\r?\n/g, ' ↵ ')}
                                    </span>
                                </div>
                            )
                        })}
                    </div>
                )}
            </div>

            {!following && entries.length > 0 && (
                <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    className="absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full border shadow-md"
                    onClick={scrollToLatest}
                >
                    Scroll to latest
                </Button>
            )}
        </div>
    )
}