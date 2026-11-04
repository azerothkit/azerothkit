import { useCallback, useLayoutEffect, useRef, useState } from 'react'
import { useVirtualizer, type Virtualizer } from '@tanstack/react-virtual'
import type { LogEntry, LogLevel } from '@/core/logs'
import { Button, Badge } from '@/shared/ui'

type LogListProps = {
    entries: LogEntry[]
    resetKey: string
    emptyMessage: string
}

type ScrollAnchor = {
    id: number
    offset: number
}

type LogVirtualizer = Virtualizer<HTMLDivElement, HTMLDivElement>

const ESTIMATED_ENTRY_HEIGHT = 72
const BOTTOM_THRESHOLD = 8

const levelClasses: Record<LogLevel, string> = {
    trace: 'text-muted-foreground/70',
    debug: 'text-muted-foreground',
    info: 'text-sky-600 dark:text-sky-400',
    warn: 'text-amber-600 dark:text-amber-400',
    error: 'text-red-600 dark:text-red-400',
}

const levelBadgeClasses: Record<LogLevel, string> = {
    trace: 'bg-muted/40 text-muted-foreground/70',
    debug: 'bg-muted text-muted-foreground',
    info: 'bg-sky-500/10 text-sky-700 dark:bg-sky-400/15 dark:text-sky-400',
    warn: 'bg-amber-500/10 text-amber-700 dark:bg-amber-400/15 dark:text-amber-400',
    error: 'bg-red-500/10 text-red-700 dark:bg-red-400/15 dark:text-red-400',
}

const timeFormatter = new Intl.DateTimeFormat(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23',
})

function formatTime(timestamp: number) {
    if (!Number.isFinite(new Date(timestamp).getTime())) {
        return '--:--:--'
    }

    return timeFormatter.format(timestamp)
}

