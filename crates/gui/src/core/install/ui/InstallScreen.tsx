import { Button } from '@/shared/ui'

type ScreenAction = {
    label: string
    onClick: () => void
    disabled?: boolean
}

type InstallScreenProps = {
    title: string
    description: string
    action?: ScreenAction
    secondaryAction?: ScreenAction
}

export function InstallScreen({ title, description, action, secondaryAction }: InstallScreenProps) {
    return (
        <main className="flex min-h-0 flex-1 items-center justify-center overflow-y-auto p-6">
            <div className="w-full max-w-sm space-y-6 text-center">
                <div
                    className="space-y-3"
                    role="status"
                    aria-live="polite"
                >
                    <h1 className="text-xl font-semibold tracking-tight">{title}</h1>

                    <p className="whitespace-pre-line break-words text-sm text-muted-foreground">
                        {description}
                    </p>
                </div>

                {(action || secondaryAction) && (
                    <div className="flex flex-wrap justify-center gap-3">
                        {action && (
                            <Button
                                type="button"
                                disabled={action.disabled}
                                onClick={action.onClick}
                            >
                                {action.label}
                            </Button>
                        )}

                        {secondaryAction && (
                            <Button
                                type="button"
                                variant="outline"
                                disabled={secondaryAction.disabled}
                                onClick={secondaryAction.onClick}
                            >
                                {secondaryAction.label}
                            </Button>
                        )}
                    </div>
                )}
            </div>
        </main>
    )
}