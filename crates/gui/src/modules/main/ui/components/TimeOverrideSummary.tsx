import { Link } from '@tanstack/react-router'
import { cn } from 'cn'
import { Badge } from '@/shared/ui/components/badge'
import { buttonVariants } from '@/shared/ui/components/button'
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
} from '@/shared/ui/components/card'

type TimeOverrideSummaryProps = {
    entryTime: string
    resetOnServerSync: boolean
    dayLengthSeconds: number
}

export const TimeOverrideSummary = ({
    entryTime,
    resetOnServerSync,
    dayLengthSeconds,
}: TimeOverrideSummaryProps) => {
    return (
        <Card>
            <CardHeader>
                <CardTitle className="flex items-center gap-2">
                    Time override
                    <Badge variant="secondary">Configured</Badge>
                </CardTitle>

                <CardDescription>Saved settings for the selected client.</CardDescription>

                <CardAction>
                    <Link
                        to="/settings"
                        aria-label="Edit time override settings"
                        className={cn(buttonVariants({ variant: 'ghost', size: 'sm' }))}
                    >
                        Edit
                    </Link>
                </CardAction>
            </CardHeader>

            <CardContent>
                <dl className="divide-y text-sm">
                    <div className="flex items-baseline justify-between gap-4 py-3 first:pt-0">
                        <dt className="text-muted-foreground">Time on world entry</dt>

                        <dd className="shrink-0 font-medium tabular-nums">{entryTime}</dd>
                    </div>

                    <div className="flex items-baseline justify-between gap-4 py-3">
                        <dt className="text-muted-foreground">Reset on server sync</dt>

                        <dd className="shrink-0 font-medium">{resetOnServerSync ? 'On' : 'Off'}</dd>
                    </div>

                    <div className="flex items-baseline justify-between gap-4 py-3 last:pb-0">
                        <dt className="text-muted-foreground">Day length</dt>

                        <dd className="shrink-0 font-medium tabular-nums">
                            {dayLengthSeconds} seconds
                        </dd>
                    </div>
                </dl>

                <p className="mt-4 border-t pt-3 text-xs leading-relaxed text-muted-foreground">
                    A full in-game day takes {dayLengthSeconds} real-world seconds.
                    {resetOnServerSync && ' Server sync resets the clock to the entry time.'}
                </p>
            </CardContent>
        </Card>
    )
}