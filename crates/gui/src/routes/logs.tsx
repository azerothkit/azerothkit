import { createFileRoute } from '@tanstack/react-router'
import { LogsScreen } from '@/modules/logs'

export const Route = createFileRoute('/logs')({
    component: LogsScreen,
})