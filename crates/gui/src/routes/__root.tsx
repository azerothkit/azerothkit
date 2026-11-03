import { createRootRoute } from '@tanstack/react-router'
import { MainLayout } from '@/modules/main'
import { Bootstrap } from '@/bootstrap/Bootstrap'

export const Route = createRootRoute({
    component: () => (
        <>
            <Bootstrap>
                <MainLayout />
            </Bootstrap>
        </>
    ),
})