export function LogList({ entries, resetKey, emptyMessage }: LogListProps) {
    'use no memo'

    const scrollRef = useRef<HTMLDivElement>(null)
    const followingRef = useRef(true)
    const anchorRef = useRef<ScrollAnchor | null>(null)
    const frameRef = useRef<number | null>(null)
    const previousRef = useRef({ entries, resetKey })

    const [following, setFollowing] = useState(true)

    const setFollowingMode = useCallback((value: boolean) => {
        followingRef.current = value
        setFollowing(value)
    }, [])

    const scheduleFollow = useCallback((instance: LogVirtualizer) => {
        if (!followingRef.current || frameRef.current !== null) {
            return
        }

        frameRef.current = requestAnimationFrame(() => {
            frameRef.current = null

            const element = instance.scrollElement

            if (!followingRef.current || !element) return

            const offset = Math.max(0, instance.getTotalSize() - element.clientHeight)

            if (Math.abs(element.scrollTop - offset) > 1) {
                instance.scrollToOffset(offset, {
                    align: 'start',
                    behavior: 'auto',
                })
            }
        })
    }, [])

    const getItemKey = useCallback((index: number) => entries[index].id, [entries])

    // oxlint-disable-next-line react/incompatible-library
    const virtualizer = useVirtualizer<HTMLDivElement, HTMLDivElement>({
        count: entries.length,
        getScrollElement: () => scrollRef.current,
        estimateSize: () => ESTIMATED_ENTRY_HEIGHT,
        getItemKey,
        overscan: 8,
        onChange: scheduleFollow,
    })

    const pauseFollowing = () => {
        setFollowingMode(false)

        if (frameRef.current !== null) {
            cancelAnimationFrame(frameRef.current)
            frameRef.current = null
        }
    }

    const scrollToLatest = () => {
        setFollowingMode(true)
        scheduleFollow(virtualizer)
    }

    useLayoutEffect(() => {
        const previous = previousRef.current
        previousRef.current = { entries, resetKey }

        if (previous.resetKey !== resetKey || entries.length === 0) {
            anchorRef.current = null
            setFollowingMode(true)
        }

        if (followingRef.current) {
            scheduleFollow(virtualizer)
            return
        }

        if (previous.entries === entries) return

        const anchor = anchorRef.current
        const element = scrollRef.current

        if (!anchor || !element) return

        // Retention can remove entries above the viewport.
        // Restore the visible entry by ID instead of assuming fixed row heights.
        const index = entries.findIndex((entry) => entry.id === anchor.id)

        if (index === -1) {
            anchorRef.current = entries[0] ? { id: entries[0].id, offset: 0 } : null

            virtualizer.scrollToOffset(0)
            return
        }

        const position = virtualizer.getOffsetForIndex(index, 'start')

        if (!position) return

        const offset = Math.max(0, position[0] + anchor.offset)

        if (Math.abs(element.scrollTop - offset) > 1) {
            virtualizer.scrollToOffset(offset, {
                align: 'start',
                behavior: 'auto',
            })
        }
    }, [entries, resetKey, virtualizer, scheduleFollow, setFollowingMode])

    useLayoutEffect(() => {
        return () => {
            if (frameRef.current !== null) {
                cancelAnimationFrame(frameRef.current)
                frameRef.current = null
            }
        }
    }, [])

    return (
        <div className={'relative'}>
            <div
                ref={scrollRef}
                role="region"
                aria-label="Runtime logs"
                tabIndex={0}
                className="h-[min(60vh,560px)] min-h-64 overflow-auto overscroll-contain outline-none focus-visible:ring-2 focus-visible:ring-ring"
                style={{
                    overflowAnchor: 'none',
                    scrollbarGutter: 'stable',
                }}
                onWheel={(event) => {
                    if (event.deltaY < 0) pauseFollowing()
                }}
                onPointerDown={(event) => {
                    if (event.button === 0) pauseFollowing()
                }}
                onTouchStart={pauseFollowing}
                onKeyDown={(event) => {
                    if (event.key === 'ArrowUp' || event.key === 'PageUp' || event.key === 'Home') {
                        pauseFollowing()
                    }

                    if (event.key === 'End' && !event.shiftKey) {
                        event.preventDefault()
                        scrollToLatest()
                    }
                }}
                onScroll={(event) => {
                    const element = event.currentTarget
                    const distanceFromBottom =
                        element.scrollHeight - element.clientHeight - element.scrollTop

                    if (distanceFromBottom <= BOTTOM_THRESHOLD) {
                        setFollowingMode(true)
                    }

                    const firstVisible = virtualizer
                        .getVirtualItems()
                        .find((item) => item.end > element.scrollTop)

                    if (!firstVisible) return

                    const entry = entries[firstVisible.index]

                    if (!entry) return

                    anchorRef.current = {
                        id: entry.id,
                        offset: Math.max(0, element.scrollTop - firstVisible.start),
                    }
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
                        className="relative w-full"
                        style={{ height: virtualizer.getTotalSize() }}
                    >
                        {virtualizer.getVirtualItems().map((item) => {
                            const entry = entries[item.index]
                            const date = new Date(entry.timestamp)
                            const dateTime = Number.isFinite(date.getTime())
                                ? date.toISOString()
                                : undefined

                            return (
                                <div
                                    key={item.key}
                                    ref={virtualizer.measureElement}
                                    data-index={item.index}
                                    role="listitem"
                                    aria-posinset={item.index + 1}
                                    aria-setsize={entries.length}
                                    className="absolute left-0 top-0 w-full select-text border-b border-border/40 px-3 py-2 hover:bg-muted/25"
                                    style={{
                                        transform: `translateY(${item.start}px)`,
                                    }}
                                >
                                    <div className="grid grid-cols-[minmax(0,1fr)_auto] items-start gap-x-3 font-mono">
                                        <div className="flex min-w-0 flex-wrap items-start gap-1.5">
                                            <Badge
                                                variant="secondary"
                                                className={`h-4 rounded px-1.5 py-0 text-[10px] leading-none ${levelBadgeClasses[entry.level]}`}
                                            >
                                                {entry.level.toUpperCase()}
                                            </Badge>

                                            {entry.target && (
                                                <Badge
                                                    variant="secondary"
                                                    className="h-auto min-h-4 max-w-full min-w-0 justify-start whitespace-normal rounded bg-muted/60 px-1.5 py-0 text-[11px] leading-4 text-muted-foreground [overflow-wrap:anywhere]"
                                                >
                                                    {entry.target}
                                                </Badge>
                                            )}
                                        </div>

                                        <time
                                            dateTime={dateTime}
                                            className="whitespace-nowrap text-[10px] leading-4 tabular-nums text-muted-foreground/60"
                                        >
                                            {formatTime(entry.timestamp)}
                                        </time>
                                    </div>

                                    <div
                                        className={`mt-1 whitespace-pre-wrap font-mono text-[13px] leading-5 [overflow-wrap:anywhere] ${
                                            entry.level === 'error'
                                                ? levelClasses.error
                                                : 'text-foreground'
                                        }`}
                                    >
                                        {entry.message}
                                    </div>
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