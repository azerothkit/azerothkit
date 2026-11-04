import { cn } from 'cn'
import { Button } from '@/shared/ui'
import { EngineStatusIndicator } from '@/core/engine'

type ActivationControlProps = {
    active: boolean
    busy: boolean
    onToggle: () => Promise<void>
    onReconnect?: () => void
}

export const ActivationControl = ({
    active,
    busy,
    onToggle,
    onReconnect,
}: ActivationControlProps) => {
    const highlighted = active && !busy

    return (
        <section
            aria-label="Activation"
            className="flex flex-col items-center py-10"
        >
            <div
                className={cn(
                    'flex size-40 items-center justify-center rounded-full border transition-colors',
                    highlighted ? 'border-primary/20 bg-primary/5' : 'border-border bg-muted/30',
                )}
            >
                <Button
                    type="button"
                    variant="outline"
                    size="icon"
                    aria-label={active ? 'Disable companion' : 'Enable companion'}
                    aria-pressed={active}
                    aria-busy={busy}
                    disabled={busy}
                    onClick={() => void onToggle()}
                    className={cn(
                        'size-32 rounded-full border-2 shadow-sm transition-colors disabled:opacity-100',
                        highlighted
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
                        className={cn('size-11', busy && 'motion-safe:animate-pulse')}
                    >
                        <path d="M12 3v9" />
                        <path d="M6.35 5.65a9 9 0 1 0 11.3 0" />
                    </svg>
                </Button>
            </div>

            <EngineStatusIndicator
                size="md"
                className="mt-6"
            />

            {onReconnect && (
                <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={onReconnect}
                    className="mt-3"
                >
                    Retry connection
                </Button>
            )}
        </section>
    )
}