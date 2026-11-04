import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { cn } from 'cn'
import { settingsSnapshotQuery } from '@/core/settings'
import { Button, buttonVariants } from '@/shared/ui'
import { Alert, AlertDescription, AlertTitle } from '@/shared/ui'
import { ClientLabel, PageSkeleton } from '@/shared/ui'
import { ActivationControl } from '@/modules/main/ui/components/ActivationControl'
import { TimeOverrideSummary } from '@/modules/main/ui/components/TimeOverrideSummary'
import { formatTimeOfDay } from '@/shared/lib/time'

export const MainScreen = () => {
    const query = useQuery(settingsSnapshotQuery)
    const [enabled, setEnabled] = useState(false)

    if (query.isPending) {
        return <PageSkeleton />
    }

    if (!query.data) {
        return (
            <Alert variant="destructive">
                <AlertTitle>Could not load settings</AlertTitle>

                <AlertDescription className="space-y-3">
                    <p>{query.error?.message}</p>

                    <Button
                        type="button"
                        variant="outline"
                        disabled={query.isFetching}
                        onClick={() => void query.refetch()}
                    >
                        Retry
                    </Button>
                </AlertDescription>
            </Alert>
        )
    }

    const { clients, settings } = query.data
    const client = clients.find((item) => item.id === settings.selectedClient)

    if (!client) {
        return (
            <Alert variant="destructive">
                <AlertTitle>Unknown client</AlertTitle>

                <AlertDescription>
                    The selected client is missing from the settings snapshot.
                </AlertDescription>
            </Alert>
        )
    }

    const timeOverride = settings.settingsByClient[client.id]?.features.timeOverride

    return (
        <div>
            <div className="flex items-center justify-between gap-4">
                <div>
                    <p className="text-xs text-muted-foreground">Game client</p>

                    <p className="mt-1 text-sm font-medium">
                        <ClientLabel
                            clientId={client.id}
                            label={client.label}
                        />
                    </p>
                </div>

                <Link
                    to="/settings"
                    aria-label="Change game client"
                    className={cn(buttonVariants({ variant: 'outline', size: 'sm' }))}
                >
                    Change
                </Link>
            </div>

            <ActivationControl
                enabled={enabled}
                onEnabledChange={setEnabled}
            />

            {client.capabilities.timeOverride &&
                (timeOverride ? (
                    <TimeOverrideSummary
                        enabled={timeOverride.enabled}
                        entryTime={formatTimeOfDay(timeOverride.entryTimeMinutes)}
                        resetOnServerSync={timeOverride.resetOnServerSync}
                        dayLengthMs={timeOverride.dayLengthMs}
                        dayLengthUnit={timeOverride.dayLengthUnit}
                    />
                ) : (
                    <Alert variant="destructive">
                        <AlertTitle>Missing time override settings</AlertTitle>

                        <AlertDescription>
                            No saved settings were found for the selected client.
                        </AlertDescription>
                    </Alert>
                ))}
        </div>
    )
}