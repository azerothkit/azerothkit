import { createRootRoute } from '@tanstack/react-router'
import { MainLayout } from '@/modules/main'
import { Bootstrap } from '@/bootstrap/Bootstrap'
import { WindowFrame } from '@/bootstrap/WindowFrame'

export const Route = createRootRoute({
    component: () => (
        <WindowFrame>
            <Bootstrap>
                <MainLayout />
            </Bootstrap>
        </WindowFrame>
    ),
})