import { cn } from 'cn'
import { Button } from '@/shared/ui/components/button'

type ActivationControlProps = {
    enabled: boolean
    onEnabledChange: (enabled: boolean) => void
}

export const ActivationControl = ({ enabled, onEnabledChange }: ActivationControlProps) => {
    return (
        <section
            aria-label="Activation"
            className="flex flex-col items-center py-10"
        >
            <div
                className={cn(
                    'flex size-40 items-center justify-center rounded-full border transition-colors',
                    enabled ? 'border-primary/20 bg-primary/5' : 'border-border bg-muted/30',
                )}
            >
                <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    aria-label="Enable companion"
                    aria-pressed={enabled}
                    onClick={() => onEnabledChange(!enabled)}
                    className={cn(
                        'size-32 rounded-full border-2 shadow-sm',
                        enabled
                            ? 'border-primary bg-primary text-primary-foreground hover:bg-primary/90 hover:text-primary-foreground dark:bg-primary'
                            : 'border-border bg-background text-muted-foreground hover:bg-muted hover:text-foreground dark:bg-background',
                    )}
                >
                    <svg
                        aria-hidden="true"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth={1.75}
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        className="size-11"
                    >
                        <path d="M12 3v9" />
                        <path d="M6.35 5.65a9 9 0 1 0 11.3 0" />
                    </svg>
                </Button>
            </div>

            <div
                role="status"
                className="mt-6 text-center"
            >
                <div className="flex items-center justify-center gap-2">
                    <span
                        aria-hidden="true"
                        className={cn(
                            'size-2 rounded-full',
                            enabled ? 'bg-primary' : 'bg-muted-foreground/50',
                        )}
                    />

                    <h1 className="text-lg font-semibold tracking-tight">
                        {enabled ? 'Waiting for game' : 'Disabled'}
                    </h1>
                </div>

                <p className="mt-2 max-w-72 text-sm text-muted-foreground">
                    {enabled
                        ? 'Launch the selected WoW client to apply your settings.'
                        : 'Enable the companion when you are ready to play.'}
                </p>
            </div>
        </section>
    )
}