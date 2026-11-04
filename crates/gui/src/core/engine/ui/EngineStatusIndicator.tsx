import { StatusIndicator, type StatusIndicatorProps } from '@/shared/ui'
import { useEngine } from '../EngineProvider'
import type { EngineStatus } from '@/shared/api'

type EngineStatusIndicatorProps = Pick<StatusIndicatorProps, 'size' | 'className'>

const statusTones: Record<EngineStatus, StatusIndicatorProps['tone']> = {
    stopped: 'danger',
    starting: 'neutral',
    waiting_for_game: 'success',
    waiting_for_keys: 'success',
    running: 'success',
    stopping: 'neutral',
}

export function EngineStatusIndicator(props: EngineStatusIndicatorProps) {
    const { status, statusLabel, connection } = useEngine()

    const tone = connection === 'ready' && status !== null ? statusTones[status] : 'neutral'

    return (
        <StatusIndicator
            {...props}
            label={statusLabel}
            tone={tone}
        />
    )
}