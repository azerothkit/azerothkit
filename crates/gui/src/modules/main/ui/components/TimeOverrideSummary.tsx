import { Link } from '@tanstack/react-router'
import { cn } from 'cn'
import { Badge } from '@/shared/ui'
import { buttonVariants } from '@/shared/ui'
import { Clock } from 'lucide-react'
import type { DurationUnit } from '@/shared/lib/time'
import { formatDuration } from '@/shared/lib/time'

type TimeOverrideSummaryProps = {
    enabled: boolean
    entryTime: string
    resetOnServerSync: boolean
    dayLengthMs: number
    dayLengthUnit: DurationUnit
}

export const TimeOverrideSummary = ({
    enabled,
    entryTime,
    resetOnServerSync,
    dayLengthMs,
    dayLengthUnit,
}: TimeOverrideSummaryProps) => {
    const formattedDayLength = formatDuration(dayLengthMs, dayLengthUnit, 'en')

    return (
        <div className="rounded-lg border px-4 py-3">
            <div className="flex items-center justify-between gap-3">
                <div className="flex flex-wrap items-center gap-2">
                    <h2 className="text-sm font-medium flex items-center gap-1.5">
                        <Clock
                            className="size-4 shrink-0"
                            strokeWidth={1.75}
                            aria-hidden="true"
                        />{' '}
                        Time override
                    </h2>

                    <Badge variant={enabled ? 'secondary' : 'outline'}>
                        {enabled ? 'Enabled' : 'Disabled'}
                    </Badge>
                </div>

                <Link
                    to="/settings"
                    aria-label="Edit time override settings"
                    className={cn(buttonVariants({ variant: 'ghost', size: 'sm' }), 'h-7 px-2')}
                >
                    Edit
                </Link>
            </div>

            {enabled && (
                <div className="mt-2 flex flex-wrap gap-1.5">
                    <Badge variant="outline">
                        <span className="font-normal text-muted-foreground">Entry</span>
                        <span className="tabular-nums">{entryTime}</span>
                    </Badge>

                    <Badge variant="outline">
                        <span className="font-normal text-muted-foreground">Day</span>
                        <span className="tabular-nums">{formattedDayLength}</span>
                    </Badge>

                    <Badge variant="outline">
                        <span className="font-normal text-muted-foreground">Sync reset</span>
                        {resetOnServerSync ? 'On' : 'Off'}
                    </Badge>
                </div>
            )}
        </div>
    )
}