import { createFileRoute } from '@tanstack/react-router'
import { MainScreen } from '@/modules/main'

export const Route = createFileRoute('/')({
    component: MainScreen,
})