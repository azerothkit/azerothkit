import { cn } from 'cn'

export type StatusIndicatorProps = {
    label: string
    tone: 'success' | 'danger' | 'neutral'
    muted?: boolean
    size?: 'sm' | 'md'
    className?: string
}

export function StatusIndicator({
    label,
    tone,
    muted = false,
    size = 'sm',
    className,
}: StatusIndicatorProps) {
    return (
        <div
            role="status"
            className={cn(
                'flex items-center gap-2',
                size === 'sm'
                    ? 'text-xs text-muted-foreground'
                    : 'justify-center text-lg font-semibold tracking-tight',
                className,
            )}
        >
            <span
                aria-hidden="true"
                className={cn(
                    'shrink-0 rounded-full transition-colors',
                    size === 'sm' ? 'size-1.5' : 'size-2',
                    tone === 'success' && (muted ? 'bg-green-500/40' : 'bg-green-500'),
                    tone === 'danger' && (muted ? 'bg-red-500/40' : 'bg-red-500'),
                    tone === 'neutral' && 'bg-muted-foreground/40',
                )}
            />

            <span>{label}</span>
        </div>
    )
}