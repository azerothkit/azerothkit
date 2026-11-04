import { useState } from 'react'
import { Link } from '@tanstack/react-router'
import { cn } from 'cn'
import { buttonVariants } from '@/shared/ui/components/button'
import { ActivationControl } from '@/modules/main/ui/components/ActivationControl'
import { TimeOverrideSummary } from '@/modules/main/ui/components/TimeOverrideSummary'

const preview = {
    client: 'WoW Forever',
    timeOverride: {
        entryTime: '18:00',
        resetOnServerSync: true,
        dayLengthSeconds: 20,
    },
}

export const MainScreen = () => {
    const [enabled, setEnabled] = useState(false)

    return (
        <div>
            <div className="flex items-center justify-between gap-4">
                <div>
                    <p className="text-xs text-muted-foreground">Game client</p>

                    <p className="mt-1 text-sm font-medium">{preview.client}</p>
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

            <TimeOverrideSummary {...preview.timeOverride} />
        </div>
    )
}